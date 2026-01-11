//! Hybrid Search - Combines Vector Search with Knowledge Graph
//!
//! This module implements **Graph-Based Retrieval** - finding not just semantically
//! similar code, but also following dependency relationships to find ALL related code.
//!
//! ## How It Works
//!
//! 1. **Vector Search**: Find initial results based on semantic similarity
//! 2. **Graph Expansion**: Follow relationships in the knowledge graph to find:
//!    - Direct dependencies (files that import/use the found code)
//!    - Indirect dependencies (transitive closure)
//!    - Related implementations (trait impls, derived types)
//! 3. **Ranking**: Combine vector similarity with graph distance for final ranking
//!
//! ## Example
//!
//! ```text
//! User: "Tell me about User authentication"
//!
//! Step 1: Vector Search finds:
//!   - src/auth/user.rs (struct User)
//!   - src/auth/login.rs (fn login)
//!
//! Step 2: Graph Expansion discovers:
//!   - src/auth/session.rs (uses User)
//!   - src/api/auth_controller.rs (calls login)
//!   - src/middleware/auth.rs (validates User)
//!   - src/db/user_repo.rs (stores User)
//!
//! Result: Complete picture of authentication system (6 files, not just 2!)
//! ```

use crate::berrycode::Result;
use crate::berrycode::vector_search::{VectorSearch, SearchResult};
use crate::berrycode::knowledge_graph::{KnowledgeGraph, CodeEntity, EntityType};
use crate::berrycode::reranker::Reranker;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Hybrid search result with both vector similarity and graph context
#[derive(Debug, Clone, Serialize)]
pub struct HybridSearchResult {
    /// Original vector search result
    pub file_path: String,
    /// Similarity score from vector search
    pub similarity_score: f32,
    /// Graph distance (0 = direct match, 1 = 1 hop away, etc.)
    pub graph_distance: usize,
    /// Combined score (higher is better)
    pub combined_score: f32,
    /// Relationship type (if found via graph expansion)
    pub relationship: Option<String>,
    /// Chunk summary
    pub summary: String,
    /// Chunk type
    pub chunk_type: Option<String>,
    /// Line range
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
}

/// Hybrid search engine combining vector search and knowledge graph
pub struct HybridSearch {
    vector_search: VectorSearch,
    enable_graph_expansion: bool,
}

impl HybridSearch {
    /// Create a new hybrid search engine
    pub fn new(vector_search: VectorSearch) -> Self {
        Self {
            vector_search,
            enable_graph_expansion: true,
        }
    }

    /// Enable or disable graph expansion
    pub fn set_graph_expansion(&mut self, enabled: bool) {
        self.enable_graph_expansion = enabled;
    }

    /// Perform hybrid search: Vector + Graph
    ///
    /// # Arguments
    /// * `query` - Search query
    /// * `limit` - Maximum number of results
    /// * `knowledge_graph` - Optional knowledge graph for expansion
    ///
    /// # Returns
    /// * Vector of hybrid search results
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        knowledge_graph: Option<&KnowledgeGraph>,
    ) -> Result<Vec<HybridSearchResult>> {
        // Step 1: Vector search
        let vector_results = self.vector_search.search(query, limit * 2).await?;

        if !self.enable_graph_expansion || knowledge_graph.is_none() {
            // No graph expansion - just convert vector results
            return Ok(vector_results
                .into_iter()
                .map(|r| HybridSearchResult {
                    file_path: r.file_path,
                    similarity_score: r.score,
                    graph_distance: 0,
                    combined_score: r.score,
                    relationship: None,
                    summary: r.summary,
                    chunk_type: r.chunk_type,
                    start_line: r.start_line,
                    end_line: r.end_line,
                })
                .collect());
        }

        let kg = knowledge_graph.unwrap();

        // Step 2: Graph expansion
        let mut results = Vec::new();
        let mut seen_files = HashSet::new();

        for vector_result in vector_results {
            // Add original result
            if !seen_files.contains(&vector_result.file_path) {
                results.push(HybridSearchResult {
                    file_path: vector_result.file_path.clone(),
                    similarity_score: vector_result.score,
                    graph_distance: 0,
                    combined_score: vector_result.score,
                    relationship: None,
                    summary: vector_result.summary.clone(),
                    chunk_type: vector_result.chunk_type.clone(),
                    start_line: vector_result.start_line,
                    end_line: vector_result.end_line,
                });
                seen_files.insert(vector_result.file_path.clone());
            }

            // Find related files via graph (1-hop neighbors)
            let related = kg.find_related_files(&vector_result.file_path, 1);

            for (related_file, relationship, distance) in related {
                if seen_files.contains(&related_file) {
                    continue;
                }

                // Calculate combined score (decay with graph distance)
                let decay_factor = 0.5_f32.powi(distance as i32);
                let combined_score = vector_result.score * decay_factor;

                results.push(HybridSearchResult {
                    file_path: related_file.clone(),
                    similarity_score: 0.0, // Not directly matched by vector search
                    graph_distance: distance,
                    combined_score,
                    relationship: Some(relationship.clone()),
                    summary: format!("Related via {}", relationship),
                    chunk_type: None,
                    start_line: None,
                    end_line: None,
                });

                seen_files.insert(related_file);
            }
        }

        // Sort by combined score
        results.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());

        // Limit results
        results.truncate(limit);

        tracing::debug!(
            "🔍 Hybrid search: {} results (vector: {}, expanded: {})",
            results.len(),
            results.iter().filter(|r| r.graph_distance == 0).count(),
            results.iter().filter(|r| r.graph_distance > 0).count()
        );

        Ok(results)
    }

    /// Get the underlying vector search engine (for direct access)
    pub fn vector_search(&self) -> &VectorSearch {
        &self.vector_search
    }

    /// Get mutable reference to vector search
    pub fn vector_search_mut(&mut self) -> &mut VectorSearch {
        &mut self.vector_search
    }

    /// Perform hybrid search with optional reranking
    ///
    /// # Arguments
    /// * `query` - Search query
    /// * `limit` - Maximum number of results
    /// * `knowledge_graph` - Optional knowledge graph for expansion
    /// * `reranker` - Optional reranker for precision improvement
    ///
    /// # Returns
    /// * Vector of hybrid search results (reranked if reranker provided)
    pub async fn search_with_reranking(
        &self,
        query: &str,
        limit: usize,
        knowledge_graph: Option<&KnowledgeGraph>,
        reranker: Option<&Reranker>,
    ) -> Result<Vec<HybridSearchResult>> {
        // Step 1: Hybrid search (vector + graph)
        let mut results = self.search(query, limit, knowledge_graph).await?;

        // Step 2: Optional reranking
        if let Some(reranker) = reranker {
            results = reranker.rerank_auto(query, results)?;

            tracing::debug!(
                "🎯 Applied reranking to {} results",
                results.len()
            );
        }

        Ok(results)
    }
}

// Extend KnowledgeGraph with helper method
impl KnowledgeGraph {
    /// Find files related to the given file via graph traversal
    ///
    /// Returns: Vec<(file_path, relationship_type, distance)>
    pub fn find_related_files(&self, file_path: &str, max_hops: usize) -> Vec<(String, String, usize)> {
        // This is a placeholder implementation
        // In a full implementation, you would:
        // 1. Find the entity node for this file
        // 2. Perform BFS/DFS traversal up to max_hops
        // 3. Collect all reachable files with their relationships

        // For now, return empty (to be implemented when KnowledgeGraph is fully built)
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_hybrid_search_creation() {
        let temp_dir = TempDir::new().unwrap();
        let vs = VectorSearch::new(temp_dir.path()).unwrap();
        let hs = HybridSearch::new(vs);

        assert!(hs.enable_graph_expansion);
    }

    #[tokio::test]
    async fn test_hybrid_search_without_graph() {
        let temp_dir = TempDir::new().unwrap();
        let mut vs = VectorSearch::new(temp_dir.path()).unwrap();

        // Create a test file
        let test_file = temp_dir.path().join("test.rs");
        std::fs::write(&test_file, "fn main() { println!(\"Hello\"); }").unwrap();

        vs.index_file(&test_file).await.unwrap();

        let hs = HybridSearch::new(vs);

        // Search without graph should work
        let results = hs.search("test", 10, None).await.unwrap();

        // Should have at least 1 result
        assert!(!results.is_empty());

        // All results should have graph_distance = 0 (no expansion)
        for result in results {
            assert_eq!(result.graph_distance, 0);
        }
    }

    #[test]
    fn test_graph_expansion_toggle() {
        let temp_dir = TempDir::new().unwrap();
        let vs = VectorSearch::new(temp_dir.path()).unwrap();
        let mut hs = HybridSearch::new(vs);

        assert!(hs.enable_graph_expansion);

        hs.set_graph_expansion(false);
        assert!(!hs.enable_graph_expansion);

        hs.set_graph_expansion(true);
        assert!(hs.enable_graph_expansion);
    }

    #[tokio::test]
    #[cfg(feature = "fastembed")]
    async fn test_hybrid_search_with_reranking() {
        use crate::berrycode::reranker::Reranker;

        let temp_dir = TempDir::new().unwrap();
        let mut vs = VectorSearch::new(temp_dir.path()).unwrap();

        // Create test files
        let test_file1 = temp_dir.path().join("auth.rs");
        std::fs::write(&test_file1, "fn authenticate_user() { /* auth logic */ }").unwrap();

        let test_file2 = temp_dir.path().join("login.rs");
        std::fs::write(&test_file2, "fn login() { /* login logic */ }").unwrap();

        vs.index_file(&test_file1).await.unwrap();
        vs.index_file(&test_file2).await.unwrap();

        let hs = HybridSearch::new(vs);
        let reranker = Reranker::new().unwrap();

        // Search with reranking
        let results = hs
            .search_with_reranking("authentication", 10, None, Some(&reranker))
            .await
            .unwrap();

        // Should have results
        assert!(!results.is_empty());

        // Results should have been reranked (scores updated by cross-encoder)
        // NOTE: Cross-encoder scores can be negative (they are logit values, not probabilities)
        // What matters is the relative ordering - higher scores are more relevant
        for result in &results {
            // Just verify that results exist and have valid (finite) scores
            assert!(result.combined_score.is_finite(),
                "Score should be finite, got {} for {}",
                result.combined_score, result.file_path);
        }

        // Verify results are sorted by score (highest first)
        for i in 0..results.len().saturating_sub(1) {
            assert!(
                results[i].combined_score >= results[i + 1].combined_score,
                "Results should be sorted by score: {} ({}) should be >= {} ({})",
                results[i].file_path, results[i].combined_score,
                results[i + 1].file_path, results[i + 1].combined_score
            );
        }
    }
}
