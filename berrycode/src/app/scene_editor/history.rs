#![allow(dead_code)]
//! Undo/Redo via SceneModel snapshots with a command-pattern overlay.
//!
//! ## Hybrid Architecture
//!
//! The original snapshot-based `EditHistory` is retained as the backend: each
//! undo/redo still restores a full `SceneModel` clone. On top of that we add
//! `SceneCommand` and `CommandHistory` which *record what* was done alongside
//! each snapshot. This gives us:
//!
//! - **Backward-compatible undo/redo** (snapshot restore, trivially correct)
//! - **Structured audit trail** (the command log describes every operation)
//! - **Future delta path** (a later phase can apply commands in reverse instead
//!   of restoring full snapshots, without changing any call site)

use super::model::{SceneModel, TransformData};

// ---------------------------------------------------------------------------
// SceneCommand — describes a single editor operation
// ---------------------------------------------------------------------------

/// A high-level description of a scene edit operation.
///
/// Each variant carries enough information to display a human-readable
/// description and, in the future, to invert the operation for delta-based
/// undo.
#[derive(Debug, Clone)]
pub enum SceneCommand {
    /// Transform changed on an entity.
    SetTransform {
        entity_id: u64,
        old: TransformData,
        new: TransformData,
    },
    /// A new entity was added to the scene.
    AddEntity { entity_id: u64, name: String },
    /// An entity was removed from the scene.
    RemoveEntity { entity_id: u64 },
    /// An entity was moved to a new parent.
    ReparentEntity {
        entity_id: u64,
        old_parent: Option<u64>,
        new_parent: Option<u64>,
    },
    /// An entity was renamed.
    RenameEntity {
        entity_id: u64,
        old_name: String,
        new_name: String,
    },
    /// A component on an entity was modified (free-form description).
    ModifyComponent { entity_id: u64, description: String },
    /// An entity was duplicated.
    DuplicateEntity { source_id: u64, new_id: u64 },
    /// A batch of commands executed atomically.
    Batch(Vec<SceneCommand>),
    /// Fallback for operations not yet given a specialised variant.
    Generic(String),
}

impl SceneCommand {
    /// Human-readable one-line description of this command.
    pub fn description(&self) -> String {
        match self {
            SceneCommand::SetTransform { entity_id, .. } => {
                format!("Transform entity {}", entity_id)
            }
            SceneCommand::AddEntity { name, .. } => format!("Add entity '{}'", name),
            SceneCommand::RemoveEntity { entity_id } => {
                format!("Remove entity {}", entity_id)
            }
            SceneCommand::ReparentEntity { entity_id, .. } => {
                format!("Reparent entity {}", entity_id)
            }
            SceneCommand::RenameEntity {
                old_name, new_name, ..
            } => format!("Rename '{}' -> '{}'", old_name, new_name),
            SceneCommand::ModifyComponent { description, .. } => description.clone(),
            SceneCommand::DuplicateEntity { source_id, .. } => {
                format!("Duplicate entity {}", source_id)
            }
            SceneCommand::Batch(cmds) => format!("{} operations", cmds.len()),
            SceneCommand::Generic(desc) => desc.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// EditHistory — unchanged snapshot backend
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct EditHistory {
    undo_stack: Vec<SceneModel>,
    redo_stack: Vec<SceneModel>,
    max_history: usize,
}

impl EditHistory {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 100,
        }
    }

    /// Save a snapshot of the current state. Call this BEFORE making changes.
    /// Any new edit invalidates the redo stack.
    pub fn snapshot(&mut self, current: &SceneModel) {
        self.undo_stack.push(current.clone());
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Returns the previous state if available; pushes the current onto redo.
    pub fn undo(&mut self, current: &SceneModel) -> Option<SceneModel> {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(current.clone());
            Some(prev)
        } else {
            None
        }
    }

    /// Returns the next state if available; pushes the current onto undo.
    pub fn redo(&mut self, current: &SceneModel) -> Option<SceneModel> {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(current.clone());
            Some(next)
        } else {
            None
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

// ---------------------------------------------------------------------------
// CommandHistory — hybrid wrapper: commands + snapshot backend
// ---------------------------------------------------------------------------

/// Wraps [`EditHistory`] with a parallel command log so each snapshot is
/// annotated with a [`SceneCommand`] describing the operation.
///
/// Call sites use [`CommandHistory::execute`] instead of
/// [`EditHistory::snapshot`]. Under the hood it still takes a full snapshot,
/// but the command log enables future delta-based undo and provides a
/// human-readable edit trail.
pub struct CommandHistory {
    pub edit_history: EditHistory,
    /// Parallel log of commands; `command_log[i]` corresponds to the snapshot
    /// at `edit_history.undo_stack[i]`.
    command_log: Vec<SceneCommand>,
    /// Points one past the last executed command.  Matches
    /// `edit_history.undo_stack.len()` after each `execute` / `undo` / `redo`.
    undo_index: usize,
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            edit_history: EditHistory::new(),
            command_log: Vec::new(),
            undo_index: 0,
        }
    }

    /// Record a command and take a snapshot of the current scene.
    ///
    /// Call this BEFORE applying the mutation to `scene`.
    pub fn execute(&mut self, command: SceneCommand, scene: &SceneModel) {
        self.edit_history.snapshot(scene);
        // Truncate any commands that were ahead of us (redo path invalidated).
        self.command_log.truncate(self.undo_index);
        self.command_log.push(command);
        self.undo_index = self.command_log.len();
    }

    /// Undo the last command, returning the previous scene state.
    pub fn undo(&mut self, scene: &SceneModel) -> Option<SceneModel> {
        let result = self.edit_history.undo(scene);
        if result.is_some() && self.undo_index > 0 {
            self.undo_index -= 1;
        }
        result
    }

    /// Redo the next command, returning the next scene state.
    pub fn redo(&mut self, scene: &SceneModel) -> Option<SceneModel> {
        let result = self.edit_history.redo(scene);
        if result.is_some() && self.undo_index < self.command_log.len() {
            self.undo_index += 1;
        }
        result
    }

    pub fn can_undo(&self) -> bool {
        self.edit_history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.edit_history.can_redo()
    }

    /// The most recently executed (or undone-to) command, if any.
    pub fn last_command(&self) -> Option<&SceneCommand> {
        if self.undo_index > 0 {
            self.command_log.get(self.undo_index - 1)
        } else {
            None
        }
    }

    /// Description of the command that would be undone next.
    pub fn undo_description(&self) -> Option<String> {
        self.last_command().map(|c| c.description())
    }

    /// Description of the command that would be redone next.
    pub fn redo_description(&self) -> Option<String> {
        self.command_log
            .get(self.undo_index)
            .map(|c| c.description())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::scene_editor::model::ComponentData;

    // --- Original EditHistory tests (preserved) ---

    #[test]
    fn snapshot_then_undo_restores_state() {
        let mut model = SceneModel::new();
        let mut history = EditHistory::new();

        history.snapshot(&model);
        let id = model.add_entity(
            "Cube".into(),
            vec![ComponentData::MeshCube {
                size: 1.0,
                color: [1.0, 1.0, 1.0],
                metallic: 0.0,
                roughness: 0.5,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        assert!(model.entities.contains_key(&id));

        let restored = history.undo(&model).expect("undo should yield prev state");
        assert!(!restored.entities.contains_key(&id));
        assert!(history.can_redo());
    }

    #[test]
    fn redo_after_undo_returns_state() {
        let mut model = SceneModel::new();
        let mut history = EditHistory::new();

        history.snapshot(&model);
        model.add_entity("Cube".into(), vec![]);
        let after_add = model.clone();

        let prev = history.undo(&model).unwrap();
        let next = history.redo(&prev).unwrap();
        assert_eq!(next.entities.len(), after_add.entities.len());
    }

    #[test]
    fn snapshot_clears_redo_stack() {
        let mut model = SceneModel::new();
        let mut history = EditHistory::new();

        history.snapshot(&model);
        model.add_entity("A".into(), vec![]);
        let _ = history.undo(&model);
        assert!(history.can_redo());

        history.snapshot(&model);
        assert!(!history.can_redo());
    }

    #[test]
    fn max_history_caps_undo_stack() {
        let mut model = SceneModel::new();
        let mut history = EditHistory::new();
        history.max_history = 3;

        for _ in 0..5 {
            history.snapshot(&model);
        }
        assert!(history.undo_stack.len() <= 3);
    }

    // --- CommandHistory tests ---

    #[test]
    fn command_history_execute_and_undo() {
        let mut model = SceneModel::new();
        let mut cmd_history = CommandHistory::new();

        cmd_history.execute(
            SceneCommand::AddEntity {
                entity_id: 0,
                name: "Cube".into(),
            },
            &model,
        );
        let id = model.add_entity("Cube".into(), vec![]);
        assert!(model.entities.contains_key(&id));

        let restored = cmd_history.undo(&model).expect("undo should work");
        assert!(!restored.entities.contains_key(&id));
        assert!(cmd_history.can_redo());
    }

    #[test]
    fn command_history_redo_after_undo() {
        let mut model = SceneModel::new();
        let mut cmd_history = CommandHistory::new();

        cmd_history.execute(
            SceneCommand::AddEntity {
                entity_id: 0,
                name: "Cube".into(),
            },
            &model,
        );
        model.add_entity("Cube".into(), vec![]);
        let after_add = model.clone();

        let prev = cmd_history.undo(&model).unwrap();
        let next = cmd_history.redo(&prev).unwrap();
        assert_eq!(next.entities.len(), after_add.entities.len());
    }

    #[test]
    fn command_history_execute_clears_redo() {
        let mut model = SceneModel::new();
        let mut cmd_history = CommandHistory::new();

        cmd_history.execute(SceneCommand::Generic("first".into()), &model);
        model.add_entity("A".into(), vec![]);

        let prev = cmd_history.undo(&model).unwrap();
        assert!(cmd_history.can_redo());

        // A new execute should clear redo
        cmd_history.execute(SceneCommand::Generic("second".into()), &prev);
        assert!(!cmd_history.can_redo());
    }

    #[test]
    fn command_history_last_command_tracking() {
        let model = SceneModel::new();
        let mut cmd_history = CommandHistory::new();

        assert!(cmd_history.last_command().is_none());

        cmd_history.execute(
            SceneCommand::AddEntity {
                entity_id: 1,
                name: "Cube".into(),
            },
            &model,
        );
        let desc = cmd_history.last_command().unwrap().description();
        assert!(desc.contains("Cube"), "Expected 'Cube' in '{}'", desc);

        cmd_history.execute(SceneCommand::RemoveEntity { entity_id: 1 }, &model);
        let desc = cmd_history.last_command().unwrap().description();
        assert!(desc.contains("Remove"), "Expected 'Remove' in '{}'", desc);
    }

    #[test]
    fn command_description_formatting() {
        let cases = vec![
            (
                SceneCommand::SetTransform {
                    entity_id: 42,
                    old: TransformData::default(),
                    new: TransformData::default(),
                },
                "Transform entity 42",
            ),
            (
                SceneCommand::AddEntity {
                    entity_id: 1,
                    name: "Player".into(),
                },
                "Add entity 'Player'",
            ),
            (
                SceneCommand::RemoveEntity { entity_id: 7 },
                "Remove entity 7",
            ),
            (
                SceneCommand::ReparentEntity {
                    entity_id: 3,
                    old_parent: None,
                    new_parent: Some(1),
                },
                "Reparent entity 3",
            ),
            (
                SceneCommand::RenameEntity {
                    entity_id: 5,
                    old_name: "Old".into(),
                    new_name: "New".into(),
                },
                "Rename 'Old' -> 'New'",
            ),
            (
                SceneCommand::ModifyComponent {
                    entity_id: 1,
                    description: "Change color".into(),
                },
                "Change color",
            ),
            (
                SceneCommand::DuplicateEntity {
                    source_id: 10,
                    new_id: 11,
                },
                "Duplicate entity 10",
            ),
            (
                SceneCommand::Batch(vec![
                    SceneCommand::Generic("a".into()),
                    SceneCommand::Generic("b".into()),
                ]),
                "2 operations",
            ),
            (SceneCommand::Generic("custom op".into()), "custom op"),
        ];

        for (cmd, expected) in cases {
            assert_eq!(cmd.description(), expected);
        }
    }

    #[test]
    fn command_history_undo_redo_descriptions() {
        let model = SceneModel::new();
        let mut cmd_history = CommandHistory::new();

        assert!(cmd_history.undo_description().is_none());
        assert!(cmd_history.redo_description().is_none());

        cmd_history.execute(
            SceneCommand::AddEntity {
                entity_id: 1,
                name: "A".into(),
            },
            &model,
        );
        cmd_history.execute(
            SceneCommand::AddEntity {
                entity_id: 2,
                name: "B".into(),
            },
            &model,
        );

        // Undo description should be the last executed command
        assert_eq!(cmd_history.undo_description().unwrap(), "Add entity 'B'");

        cmd_history.undo(&model);

        // After undo, redo description should be the undone command
        assert_eq!(cmd_history.redo_description().unwrap(), "Add entity 'B'");
        assert_eq!(cmd_history.undo_description().unwrap(), "Add entity 'A'");
    }
}
