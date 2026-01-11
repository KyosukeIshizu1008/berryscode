//! Operational Transformation engine
//!
//! Implements a basic OT algorithm for conflict-free collaborative text editing
//! Based on the Jupiter approach with client-server architecture

use serde::{Deserialize, Serialize};

/// Operation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OperationType {
    /// Insert text at position
    Insert {
        position: usize,
        text: String,
    },
    /// Delete text from position
    Delete {
        position: usize,
        length: usize,
    },
    /// Retain (no-op for transformation)
    Retain {
        count: usize,
    },
}

/// Operation with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub user_id: String,
    pub session_id: String,
    pub file_path: String,
    pub op_type: OperationType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: u64,
}

impl Operation {
    /// Create a new operation
    pub fn new(
        user_id: String,
        session_id: String,
        file_path: String,
        op_type: OperationType,
        version: u64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            session_id,
            file_path,
            op_type,
            timestamp: chrono::Utc::now(),
            version,
        }
    }

    /// Apply this operation to a text string
    pub fn apply(&self, text: &str) -> Result<String, String> {
        match &self.op_type {
            OperationType::Insert { position, text: insert_text } => {
                if *position > text.len() {
                    return Err(format!("Insert position {} out of bounds", position));
                }
                let mut result = String::with_capacity(text.len() + insert_text.len());
                result.push_str(&text[..*position]);
                result.push_str(insert_text);
                result.push_str(&text[*position..]);
                Ok(result)
            }
            OperationType::Delete { position, length } => {
                if *position > text.len() {
                    return Err(format!("Delete position {} out of bounds", position));
                }
                let end_pos = (*position + *length).min(text.len());
                let mut result = String::with_capacity(text.len());
                result.push_str(&text[..*position]);
                result.push_str(&text[end_pos..]);
                Ok(result)
            }
            OperationType::Retain { .. } => Ok(text.to_string()),
        }
    }

    /// Transform this operation against another operation (OT transformation)
    pub fn transform(&self, other: &Operation) -> Result<Operation, String> {
        let new_op_type = match (&self.op_type, &other.op_type) {
            // Insert vs Insert
            (
                OperationType::Insert { position: pos1, text: text1 },
                OperationType::Insert { position: pos2, text: text2 },
            ) => {
                if pos1 < pos2 || (pos1 == pos2 && self.user_id < other.user_id) {
                    // This insert comes before, no change needed
                    OperationType::Insert {
                        position: *pos1,
                        text: text1.clone(),
                    }
                } else {
                    // This insert comes after, adjust position
                    OperationType::Insert {
                        position: pos1 + text2.len(),
                        text: text1.clone(),
                    }
                }
            }

            // Insert vs Delete
            (
                OperationType::Insert { position, text },
                OperationType::Delete { position: del_pos, length },
            ) => {
                let del_end = *del_pos + *length;
                if *position <= *del_pos {
                    // Insert before delete, no change
                    OperationType::Insert {
                        position: *position,
                        text: text.clone(),
                    }
                } else if *position > del_end {
                    // Insert after delete, shift back
                    OperationType::Insert {
                        position: position - length,
                        text: text.clone(),
                    }
                } else {
                    // Insert within delete range, place at delete start
                    OperationType::Insert {
                        position: *del_pos,
                        text: text.clone(),
                    }
                }
            }

            // Delete vs Insert
            (
                OperationType::Delete { position, length },
                OperationType::Insert { position: ins_pos, text },
            ) => {
                let del_end = *position + *length;
                if *ins_pos <= *position {
                    // Insert before delete, shift forward
                    OperationType::Delete {
                        position: position + text.len(),
                        length: *length,
                    }
                } else if *ins_pos >= del_end {
                    // Insert after delete, no change
                    OperationType::Delete {
                        position: *position,
                        length: *length,
                    }
                } else {
                    // Insert within delete range, expand delete
                    OperationType::Delete {
                        position: *position,
                        length: length + text.len(),
                    }
                }
            }

            // Delete vs Delete
            (
                OperationType::Delete { position: pos1, length: len1 },
                OperationType::Delete { position: pos2, length: len2 },
            ) => {
                let end1 = *pos1 + *len1;
                let end2 = *pos2 + *len2;

                if end1 <= *pos2 {
                    // This delete is before other, no change
                    OperationType::Delete {
                        position: *pos1,
                        length: *len1,
                    }
                } else if *pos1 >= end2 {
                    // This delete is after other, shift back
                    OperationType::Delete {
                        position: pos1 - len2,
                        length: *len1,
                    }
                } else {
                    // Overlapping deletes - complex case
                    let new_pos = (*pos1).min(*pos2);
                    let overlap_start = (*pos1).max(*pos2);
                    let overlap_end = end1.min(end2);
                    let overlap_len = if overlap_end > overlap_start {
                        overlap_end - overlap_start
                    } else {
                        0
                    };

                    let new_length = if len1 > &overlap_len {
                        len1 - overlap_len
                    } else {
                        0
                    };

                    if new_length == 0 {
                        // Delete is completely contained in other delete
                        OperationType::Retain { count: 0 }
                    } else {
                        OperationType::Delete {
                            position: new_pos,
                            length: new_length,
                        }
                    }
                }
            }

            // Retain cases (no-op)
            (OperationType::Retain { .. }, _) | (_, OperationType::Retain { .. }) => {
                return Ok(self.clone());
            }
        };

        Ok(Operation {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: self.user_id.clone(),
            session_id: self.session_id.clone(),
            file_path: self.file_path.clone(),
            op_type: new_op_type,
            timestamp: chrono::Utc::now(),
            version: self.version,
        })
    }
}

/// OT Engine manages operation history and transformation
pub struct OTEngine {
    /// Operation history per file
    history: std::collections::HashMap<String, Vec<Operation>>,
    /// Current version per file
    versions: std::collections::HashMap<String, u64>,
}

impl OTEngine {
    /// Create a new OT engine
    pub fn new() -> Self {
        Self {
            history: std::collections::HashMap::new(),
            versions: std::collections::HashMap::new(),
        }
    }

    /// Apply an operation and store in history
    pub fn apply_operation(&mut self, mut operation: Operation, text: &str) -> Result<String, String> {
        let file_path = operation.file_path.clone();

        // Get current version
        let current_version = *self.versions.get(&file_path).unwrap_or(&0);

        // Transform operation against all operations since the operation's version
        let mut transformed_op = operation.clone();
        if let Some(history) = self.history.get(&file_path) {
            for historical_op in history.iter().filter(|op| op.version >= operation.version) {
                transformed_op = transformed_op.transform(historical_op)?;
            }
        }

        // Apply the transformed operation
        let result = transformed_op.apply(text)?;

        // Update version
        operation.version = current_version + 1;
        self.versions.insert(file_path.clone(), current_version + 1);

        // Add to history
        self.history
            .entry(file_path)
            .or_insert_with(Vec::new)
            .push(operation);

        Ok(result)
    }

    /// Get current version for a file
    pub fn get_version(&self, file_path: &str) -> u64 {
        *self.versions.get(file_path).unwrap_or(&0)
    }

    /// Get operation history for a file
    pub fn get_history(&self, file_path: &str) -> Vec<Operation> {
        self.history
            .get(file_path)
            .cloned()
            .unwrap_or_default()
    }

    /// Clear history for a file (when file is closed)
    pub fn clear_file_history(&mut self, file_path: &str) {
        self.history.remove(file_path);
        self.versions.remove(file_path);
    }

    /// Get history size (for debugging)
    pub fn history_size(&self, file_path: &str) -> usize {
        self.history.get(file_path).map(|h| h.len()).unwrap_or(0)
    }
}

impl Default for OTEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_operation() {
        let op = Operation::new(
            "user1".to_string(),
            "session1".to_string(),
            "test.txt".to_string(),
            OperationType::Insert {
                position: 5,
                text: "world".to_string(),
            },
            0,
        );

        let text = "hello";
        let result = op.apply(text).unwrap();
        assert_eq!(result, "helloworld");
    }

    #[test]
    fn test_delete_operation() {
        let op = Operation::new(
            "user1".to_string(),
            "session1".to_string(),
            "test.txt".to_string(),
            OperationType::Delete {
                position: 5,
                length: 6,
            },
            0,
        );

        let text = "hello world";
        let result = op.apply(text).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_transform_insert_insert() {
        let op1 = Operation::new(
            "user1".to_string(),
            "session1".to_string(),
            "test.txt".to_string(),
            OperationType::Insert {
                position: 5,
                text: "A".to_string(),
            },
            0,
        );

        let op2 = Operation::new(
            "user2".to_string(),
            "session1".to_string(),
            "test.txt".to_string(),
            OperationType::Insert {
                position: 5,
                text: "B".to_string(),
            },
            0,
        );

        let transformed = op1.transform(&op2).unwrap();

        // Since user1 < user2, op1's position should remain 5
        match transformed.op_type {
            OperationType::Insert { position, .. } => {
                assert_eq!(position, 5);
            }
            _ => panic!("Expected Insert operation"),
        }
    }

    #[test]
    fn test_ot_engine() {
        let mut engine = OTEngine::new();

        let op1 = Operation::new(
            "user1".to_string(),
            "session1".to_string(),
            "test.txt".to_string(),
            OperationType::Insert {
                position: 0,
                text: "hello".to_string(),
            },
            0,
        );

        let result1 = engine.apply_operation(op1, "").unwrap();
        assert_eq!(result1, "hello");
        assert_eq!(engine.get_version("test.txt"), 1);

        let op2 = Operation::new(
            "user2".to_string(),
            "session1".to_string(),
            "test.txt".to_string(),
            OperationType::Insert {
                position: 5,
                text: " world".to_string(),
            },
            1,
        );

        let result2 = engine.apply_operation(op2, &result1).unwrap();
        assert_eq!(result2, "hello world");
        assert_eq!(engine.get_version("test.txt"), 2);
    }
}
