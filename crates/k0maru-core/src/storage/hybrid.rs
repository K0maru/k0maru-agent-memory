//! Reciprocal Rank Fusion (RRF) Hybrid Search Engine.
//!
//! Combines BM25 full-text keyword search, cosine vector embeddings similarity,
//! and 1-hop WikiLinks graph adjacency into a unified retrieval engine.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::core::traits::CacheStorage;
use crate::storage::SqliteStorage;
use crate::vector::EmbeddingEngine;

/// Search execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    /// Reciprocal Rank Fusion combining BM25, Vector, and Graph Boost.
    Hybrid,
    /// Pure BM25 full-text keyword search via FTS5.
    Bm25,
    /// Pure semantic vector nearest-neighbor search via sqlite-vec.
    Vector,
}

/// Unified search result representing a ranked document hit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    /// Relative path of the document within the vault.
    pub path: String,
    /// Extracted document title.
    pub title: String,
    /// Combined retrieval score.
    pub score: f32,
    /// 1-based rank in BM25 search results, if matched.
    pub bm25_rank: Option<usize>,
    /// 1-based rank in Vector search results, if matched.
    pub vector_rank: Option<usize>,
    /// Graph boost weight applied to this result.
    pub graph_boost: f32,
    /// Contextual snippet extracted around matching query terms or document start.
    pub snippet: String,
}

/// Hybrid search engine combining lexical (BM25), semantic (Vector), and graph topology signals.
pub struct HybridSearchEngine<'a> {
    storage: &'a SqliteStorage,
    embedder: Option<Arc<dyn EmbeddingEngine>>,
    rrf_k: usize,
    weight_bm25: f32,
    weight_vector: f32,
    graph_boost_weight: f32,
}

impl<'a> HybridSearchEngine<'a> {
    /// Default RRF smoothing constant $k = 60$.
    pub const DEFAULT_RRF_K: usize = 60;
    /// Default BM25 fusion weight ($0.5$).
    pub const DEFAULT_WEIGHT_BM25: f32 = 0.5;
    /// Default vector fusion weight ($0.5$).
    pub const DEFAULT_WEIGHT_VECTOR: f32 = 0.5;
    /// Default graph boost weight added for connected nodes ($0.05$).
    pub const DEFAULT_GRAPH_BOOST: f32 = 0.05;

    /// Creates a new `HybridSearchEngine` bound to SQLite storage and optional embedding engine.
    pub fn new(storage: &'a SqliteStorage, embedder: Option<Arc<dyn EmbeddingEngine>>) -> Self {
        Self {
            storage,
            embedder,
            rrf_k: Self::DEFAULT_RRF_K,
            weight_bm25: Self::DEFAULT_WEIGHT_BM25,
            weight_vector: Self::DEFAULT_WEIGHT_VECTOR,
            graph_boost_weight: Self::DEFAULT_GRAPH_BOOST,
        }
    }

    /// Sets the RRF smoothing constant $k$ (default 60).
    pub fn with_rrf_k(mut self, k: usize) -> Self {
        self.rrf_k = k;
        self
    }

    /// Sets the relative weights for BM25 and Vector search.
    pub fn with_weights(mut self, weight_bm25: f32, weight_vector: f32) -> Self {
        self.weight_bm25 = weight_bm25;
        self.weight_vector = weight_vector;
        self
    }

    /// Sets the graph boost score added to connected notes in hybrid mode.
    pub fn with_graph_boost_weight(mut self, graph_boost_weight: f32) -> Self {
        self.graph_boost_weight = graph_boost_weight;
        self
    }

    /// Returns the current RRF smoothing constant $k$.
    pub fn rrf_k(&self) -> usize {
        self.rrf_k
    }

    /// Returns the BM25 fusion weight.
    pub fn weight_bm25(&self) -> f32 {
        self.weight_bm25
    }

    /// Returns the Vector fusion weight.
    pub fn weight_vector(&self) -> f32 {
        self.weight_vector
    }

    /// Returns the graph boost weight.
    pub fn graph_boost_weight(&self) -> f32 {
        self.graph_boost_weight
    }

    /// Executes search with the specified mode and result limit.
    pub fn search(
        &self,
        query: &str,
        mode: SearchMode,
        limit: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let trimmed = query.trim();
        if trimmed.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        match mode {
            SearchMode::Bm25 => self.search_bm25(trimmed, limit),
            SearchMode::Vector => self.search_vector(trimmed, limit),
            SearchMode::Hybrid => self.search_hybrid(trimmed, limit),
        }
    }

    fn search_bm25(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let docs = self.storage.search_fts(query, limit)?;
        let mut results = Vec::with_capacity(docs.len());

        for (i, doc) in docs.into_iter().enumerate() {
            let rank = i + 1;
            let score = 1.0 / (self.rrf_k as f32 + rank as f32);
            let snippet = generate_snippet(&doc.body, query, 200);

            results.push(SearchResult {
                path: doc.path.to_string_lossy().to_string(),
                title: doc.title,
                score,
                bm25_rank: Some(rank),
                vector_rank: None,
                graph_boost: 0.0,
                snippet,
            });
        }

        Ok(results)
    }

    fn search_vector(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let embedder = self
            .embedder
            .as_ref()
            .ok_or("No embedding engine configured for vector search")?;

        let query_emb = embedder.embed(query)?;
        let hits = self.storage.search_vectors(&query_emb, limit)?;
        if hits.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(hits.len());
        for (i, (path_str, _distance)) in hits.into_iter().enumerate() {
            let rank = i + 1;
            let score = 1.0 / (self.rrf_k as f32 + rank as f32);

            let path = PathBuf::from(&path_str);
            let (title, body) = match self.storage.get_document(&path)? {
                Some(doc) => (doc.title, doc.body),
                None => (
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&path_str)
                        .to_string(),
                    String::new(),
                ),
            };

            let snippet = generate_snippet(&body, query, 200);

            results.push(SearchResult {
                path: path_str,
                title,
                score,
                bm25_rank: None,
                vector_rank: Some(rank),
                graph_boost: 0.0,
                snippet,
            });
        }

        Ok(results)
    }

    fn search_hybrid(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let pool_limit = (limit * 3).max(20);

        // 1. Lexical BM25 retrieval
        let bm25_docs = self.storage.search_fts(query, pool_limit)?;

        // 2. Semantic vector retrieval (if embedder configured)
        let vector_hits = if let Some(ref embedder) = self.embedder {
            match embedder.embed(query) {
                Ok(emb) => self
                    .storage
                    .search_vectors(&emb, pool_limit)
                    .unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        if bm25_docs.is_empty() && vector_hits.is_empty() {
            return Ok(Vec::new());
        }

        // Determine normalized weights when one modality is unavailable or empty
        let has_bm25 = !bm25_docs.is_empty();
        let has_vector = !vector_hits.is_empty();

        let (w_bm25, w_vec) = match (has_bm25, has_vector) {
            (true, true) => {
                let total_weight = self.weight_bm25 + self.weight_vector;
                if total_weight > 0.0 {
                    (
                        self.weight_bm25 / total_weight,
                        self.weight_vector / total_weight,
                    )
                } else {
                    (0.5, 0.5)
                }
            }
            (true, false) => (1.0, 0.0),
            (false, true) => (0.0, 1.0),
            (false, false) => (0.0, 0.0),
        };

        // 3. Collate candidate documents
        struct CandidateEntry {
            path: String,
            title: String,
            body: String,
            bm25_rank: Option<usize>,
            vector_rank: Option<usize>,
        }

        let mut candidates: HashMap<String, CandidateEntry> = HashMap::new();

        for (i, doc) in bm25_docs.into_iter().enumerate() {
            let path_str = doc.path.to_string_lossy().to_string();
            candidates.insert(
                path_str.clone(),
                CandidateEntry {
                    path: path_str,
                    title: doc.title,
                    body: doc.body,
                    bm25_rank: Some(i + 1),
                    vector_rank: None,
                },
            );
        }

        for (i, (path_str, _)) in vector_hits.into_iter().enumerate() {
            let rank = i + 1;
            if let Some(entry) = candidates.get_mut(&path_str) {
                entry.vector_rank = Some(rank);
            } else {
                let path = PathBuf::from(&path_str);
                let (title, body) = match self.storage.get_document(&path)? {
                    Some(doc) => (doc.title, doc.body),
                    None => (
                        path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or(&path_str)
                            .to_string(),
                        String::new(),
                    ),
                };
                candidates.insert(
                    path_str.clone(),
                    CandidateEntry {
                        path: path_str,
                        title,
                        body,
                        bm25_rank: None,
                        vector_rank: Some(rank),
                    },
                );
            }
        }

        // 4. Graph Boost evaluation
        let pool_paths: HashSet<String> = candidates.keys().cloned().collect();

        let mut results = Vec::with_capacity(candidates.len());

        for cand in candidates.values() {
            let mut has_graph_connection = false;

            // (a) Check forward links from cand to any other candidate in the pool
            if let Ok(outgoing) = self.storage.get_outgoing_links(Path::new(&cand.path)) {
                for link in outgoing {
                    for other_path in &pool_paths {
                        if other_path != &cand.path {
                            if let Some(other_cand) = candidates.get(other_path) {
                                if target_matches_candidate(
                                    &link.target,
                                    &other_cand.path,
                                    &other_cand.title,
                                ) {
                                    has_graph_connection = true;
                                    break;
                                }
                            }
                        }
                    }
                    if has_graph_connection {
                        break;
                    }
                }
            }

            // (b) Check backlinks to cand from any other candidate in the pool
            if !has_graph_connection {
                if let Ok(backlinks) = self.storage.get_backlinks(&cand.title) {
                    for src in backlinks {
                        let s = src.to_string_lossy();
                        if s != cand.path.as_str() && pool_paths.contains(s.as_ref()) {
                            has_graph_connection = true;
                            break;
                        }
                    }
                }
            }

            // Also check backlinks using file stem if different from title
            if !has_graph_connection {
                if let Some(stem) = Path::new(&cand.path).file_stem().and_then(|s| s.to_str()) {
                    if stem != cand.title {
                        if let Ok(stem_backlinks) = self.storage.get_backlinks(stem) {
                            for src in stem_backlinks {
                                let s = src.to_string_lossy();
                                if s != cand.path.as_str() && pool_paths.contains(s.as_ref()) {
                                    has_graph_connection = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            let graph_boost = if has_graph_connection {
                self.graph_boost_weight
            } else {
                0.0
            };

            // 5. Compute RRF score
            let mut rrf_score = 0.0f32;
            if let Some(r_bm25) = cand.bm25_rank {
                rrf_score += w_bm25 / (self.rrf_k as f32 + r_bm25 as f32);
            }
            if let Some(r_vec) = cand.vector_rank {
                rrf_score += w_vec / (self.rrf_k as f32 + r_vec as f32);
            }

            let final_score = rrf_score + graph_boost;
            let snippet = generate_snippet(&cand.body, query, 200);

            results.push(SearchResult {
                path: cand.path.clone(),
                title: cand.title.clone(),
                score: final_score,
                bm25_rank: cand.bm25_rank,
                vector_rank: cand.vector_rank,
                graph_boost,
                snippet,
            });
        }

        // 6. Sort by descending final score with deterministic tie-breaking
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| match (a.bm25_rank, b.bm25_rank) {
                    (Some(ra), Some(rb)) => ra.cmp(&rb),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                })
                .then_with(|| match (a.vector_rank, b.vector_rank) {
                    (Some(ra), Some(rb)) => ra.cmp(&rb),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                })
                .then_with(|| a.path.cmp(&b.path))
        });

        results.truncate(limit);
        Ok(results)
    }
}

/// Checks whether a WikiLink target string matches a candidate document.
fn target_matches_candidate(target: &str, candidate_path: &str, candidate_title: &str) -> bool {
    let clean_target = target.strip_suffix(".md").unwrap_or(target);
    if clean_target.eq_ignore_ascii_case(candidate_title) {
        return true;
    }
    if target.eq_ignore_ascii_case(candidate_path) {
        return true;
    }
    if let Some(target_stem) = Path::new(clean_target).file_name().and_then(|f| f.to_str()) {
        if target_stem.eq_ignore_ascii_case(candidate_title) {
            return true;
        }
        if let Some(cand_stem) = Path::new(candidate_path)
            .file_stem()
            .and_then(|f| f.to_str())
        {
            if target_stem.eq_ignore_ascii_case(cand_stem) {
                return true;
            }
        }
    }
    let cand_clean = candidate_path.strip_suffix(".md").unwrap_or(candidate_path);
    if cand_clean.ends_with(&format!("/{}", clean_target)) {
        return true;
    }
    false
}

/// Generates a relevant text snippet centered around matching query terms or start of text.
fn generate_snippet(body: &str, query: &str, max_len: usize) -> String {
    let trimmed_body = body.trim();
    if trimmed_body.is_empty() {
        return String::new();
    }

    let lower_body = trimmed_body.to_lowercase();
    let query_terms: Vec<&str> = query.split_whitespace().filter(|t| !t.is_empty()).collect();

    let mut match_byte_pos = None;
    let lower_query = query.to_lowercase();
    if let Some(pos) = lower_body.find(&lower_query) {
        match_byte_pos = Some(pos);
    } else {
        for term in query_terms {
            let term_lower = term.to_lowercase();
            if let Some(pos) = lower_body.find(&term_lower) {
                match_byte_pos = Some(pos);
                break;
            }
        }
    }

    let char_indices: Vec<(usize, char)> = trimmed_body.char_indices().collect();
    if char_indices.len() <= max_len {
        return trimmed_body.replace('\n', " ").trim().to_string();
    }

    if let Some(byte_pos) = match_byte_pos {
        let char_idx = char_indices
            .iter()
            .position(|&(idx, _)| idx >= byte_pos)
            .unwrap_or(0);

        let half = max_len / 2;
        let start_char = char_idx.saturating_sub(half);
        let end_char = (start_char + max_len).min(char_indices.len());
        let start_char = if end_char == char_indices.len() && char_indices.len() > max_len {
            char_indices.len().saturating_sub(max_len)
        } else {
            start_char
        };

        let start_byte = char_indices[start_char].0;
        let end_byte = if end_char < char_indices.len() {
            char_indices[end_char].0
        } else {
            trimmed_body.len()
        };

        let slice = &trimmed_body[start_byte..end_byte];
        let mut snippet = slice.replace('\n', " ").trim().to_string();
        if start_char > 0 {
            snippet = format!("...{}", snippet);
        }
        if end_char < char_indices.len() {
            snippet = format!("{}...", snippet);
        }
        snippet
    } else {
        let end_byte = char_indices[max_len.min(char_indices.len() - 1)].0;
        let mut snippet = trimmed_body[..end_byte]
            .replace('\n', " ")
            .trim()
            .to_string();
        if char_indices.len() > max_len {
            snippet.push_str("...");
        }
        snippet
    }
}
