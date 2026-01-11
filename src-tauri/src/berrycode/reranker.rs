//! Reranker - Cross-Encoder based precision reranking
//!
//! This module implements **Cross-Encoder Reranking** - improving search precision
//! by reordering results using a more accurate (but slower) model.
//!
//! ## Why Reranking is Critical
//!
//! - **Bi-Encoder (Vector Search)**: Fast but less accurate
//!   - Encodes query and documents separately
//!   - Compares via cosine similarity
//!   - Can miss subtle relevance signals
//!
//! - **Cross-Encoder (Reranker)**: Slow but highly accurate
//!   - Encodes query + document together
//!   - Captures interaction signals
//!   - 5-10% improvement in relevance
//!
//! ## Strategy: Two-Stage Retrieval
//!
//! 1. **Stage 1: Vector Search** (fast, recall-focused)
//!    - Retrieve top 50 candidates quickly
//!    - Use semantic similarity
//!
//! 2. **Stage 2: Reranking** (slow, precision-focused)
//!    - Rerank top 20 with cross-encoder
//!    - Return top 5 final results
//!
//! ## Performance
//!
//! - **Vector Search**: 10-50ms for 1000s of documents
//! - **Reranking**: 50-200ms for 20 documents
//! - **Total**: Still <250ms (acceptable for AI pair programming)
//!
//! ## Example
//!
//! ```text
//! User: "Find user authentication logic"
//!
//! Step 1: Vector Search (50 results)
//!   1. src/auth/user.rs (0.85)
//!   2. src/api/users.rs (0.82)
//!   3. src/middleware/auth.rs (0.80)
//!   ...
//!   50. src/utils/crypto.rs (0.45)
//!
//! Step 2: Reranking (top 20 → top 5)
//!   1. src/auth/login.rs (0.95)     ← Moved up!
//!   2. src/auth/user.rs (0.92)
//!   3. src/middleware/auth.rs (0.88)
//!   4. src/api/auth_controller.rs (0.85)
//!   5. src/auth/session.rs (0.82)
//!
//! Result: More relevant results ranked higher!
//! ```

use crate::berrycode::Result;
use crate::berrycode::hybrid_search::HybridSearchResult;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Reranker using cross-encoder models
pub struct Reranker {
    #[cfg(feature = "fastembed")]
    model: Arc<Mutex<fastembed::TextRerank>>,
}

impl Reranker {
    /// Create a new reranker with default model (BAAI/bge-reranker-base)
    pub fn new() -> Result<Self> {
        Self::with_model(None)
    }

    /// Create reranker with specific model
    #[cfg(feature = "fastembed")]
    pub fn with_model(_model_name: Option<String>) -> Result<Self> {
        use fastembed::{TextRerank, RerankInitOptions};

        // Initialize with default model (BAAI/bge-reranker-base)
        // Note: FastEmbed v5 uses Default::default() for initialization
        let model = TextRerank::try_new(Default::default())
            .map_err(|e| anyhow!("Failed to initialize reranker model: {}", e))?;

        tracing::info!("🎯 Reranker initialized with BAAI/bge-reranker-base");

        Ok(Self {
            model: Arc::new(Mutex::new(model)),
        })
    }

    #[cfg(not(feature = "fastembed"))]
    pub fn with_model(_model_name: Option<String>) -> Result<Self> {
        Err(anyhow!("fastembed feature not enabled"))
    }

    /// Rerank search results using cross-encoder
    ///
    /// # Arguments
    /// * `query` - Search query
    /// * `results` - Initial search results (from vector search or hybrid search)
    /// * `top_k` - Number of results to rerank (default: 20)
    ///
    /// # Returns
    /// * Reranked results with updated scores
    pub fn rerank(
        &self,
        query: &str,
        results: Vec<HybridSearchResult>,
        top_k: Option<usize>,
    ) -> Result<Vec<HybridSearchResult>> {
        #[cfg(feature = "fastembed")]
        {
            if results.is_empty() {
                return Ok(results);
            }

            // Determine how many to rerank
            let rerank_count = top_k.unwrap_or(20).min(results.len());

            // Extract documents to rerank
            let documents: Vec<String> = results
                .iter()
                .take(rerank_count)
                .map(|r| {
                    // Create a rich context for the cross-encoder
                    format!(
                        "File: {}\nType: {}\nSummary: {}",
                        r.file_path,
                        r.chunk_type.as_ref().unwrap_or(&"unknown".to_string()),
                        r.summary
                    )
                })
                .collect();

            let doc_refs: Vec<&str> = documents.iter().map(|s| s.as_str()).collect();

            // Rerank using cross-encoder
            let mut model = self.model.lock().unwrap();
            let rerank_results = model
                .rerank(query, doc_refs, false, None)
                .map_err(|e| anyhow!("Failed to rerank: {}", e))?;

            drop(model); // Release lock

            // Update scores in results
            let mut reranked = Vec::new();
            for (i, rerank_result) in rerank_results.iter().enumerate() {
                if let Some(original) = results.get(rerank_result.index) {
                    let mut updated = original.clone();
                    // Cross-encoder score is more accurate - use it directly
                    updated.combined_score = rerank_result.score;
                    updated.similarity_score = rerank_result.score; // Update similarity too
                    reranked.push(updated);
                }
            }

            // Add remaining results (not reranked)
            for result in results.iter().skip(rerank_count) {
                reranked.push(result.clone());
            }

            tracing::debug!(
                "🎯 Reranked {} results (top_k={})",
                rerank_count,
                rerank_count
            );

            Ok(reranked)
        }

        #[cfg(not(feature = "fastembed"))]
        {
            tracing::warn!("fastembed not enabled, returning original results");
            Ok(results)
        }
    }

    /// Rerank with automatic top_k selection
    ///
    /// Automatically chooses top_k based on result count:
    /// - < 10 results: rerank all
    /// - 10-50 results: rerank top 20
    /// - > 50 results: rerank top 30
    pub fn rerank_auto(&self, query: &str, results: Vec<HybridSearchResult>) -> Result<Vec<HybridSearchResult>> {
        let top_k = match results.len() {
            0..=10 => results.len(),
            11..=50 => 20,
            _ => 30,
        };

        self.rerank(query, results, Some(top_k))
    }
}

impl Default for Reranker {
    fn default() -> Self {
        Self::new().expect("Failed to create default reranker")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reranker_creation() {
        #[cfg(feature = "fastembed")]
        {
            let reranker = Reranker::new();
            assert!(reranker.is_ok());
        }

        #[cfg(not(feature = "fastembed"))]
        {
            let reranker = Reranker::new();
            assert!(reranker.is_err());
        }
    }

    #[test]
    #[cfg(feature = "fastembed")]
    fn test_rerank_empty_results() {
        // Try to create reranker, but skip test if model download fails
        let reranker = match Reranker::new() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Skipping test - reranker initialization failed: {}", e);
                return;
            }
        };

        let results = Vec::new();
        let reranked = reranker.rerank("test query", results, None).unwrap();
        assert_eq!(reranked.len(), 0);
    }

    #[test]
    #[cfg(feature = "fastembed")]
    fn test_rerank_with_results() {
        // Try to create reranker, but skip test if model download fails
        let reranker = match Reranker::new() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Skipping test - reranker initialization failed: {}", e);
                return;
            }
        };

        // Create mock results
        let results = vec![
            HybridSearchResult {
                file_path: "src/auth/user.rs".to_string(),
                similarity_score: 0.80,
                graph_distance: 0,
                combined_score: 0.80,
                relationship: None,
                summary: "User struct definition".to_string(),
                chunk_type: Some("struct".to_string()),
                start_line: Some(1),
                end_line: Some(10),
            },
            HybridSearchResult {
                file_path: "src/auth/login.rs".to_string(),
                similarity_score: 0.75,
                graph_distance: 0,
                combined_score: 0.75,
                relationship: None,
                summary: "Login function implementation".to_string(),
                chunk_type: Some("function".to_string()),
                start_line: Some(20),
                end_line: Some(50),
            },
        ];

        let reranked = reranker.rerank("user authentication", results.clone(), Some(2)).unwrap();

        // Should have same number of results
        assert_eq!(reranked.len(), results.len());

        // Scores should be updated (cross-encoder scores)
        assert_ne!(reranked[0].combined_score, 0.80);
    }

    #[test]
    #[cfg(feature = "fastembed")]
    fn test_rerank_auto() {
        // Try to create reranker, but skip test if model download fails
        let reranker = match Reranker::new() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Skipping test - reranker initialization failed: {}", e);
                return;
            }
        };

        // Test with small result set
        let small_results = vec![
            HybridSearchResult {
                file_path: "test1.rs".to_string(),
                similarity_score: 0.80,
                graph_distance: 0,
                combined_score: 0.80,
                relationship: None,
                summary: "Test file 1".to_string(),
                chunk_type: Some("file".to_string()),
                start_line: None,
                end_line: None,
            },
        ];

        let reranked = reranker.rerank_auto("test", small_results.clone()).unwrap();
        assert_eq!(reranked.len(), small_results.len());
    }
}
