//! Code Chunking - Smart code splitting for embeddings
//!
//! This module provides intelligent code chunking strategies optimized for
//! local embedding models with token limits (512 tokens ≈ 400-500 chars).
//!
//! ## Why Chunking is Critical
//!
//! - **OpenAI API**: 8191 tokens (whole files fit)
//! - **Local Model (all-MiniLM-L6-v2)**: 512 tokens (requires chunking)
//!
//! Without proper chunking, content beyond 512 tokens is **silently truncated**,
//! destroying search quality.
//!
//! ## Chunking Strategies
//!
//! ### 1. AST-Based Chunking (Recommended) ✅
//!
//! Uses tree-sitter to split by semantic units:
//! - Functions
//! - Structs/Classes
//! - Impl blocks
//! - Test functions
//!
//! Benefits:
//! - Preserves code meaning
//! - Clean boundaries
//! - Each chunk is self-contained
//!
//! ### 2. Overlapping Fixed-Size Chunking (Fallback) ⚠️
//!
//! Splits by character count with overlap:
//! - Chunk size: 400 chars (safe margin for 512 tokens)
//! - Overlap: 50 chars (prevents cutting mid-sentence)
//!
//! Benefits:
//! - Works for any file type
//! - Simple and fast
//! - No parsing errors
//!
//! ## Example
//!
//! ```rust
//! let chunker = CodeChunker::new();
//! let chunks = chunker.chunk_file("src/main.rs", "fn main() { ... }", "rs");
//!
//! // Result: Multiple chunks, each ≤ 400 chars
//! for chunk in chunks {
//!     println!("{}: {}", chunk.chunk_type, chunk.content);
//! }
//! ```

use crate::berrycode::Result;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

/// A code chunk with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    /// File path (relative to project root)
    pub file_path: String,
    /// Chunk index (0-based)
    pub chunk_index: usize,
    /// Chunk type: "function", "struct", "impl", "test", "overlap_chunk", etc.
    pub chunk_type: String,
    /// Chunk name (e.g., function name, struct name)
    pub name: Option<String>,
    /// Chunk content (actual code)
    pub content: String,
    /// Line number where this chunk starts
    pub start_line: usize,
    /// Line number where this chunk ends
    pub end_line: usize,
}

/// Code chunker with multiple strategies
pub struct CodeChunker {
    /// Max chunk size in characters (conservative: 400 chars ≈ 512 tokens)
    max_chunk_size: usize,
    /// Overlap between chunks (prevents cutting mid-sentence)
    overlap_size: usize,
}

impl Default for CodeChunker {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeChunker {
    /// Create a new code chunker with sensible defaults
    pub fn new() -> Self {
        Self {
            max_chunk_size: 400, // Conservative: leaves margin for 512 token limit
            overlap_size: 50,    // 50 char overlap prevents cutting mid-sentence
        }
    }

    /// Create with custom settings
    pub fn with_config(max_chunk_size: usize, overlap_size: usize) -> Self {
        Self {
            max_chunk_size,
            overlap_size,
        }
    }

    /// Chunk a file using the best strategy for its language
    ///
    /// # Arguments
    /// * `file_path` - Relative path to the file
    /// * `content` - Full file content
    /// * `extension` - File extension (e.g., "rs", "py")
    ///
    /// # Returns
    /// * Vector of code chunks
    pub fn chunk_file(&self, file_path: &str, content: &str, extension: &str) -> Vec<CodeChunk> {
        // Try AST-based chunking first (for supported languages)
        if let Ok(chunks) = self.chunk_ast(file_path, content, extension) {
            if !chunks.is_empty() {
                tracing::debug!("✓ AST chunking: {} → {} chunks", file_path, chunks.len());
                return chunks;
            }
        }

        // Fallback: Overlap-based chunking
        let chunks = self.chunk_overlap(file_path, content);
        tracing::debug!(
            "⚠ Overlap chunking: {} → {} chunks",
            file_path,
            chunks.len()
        );
        chunks
    }

    /// AST-based chunking using tree-sitter
    ///
    /// Splits code by semantic units (functions, structs, impl blocks)
    fn chunk_ast(&self, file_path: &str, content: &str, extension: &str) -> Result<Vec<CodeChunk>> {
        #[cfg(feature = "tree-sitter")]
        {
            use tree_sitter::{Parser, Language};

            // Get language parser
            let language = match extension {
                "rs" => tree_sitter_rust::LANGUAGE,
                "py" => tree_sitter_python::LANGUAGE,
                "js" | "jsx" => tree_sitter_javascript::LANGUAGE,
                "ts" | "tsx" => tree_sitter_javascript::LANGUAGE, // TypeScript uses JS parser
                "c" | "h" => tree_sitter_c::LANGUAGE,
                "cpp" | "hpp" | "cc" => tree_sitter_cpp::LANGUAGE,
                _ => return Err(anyhow!("Unsupported language: {}", extension)),
            };

            let mut parser = Parser::new();
            parser
                .set_language(&language.into())
                .map_err(|e| anyhow!("Failed to set language: {}", e))?;

            let tree = parser
                .parse(content, None)
                .ok_or_else(|| anyhow!("Failed to parse file"))?;

            let root_node = tree.root_node();
            let mut chunks = Vec::new();
            let mut chunk_index = 0;

            // Walk the syntax tree
            let mut cursor = root_node.walk();
            for child in root_node.children(&mut cursor) {
                let node_type = child.kind();

                // Check if this is a "chunkable" node
                let chunk_type = match node_type {
                    "function_item" | "function_declaration" | "function_definition" => "function",
                    "struct_item" | "struct_declaration" => "struct",
                    "impl_item" => "impl",
                    "class_declaration" | "class_definition" => "class",
                    "method_definition" => "method",
                    _ => continue, // Skip non-chunkable nodes
                };

                // Extract chunk content
                let start_byte = child.start_byte();
                let end_byte = child.end_byte();
                let chunk_content = &content[start_byte..end_byte];

                // Skip empty or tiny chunks
                if chunk_content.trim().is_empty() || chunk_content.len() < 10 {
                    continue;
                }

                // Extract name (if possible)
                let name = self.extract_name(&child, content);

                // Calculate line numbers
                let start_line = content[..start_byte].lines().count();
                let end_line = content[..end_byte].lines().count();

                // If chunk is too large, split it further
                if chunk_content.len() > self.max_chunk_size {
                    // Fallback to overlap chunking for this large node
                    let sub_chunks = self.chunk_overlap_raw(chunk_content, chunk_index);
                    for mut sub_chunk in sub_chunks {
                        sub_chunk.file_path = file_path.to_string();
                        sub_chunk.chunk_type = format!("{}_split", chunk_type);
                        sub_chunk.name = name.clone();
                        sub_chunk.start_line += start_line;
                        sub_chunk.end_line += start_line;
                        chunks.push(sub_chunk);
                        chunk_index += 1;
                    }
                } else {
                    chunks.push(CodeChunk {
                        file_path: file_path.to_string(),
                        chunk_index,
                        chunk_type: chunk_type.to_string(),
                        name,
                        content: chunk_content.to_string(),
                        start_line,
                        end_line,
                    });
                    chunk_index += 1;
                }
            }

            Ok(chunks)
        }

        #[cfg(not(feature = "tree-sitter"))]
        {
            Err(anyhow!("tree-sitter feature not enabled"))
        }
    }

    /// Extract name from a tree-sitter node (function name, struct name, etc.)
    #[cfg(feature = "tree-sitter")]
    fn extract_name(&self, node: &tree_sitter::Node, content: &str) -> Option<String> {
        // Look for "name" or "identifier" child
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "name" {
                let start = child.start_byte();
                let end = child.end_byte();
                return Some(content[start..end].to_string());
            }
        }
        None
    }

    /// Overlap-based chunking (fallback for unsupported languages)
    ///
    /// Splits text into fixed-size chunks with overlap
    fn chunk_overlap(&self, file_path: &str, content: &str) -> Vec<CodeChunk> {
        let chunks = self.chunk_overlap_raw(content, 0);
        chunks
            .into_iter()
            .map(|mut chunk| {
                chunk.file_path = file_path.to_string();
                chunk
            })
            .collect()
    }

    /// Raw overlap chunking (returns chunks without file_path set)
    fn chunk_overlap_raw(&self, content: &str, start_index: usize) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut current_pos = 0;
        let mut chunk_index = start_index;

        while current_pos < content.len() {
            // Grab next chunk
            let chunk_end = (current_pos + self.max_chunk_size).min(content.len());
            let chunk_content = &content[current_pos..chunk_end];

            // Calculate line numbers
            let start_line = content[..current_pos].lines().count();
            let end_line = content[..chunk_end].lines().count();

            chunks.push(CodeChunk {
                file_path: String::new(), // Will be set by caller
                chunk_index,
                chunk_type: "overlap_chunk".to_string(),
                name: None,
                content: chunk_content.to_string(),
                start_line,
                end_line,
            });

            // Move forward with overlap
            current_pos += self.max_chunk_size;
            if current_pos < content.len() {
                current_pos -= self.overlap_size; // Back up for overlap
            }

            chunk_index += 1;
        }

        chunks
    }

    /// Get recommended max chunk size
    pub fn max_chunk_size(&self) -> usize {
        self.max_chunk_size
    }

    /// Get overlap size
    pub fn overlap_size(&self) -> usize {
        self.overlap_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunker_creation() {
        let chunker = CodeChunker::new();
        assert_eq!(chunker.max_chunk_size(), 400);
        assert_eq!(chunker.overlap_size(), 50);
    }

    #[test]
    fn test_chunker_custom_config() {
        let chunker = CodeChunker::with_config(500, 100);
        assert_eq!(chunker.max_chunk_size(), 500);
        assert_eq!(chunker.overlap_size(), 100);
    }

    #[test]
    fn test_overlap_chunking_small_file() {
        let chunker = CodeChunker::new();
        let content = "fn main() {\n    println!(\"Hello\");\n}";
        let chunks = chunker.chunk_overlap("test.rs", content);

        assert_eq!(chunks.len(), 1, "Small file should be 1 chunk");
        assert_eq!(chunks[0].content, content);
        assert_eq!(chunks[0].chunk_type, "overlap_chunk");
    }

    #[test]
    fn test_overlap_chunking_large_file() {
        let chunker = CodeChunker::new();
        // Create content larger than 400 chars
        let mut content = String::new();
        for i in 0..30 {
            content.push_str(&format!("fn function_{}() {{ println!(\"Function {}\"); }}\n", i, i));
        }

        let chunks = chunker.chunk_overlap("test.rs", &content);
        assert!(chunks.len() > 1, "Large file should have multiple chunks");

        // Check overlap
        if chunks.len() >= 2 {
            let first_end = &chunks[0].content[chunks[0].content.len().saturating_sub(50)..];
            let second_start = &chunks[1].content[..50.min(chunks[1].content.len())];

            // Should have some overlap
            assert!(
                first_end.chars().rev().take(10).any(|c| second_start.contains(c)),
                "Chunks should have overlap"
            );
        }
    }

    #[test]
    #[cfg(feature = "tree-sitter")]
    fn test_ast_chunking_rust() {
        let chunker = CodeChunker::new();
        let content = r#"
struct User {
    name: String,
}

impl User {
    fn new(name: String) -> Self {
        User { name }
    }
}

fn main() {
    let user = User::new("Alice".to_string());
}
"#;

        let chunks = chunker.chunk_file("test.rs", content, "rs");

        assert!(chunks.len() >= 3, "Should have struct + impl + function");

        // Check chunk types
        let types: Vec<&str> = chunks.iter().map(|c| c.chunk_type.as_str()).collect();
        assert!(types.contains(&"struct"), "Should have struct chunk");
        assert!(types.contains(&"impl"), "Should have impl chunk");
        assert!(types.contains(&"function"), "Should have function chunk");
    }

    #[test]
    fn test_chunk_index() {
        let chunker = CodeChunker::new();
        let content = "a".repeat(1000); // Large content to force multiple chunks
        let chunks = chunker.chunk_overlap("test.txt", &content);

        // Check indices are sequential
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, i, "Chunk indices should be sequential");
        }
    }

    #[test]
    fn test_line_numbers() {
        let chunker = CodeChunker::new();
        let content = "line1\nline2\nline3\nline4\nline5";
        let chunks = chunker.chunk_overlap("test.txt", content);

        for chunk in chunks {
            assert!(
                chunk.end_line >= chunk.start_line,
                "End line should be >= start line"
            );
        }
    }
}
