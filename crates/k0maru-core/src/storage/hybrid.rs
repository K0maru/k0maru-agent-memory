//! Reciprocal Rank Fusion (RRF) Hybrid Search Engine.
//!
//! Combines BM25 full-text keyword search, cosine vector embeddings similarity,
//! and 1-hop WikiLinks graph adjacency into a unified retrieval engine.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::core::models::{Document, HierarchyLevel};
use crate::core::traits::CacheStorage;
use crate::storage::locate::{
    format_locator, locate_sections_for_results, vector_scope_enabled, SectionLocator,
};
use crate::storage::rank_signals::{
    compute_access_reinforcement, compute_decay, current_unix_timestamp, fetch_doc_ids_by_paths,
    fetch_paths_by_doc_ids, DecayConfig,
};
use crate::storage::SqliteStorage;
use crate::text::FtsQueryTier;
use crate::vector::EmbeddingEngine;

/// Search results plus scope-level tier and lifecycle metadata for federated fusion.
pub(crate) struct ScopeSearchResult {
    pub results: Vec<SearchResult>,
    pub bm25_tier: Option<FtsQueryTier>,
    pub inactive_paths: HashSet<String>,
}

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

/// Graph boost strategy applied to connected active candidates in [`SearchMode::Hybrid`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphBoostMode {
    /// Relative boost `gamma * S_max` where `S_max` is the highest direct RRF score in the pool.
    Relative(f32),
    /// Fixed absolute boost added to connected active candidates.
    Absolute(f32),
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
    /// Best matching section metadata attached during post-rank locate, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<SectionLocator>,
    /// Parent document path if this hit was added via 1-hop graph expansion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expanded_from: Option<String>,
    /// Optional scope tag (e.g. `"global"`) for federated recall.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

impl SearchResult {
    /// Renders the canonical `[@global:]<path>[:<start>-<end>][#<heading>]` locator string.
    pub fn locator(&self) -> String {
        format_locator(&self.path, self.scope.as_deref(), self.section.as_ref())
    }
}

#[derive(Default)]
struct VectorRetrievalResult {
    doc_hits: Vec<(String, f32)>,
    vec_section_ranks: HashMap<i64, usize>,
}

/// Hybrid search engine combining lexical (BM25), semantic (Vector), and graph topology signals.
pub struct HybridSearchEngine<'a> {
    storage: &'a SqliteStorage,
    embedder: Option<Arc<dyn EmbeddingEngine>>,
    rrf_k: usize,
    weight_bm25: f32,
    weight_vector: f32,
    graph_boost: GraphBoostMode,
    section_lexical_weight: f32,
    decay: DecayConfig,
    now_override: Option<i64>,
}

impl<'a> HybridSearchEngine<'a> {
    /// Default RRF smoothing constant $k = 60$.
    pub const DEFAULT_RRF_K: usize = 60;
    /// Default BM25 fusion weight ($0.5$).
    pub const DEFAULT_WEIGHT_BM25: f32 = 0.5;
    /// Default vector fusion weight ($0.5$).
    pub const DEFAULT_WEIGHT_VECTOR: f32 = 0.5;
    /// Default relative graph boost ratio $\gamma = 0.25$ applied to $S_{\max}$.
    pub const DEFAULT_GRAPH_BOOST_RATIO: f32 = 0.25;
    /// Legacy default absolute graph boost weight ($0.004$).
    #[deprecated(note = "Use DEFAULT_GRAPH_BOOST_RATIO with GraphBoostMode::Relative instead")]
    pub const DEFAULT_GRAPH_BOOST: f32 = 0.004;
    /// Score multiplier applied to superseded or deprecated notes (0.2).
    pub const SUPERSEDED_PENALTY_MULTIPLIER: f32 = 0.2;
    /// Lexical RRF multiplier applied when FTS only matched via [`FtsQueryTier::AnyTokens`] (`OR` fallback).
    pub const ANY_TOKENS_LEXICAL_MULTIPLIER: f32 = 0.55;
    /// Default 1-hop expansion multiplier for outgoing edges `p -> n`.
    pub const DEFAULT_EXPANSION_OUT_BETA: f32 = 0.50;
    /// Default 1-hop expansion multiplier for incoming edges `n -> p` when `n` is L3 evergreen.
    pub const DEFAULT_EXPANSION_IN_L3_BETA: f32 = 0.35;
    /// Maximum number of top active direct parents considered for 1-hop expansion.
    pub const MAX_EXPANSION_PARENTS: usize = 10;

    fn lexical_tier_multiplier(tier: Option<FtsQueryTier>) -> f32 {
        match tier {
            Some(FtsQueryTier::AnyTokens) => Self::ANY_TOKENS_LEXICAL_MULTIPLIER,
            _ => 1.0,
        }
    }

    /// Checks document lifecycle and applies deprecation penalty and snippet annotation if inactive.
    pub fn apply_lifecycle_penalty(
        doc: Option<&Document>,
        score: &mut f32,
        graph_boost: &mut f32,
        snippet: &mut String,
    ) {
        if let Some(doc) = doc {
            if !doc.is_active() {
                *graph_boost = 0.0;
                *score *= Self::SUPERSEDED_PENALTY_MULTIPLIER;
                if !snippet.starts_with("[SUPERSEDED") && !snippet.starts_with("[DEPRECATED") {
                    if let Some(target) = doc.superseded_by() {
                        *snippet = if snippet.is_empty() {
                            format!("[SUPERSEDED by {}]", target)
                        } else {
                            format!("[SUPERSEDED by {}] {}", target, snippet)
                        };
                    } else {
                        *snippet = if snippet.is_empty() {
                            "[DEPRECATED]".to_string()
                        } else {
                            format!("[DEPRECATED] {}", snippet)
                        };
                    }
                }
            }
        }
    }

    /// Follows `superseded_by` up to 16 hops (with cycle detection via `HashSet`) using storage, returning the final `[[...]]` target string.
    pub fn resolve_supersession_chain(&self, path_str: &str, initial_target: &str) -> String {
        let mut final_target = initial_target.to_string();
        let mut seen = HashSet::new();
        seen.insert(path_str.to_string());
        let mut hops = 0;

        // Helper to resolve a document from storage by path, name, or stem
        let resolve_doc = |target_str: &str| -> Option<Document> {
            let clean = target_str
                .trim()
                .trim_start_matches("[[")
                .trim_end_matches("]]")
                .trim();
            if clean.is_empty() {
                return None;
            }
            // 1. Direct path
            if let Ok(Some(d)) = self.storage.get_document(Path::new(clean)) {
                return Some(d);
            }
            // 2. Direct path with .md
            if !clean.ends_with(".md") {
                if let Ok(Some(d)) = self
                    .storage
                    .get_document(Path::new(&format!("{}.md", clean)))
                {
                    return Some(d);
                }
            }
            // 3. Search cached metadata by stem or file_name
            let stem = clean.strip_suffix(".md").unwrap_or(clean);
            if let Ok(meta_list) = self.storage.get_cached_metadata() {
                for meta in meta_list {
                    let matches_name = meta
                        .path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .map(|n| n == clean || n == format!("{}.md", clean))
                        .unwrap_or(false);
                    let matches_stem = meta
                        .path
                        .file_stem()
                        .and_then(|f| f.to_str())
                        .map(|s| s == stem)
                        .unwrap_or(false);
                    if matches_name || matches_stem {
                        if let Ok(Some(d)) = self.storage.get_document(&meta.path) {
                            return Some(d);
                        }
                    }
                }
            }
            None
        };

        while hops < 16 {
            hops += 1;
            let clean_target = final_target
                .trim()
                .trim_start_matches("[[")
                .trim_end_matches("]]")
                .trim()
                .to_string();
            if !seen.insert(clean_target) {
                break;
            }
            if let Some(next_doc) = resolve_doc(&final_target) {
                let next_path = next_doc.path.to_string_lossy().to_string();
                seen.insert(next_path);
                if let Some(next_target) = next_doc.superseded_by() {
                    final_target = next_target;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        final_target
    }

    /// Checks document lifecycle by path from storage and applies deprecation penalty and snippet annotation.
    pub fn apply_lifecycle_penalty_by_path(
        &self,
        path_str: &str,
        score: &mut f32,
        graph_boost: &mut f32,
        snippet: &mut String,
    ) {
        if let Ok(Some(doc)) = self.storage.get_document(Path::new(path_str)) {
            if !doc.is_active() {
                *graph_boost = 0.0;
                *score *= Self::SUPERSEDED_PENALTY_MULTIPLIER;
                *snippet =
                    format_doc_lifecycle_snippet(self, path_str, &doc, std::mem::take(snippet));
            }
        }
    }

    /// Creates a new `HybridSearchEngine` bound to SQLite storage and optional embedding engine.
    pub fn new(storage: &'a SqliteStorage, embedder: Option<Arc<dyn EmbeddingEngine>>) -> Self {
        Self {
            storage,
            embedder,
            rrf_k: Self::DEFAULT_RRF_K,
            weight_bm25: Self::DEFAULT_WEIGHT_BM25,
            weight_vector: Self::DEFAULT_WEIGHT_VECTOR,
            graph_boost: GraphBoostMode::Relative(Self::DEFAULT_GRAPH_BOOST_RATIO),
            section_lexical_weight: 0.0,
            decay: DecayConfig::default(),
            now_override: None,
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

    /// Sets the relative graph boost ratio $\gamma$ applied to $S_{\max}$ in hybrid mode.
    pub fn with_graph_boost_ratio(mut self, gamma: f32) -> Self {
        self.graph_boost = GraphBoostMode::Relative(gamma);
        self
    }

    /// Sets an absolute graph boost override added to connected active notes in hybrid mode.
    pub fn with_graph_boost_weight(mut self, graph_boost_weight: f32) -> Self {
        self.graph_boost = GraphBoostMode::Absolute(graph_boost_weight);
        self
    }

    /// Sets the optional section lexical RRF channel weight (default `0.0`, locate-only).
    pub fn with_section_lexical_weight(mut self, section_lexical_weight: f32) -> Self {
        self.section_lexical_weight = section_lexical_weight;
        self
    }

    /// Overrides the tiered time decay configuration.
    pub fn with_decay(mut self, decay: DecayConfig) -> Self {
        self.decay = decay;
        self
    }

    /// Overrides the current Unix timestamp (seconds) for deterministic decay and access scoring.
    pub fn with_now(mut self, now: i64) -> Self {
        self.now_override = Some(now);
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

    /// Returns the configured graph boost parameter (ratio for `Relative`, weight for `Absolute`).
    pub fn graph_boost_weight(&self) -> f32 {
        match self.graph_boost {
            GraphBoostMode::Relative(gamma) => gamma,
            GraphBoostMode::Absolute(abs) => abs,
        }
    }

    /// Returns the active graph boost strategy.
    pub fn graph_boost_mode(&self) -> GraphBoostMode {
        self.graph_boost
    }

    /// Returns the active tiered decay configuration.
    pub fn decay(&self) -> DecayConfig {
        self.decay
    }

    /// Returns the section lexical channel weight.
    pub fn section_lexical_weight(&self) -> f32 {
        self.section_lexical_weight
    }

    /// Executes search with the specified mode and result limit.
    pub fn search(
        &self,
        query: &str,
        mode: SearchMode,
        limit: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        Ok(self.search_with_meta(query, mode, limit)?.results)
    }

    pub(crate) fn search_with_meta(
        &self,
        query: &str,
        mode: SearchMode,
        limit: usize,
    ) -> Result<ScopeSearchResult, Box<dyn std::error::Error>> {
        let trimmed = query.trim();
        if trimmed.is_empty() || limit == 0 {
            return Ok(ScopeSearchResult {
                results: Vec::new(),
                bm25_tier: None,
                inactive_paths: HashSet::new(),
            });
        }

        match mode {
            SearchMode::Bm25 => self.search_bm25(trimmed, limit),
            SearchMode::Vector => self.search_vector(trimmed, limit),
            SearchMode::Hybrid => self.search_hybrid(trimmed, limit),
        }
    }

    /// Retrieves document-level vector hits from either section KNN (when ready) or
    /// document KNN (never mixing the two), plus a map of `section_id -> 1-based section KNN rank`.
    fn retrieve_vector_hits(
        &self,
        embedder: &dyn EmbeddingEngine,
        query: &str,
        pool_limit: usize,
    ) -> Result<VectorRetrievalResult, Box<dyn std::error::Error>> {
        if !vector_scope_enabled(self.storage, embedder) {
            return Ok(VectorRetrievalResult::default());
        }

        let query_emb = embedder.embed(query)?;
        if self.storage.section_vectors_ready(embedder.model_name()) {
            let k = (pool_limit * 5).min(4096);
            let sec_hits = self.storage.search_section_vectors(&query_emb, k)?;
            let mut vec_section_ranks: HashMap<i64, usize> = HashMap::with_capacity(sec_hits.len());
            let mut seen_docs: HashSet<i64> = HashSet::new();
            let mut doc_hits: Vec<(String, f32)> = Vec::new();

            for (idx, hit) in sec_hits.into_iter().enumerate() {
                vec_section_ranks.entry(hit.section_id).or_insert(idx + 1);
                if seen_docs.insert(hit.doc_id) && doc_hits.len() < pool_limit {
                    doc_hits.push((hit.path, hit.distance));
                }
            }
            Ok(VectorRetrievalResult {
                doc_hits,
                vec_section_ranks,
            })
        } else {
            let doc_hits = self.storage.search_vectors(&query_emb, pool_limit)?;
            Ok(VectorRetrievalResult {
                doc_hits,
                vec_section_ranks: HashMap::new(),
            })
        }
    }

    fn search_bm25(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<ScopeSearchResult, Box<dyn std::error::Error>> {
        let pool_limit = (limit * 3).max(20);
        let (docs, bm25_tier) = self.storage.search_fts_with_tier(query, pool_limit)?;
        let tier_mult = Self::lexical_tier_multiplier(bm25_tier);
        let mut results = Vec::with_capacity(docs.len());
        let mut inactive_paths = HashSet::new();

        for (i, doc) in docs.into_iter().enumerate() {
            let rank = i + 1;
            let path_str = doc.path.to_string_lossy().to_string();
            let mut score = tier_mult / (self.rrf_k as f32 + rank as f32);
            let mut snippet = generate_snippet(&doc.body, query, 200);
            let mut graph_boost = 0.0;

            if !doc.is_active() {
                inactive_paths.insert(path_str.clone());
            }
            self.apply_lifecycle_penalty_by_path(
                &path_str,
                &mut score,
                &mut graph_boost,
                &mut snippet,
            );

            results.push(SearchResult {
                path: path_str,
                title: doc.title,
                score,
                bm25_rank: Some(rank),
                vector_rank: None,
                graph_boost,
                snippet,
                section: None,
                expanded_from: None,
                scope: None,
            });
        }

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
                .then_with(|| a.path.cmp(&b.path))
        });

        results.truncate(limit);
        let empty_vec_ranks = HashMap::new();
        locate_sections_for_results(self, self.storage, query, &mut results, &empty_vec_ranks)?;
        Ok(ScopeSearchResult {
            results,
            bm25_tier,
            inactive_paths,
        })
    }

    fn search_vector(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<ScopeSearchResult, Box<dyn std::error::Error>> {
        let embedder = self
            .embedder
            .as_ref()
            .ok_or("No embedding engine configured for vector search")?;

        let pool_limit = (limit * 3).max(20);
        let VectorRetrievalResult {
            doc_hits: hits,
            vec_section_ranks,
        } = self.retrieve_vector_hits(&**embedder, query, pool_limit)?;
        if hits.is_empty() {
            return Ok(ScopeSearchResult {
                results: Vec::new(),
                bm25_tier: None,
                inactive_paths: HashSet::new(),
            });
        }

        let mut results = Vec::with_capacity(hits.len());
        let mut inactive_paths = HashSet::new();
        for (i, (path_str, _distance)) in hits.into_iter().enumerate() {
            let path = PathBuf::from(&path_str);
            let Some(doc) = self.storage.get_document(&path)? else {
                continue;
            };
            let rank = i + 1;
            let mut score = 1.0 / (self.rrf_k as f32 + rank as f32);
            if !doc.is_active() {
                inactive_paths.insert(path_str.clone());
            }
            let (title, body) = (doc.title, doc.body);

            let mut snippet = generate_snippet(&body, query, 200);
            let mut graph_boost = 0.0;

            self.apply_lifecycle_penalty_by_path(
                &path_str,
                &mut score,
                &mut graph_boost,
                &mut snippet,
            );

            results.push(SearchResult {
                path: path_str,
                title,
                score,
                bm25_rank: None,
                vector_rank: Some(rank),
                graph_boost,
                snippet,
                section: None,
                expanded_from: None,
                scope: None,
            });
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| match (a.vector_rank, b.vector_rank) {
                    (Some(ra), Some(rb)) => ra.cmp(&rb),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                })
                .then_with(|| a.path.cmp(&b.path))
        });

        results.truncate(limit);
        locate_sections_for_results(self, self.storage, query, &mut results, &vec_section_ranks)?;
        Ok(ScopeSearchResult {
            results,
            bm25_tier: None,
            inactive_paths,
        })
    }

    fn search_hybrid(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<ScopeSearchResult, Box<dyn std::error::Error>> {
        let pool_limit = (limit * 3).max(20);

        // 1. Lexical BM25 retrieval
        let (bm25_docs, bm25_tier) = self.storage.search_fts_with_tier(query, pool_limit)?;
        let bm25_tier_mult = Self::lexical_tier_multiplier(bm25_tier);

        // 2. Semantic vector retrieval (gated by embed_model match)
        let VectorRetrievalResult {
            doc_hits: vector_hits,
            vec_section_ranks,
        } = if let Some(ref embedder) = self.embedder {
            self.retrieve_vector_hits(&**embedder, query, pool_limit)
                .unwrap_or_default()
        } else {
            VectorRetrievalResult::default()
        };

        if bm25_docs.is_empty() && vector_hits.is_empty() {
            return Ok(ScopeSearchResult {
                results: Vec::new(),
                bm25_tier: None,
                inactive_paths: HashSet::new(),
            });
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
        let mut candidates: HashMap<String, CandidateEntry> = HashMap::new();

        for (i, doc) in bm25_docs.into_iter().enumerate() {
            let path_str = doc.path.to_string_lossy().to_string();
            let is_active = doc.is_active();
            let superseded_label = doc.superseded_by();
            candidates.insert(
                path_str.clone(),
                CandidateEntry {
                    path: path_str,
                    title: doc.title,
                    body: doc.body,
                    hierarchy: doc.hierarchy,
                    frontmatter: doc.frontmatter,
                    mtime: doc.mtime,
                    bm25_rank: Some(i + 1),
                    vector_rank: None,
                    is_active,
                    superseded_label,
                    rrf_score: 0.0,
                },
            );
        }

        for (i, (path_str, _)) in vector_hits.into_iter().enumerate() {
            let rank = i + 1;
            if let Some(entry) = candidates.get_mut(&path_str) {
                entry.vector_rank = Some(rank);
            } else {
                let path = PathBuf::from(&path_str);
                let Some(doc) = self.storage.get_document(&path)? else {
                    continue;
                };
                let is_active = doc.is_active();
                let superseded_label = doc.superseded_by();
                candidates.insert(
                    path_str.clone(),
                    CandidateEntry {
                        path: path_str,
                        title: doc.title,
                        body: doc.body,
                        hierarchy: doc.hierarchy,
                        frontmatter: doc.frontmatter,
                        mtime: doc.mtime,
                        bm25_rank: None,
                        vector_rank: Some(rank),
                        is_active,
                        superseded_label,
                        rrf_score: 0.0,
                    },
                );
            }
        }

        // 4. Compute RRF base scores S_i and S_max over direct candidates
        let mut s_max = 0.0f32;
        for cand in candidates.values_mut() {
            let mut rrf_score = 0.0f32;
            if let Some(r_bm25) = cand.bm25_rank {
                rrf_score += (w_bm25 * bm25_tier_mult) / (self.rrf_k as f32 + r_bm25 as f32);
            }
            if let Some(r_vec) = cand.vector_rank {
                rrf_score += w_vec / (self.rrf_k as f32 + r_vec as f32);
            }
            cand.rrf_score = rrf_score;
            if rrf_score > s_max {
                s_max = rrf_score;
            }
        }

        // 5. Batch-load decay/access signals and graph edges
        let now = self.now_override.unwrap_or_else(current_unix_timestamp);
        let path_refs: Vec<&str> = candidates.keys().map(String::as_str).collect();
        let git_times = self.storage.git_times(&path_refs)?;
        let access_stats = self.storage.access_stats(&path_refs)?;

        let link_edges_clean = self.storage.link_edges_clean();
        let (path_to_id, touching_edges) = if link_edges_clean {
            let p2id = fetch_doc_ids_by_paths(self.storage, &path_refs)?;
            let pool_ids: Vec<i64> = p2id.values().copied().collect();
            let edges = self.storage.edges_touching(&pool_ids)?;
            (p2id, edges)
        } else {
            (HashMap::new(), Vec::new())
        };

        let active_pool_neighbors: HashSet<String> = if link_edges_clean {
            compute_active_pool_neighbors_from_edges(&candidates, &path_to_id, &touching_edges)
        } else {
            HashSet::new()
        };
        let pool_paths: HashSet<String> = candidates.keys().cloned().collect();

        let boost_amount = match self.graph_boost {
            GraphBoostMode::Relative(gamma) => gamma * s_max,
            GraphBoostMode::Absolute(abs) => abs,
        };

        let mut results = Vec::with_capacity(candidates.len());

        for cand in candidates.values() {
            let has_graph_connection = if !cand.is_active {
                false
            } else if link_edges_clean {
                active_pool_neighbors.contains(&cand.path)
            } else {
                has_legacy_active_pool_neighbor(self.storage, cand, &candidates, &pool_paths)
            };

            let graph_boost = if cand.is_active && has_graph_connection {
                boost_amount
            } else {
                0.0
            };

            let d_i = compute_decay(
                cand.hierarchy,
                &cand.frontmatter,
                &cand.path,
                git_times.get(&cand.path).copied(),
                cand.mtime,
                now,
                self.decay,
            );
            let r_i = compute_access_reinforcement(access_stats.get(&cand.path).copied(), now);
            let lifecycle_mult = if cand.is_active {
                1.0
            } else {
                Self::SUPERSEDED_PENALTY_MULTIPLIER
            };

            let final_score = (cand.rrf_score + graph_boost) * d_i * r_i * lifecycle_mult;
            let mut snippet = generate_snippet(&cand.body, query, 200);

            if !cand.is_active {
                snippet = apply_lifecycle_prefix(
                    self,
                    &cand.path,
                    cand.superseded_label.as_deref(),
                    snippet,
                );
            }

            results.push(SearchResult {
                path: cand.path.clone(),
                title: cand.title.clone(),
                score: final_score,
                bm25_rank: cand.bm25_rank,
                vector_rank: cand.vector_rank,
                graph_boost,
                snippet,
                section: None,
                expanded_from: None,
                scope: None,
            });
        }

        // 6. Sort direct candidates by descending F_i with deterministic tie-breaking
        results.sort_by(compare_search_results);

        // 7. 1-hop proportional graph expansion (Hybrid mode + clean link_edges only)
        if link_edges_clean {
            let expansions = self.expand_one_hop_neighbors(ExpansionContext {
                query,
                limit,
                now,
                sorted_direct: &results,
                candidates: &candidates,
                path_to_id: &path_to_id,
                touching_edges: &touching_edges,
            })?;
            if !expansions.is_empty() {
                results.extend(expansions);
                results.sort_by(compare_search_results);
            }
        }

        let inactive_paths: HashSet<String> = candidates
            .values()
            .filter(|c| !c.is_active)
            .map(|c| c.path.clone())
            .collect();

        results.truncate(limit);
        locate_sections_for_results(self, self.storage, query, &mut results, &vec_section_ranks)?;
        Ok(ScopeSearchResult {
            results,
            bm25_tier,
            inactive_paths,
        })
    }

    fn expand_one_hop_neighbors(
        &self,
        ctx: ExpansionContext<'_>,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let max_parents = (ctx.limit * 2).min(Self::MAX_EXPANSION_PARENTS);
        let mut parent_info: HashMap<i64, (f32, &str)> = HashMap::new();
        for res in ctx.sorted_direct {
            if parent_info.len() >= max_parents {
                break;
            }
            if !ctx.candidates.get(&res.path).is_some_and(|c| c.is_active) {
                continue;
            }
            if let Some(&doc_id) = ctx.path_to_id.get(&res.path) {
                parent_info.insert(doc_id, (res.score, res.path.as_str()));
            }
        }

        if parent_info.is_empty() {
            return Ok(Vec::new());
        }

        let pool_doc_ids: HashSet<i64> = ctx.path_to_id.values().copied().collect();
        let mut neighbor_edges: HashMap<i64, Vec<(EdgeDirection, f32, &str)>> = HashMap::new();

        for &(src, dst) in ctx.touching_edges {
            if let Some(&(f_p, p_path)) = parent_info.get(&src) {
                if !pool_doc_ids.contains(&dst) {
                    neighbor_edges.entry(dst).or_default().push((
                        EdgeDirection::Outgoing,
                        Self::DEFAULT_EXPANSION_OUT_BETA * f_p,
                        p_path,
                    ));
                }
            }
            if let Some(&(f_p, p_path)) = parent_info.get(&dst) {
                if !pool_doc_ids.contains(&src) {
                    neighbor_edges.entry(src).or_default().push((
                        EdgeDirection::Incoming,
                        Self::DEFAULT_EXPANSION_IN_L3_BETA * f_p,
                        p_path,
                    ));
                }
            }
        }

        if neighbor_edges.is_empty() {
            return Ok(Vec::new());
        }

        let neighbor_ids: Vec<i64> = neighbor_edges.keys().copied().collect();
        let neighbor_paths = fetch_paths_by_doc_ids(self.storage, &neighbor_ids)?;

        struct EligibleNeighbor<'p> {
            path: String,
            doc: Document,
            s_n: f32,
            parent_path: &'p str,
        }

        let mut eligible: Vec<EligibleNeighbor<'_>> = Vec::new();
        for (n_id, edges_for_n) in &neighbor_edges {
            let Some(n_path) = neighbor_paths.get(n_id) else {
                continue;
            };
            let Some(doc) = self.storage.get_document(Path::new(n_path))? else {
                continue;
            };
            if !doc.is_active() || doc.hierarchy == HierarchyLevel::L0Ephemeral {
                continue;
            }

            let mut best: Option<(f32, &str)> = None;
            for &(dir, cand_s_n, p_path) in edges_for_n {
                let dir_allowed = match dir {
                    EdgeDirection::Outgoing => true,
                    EdgeDirection::Incoming => doc.hierarchy == HierarchyLevel::L3Evergreen,
                };
                if !dir_allowed {
                    continue;
                }
                let replace = match best {
                    None => true,
                    Some((best_s, best_p)) => {
                        cand_s_n > best_s || (cand_s_n == best_s && p_path < best_p)
                    }
                };
                if replace {
                    best = Some((cand_s_n, p_path));
                }
            }

            if let Some((s_n, parent_path)) = best {
                eligible.push(EligibleNeighbor {
                    path: n_path.clone(),
                    doc,
                    s_n,
                    parent_path,
                });
            }
        }

        if eligible.is_empty() {
            return Ok(Vec::new());
        }

        let eligible_paths: Vec<&str> = eligible.iter().map(|e| e.path.as_str()).collect();
        let n_git_times = self.storage.git_times(&eligible_paths)?;
        let n_access_stats = self.storage.access_stats(&eligible_paths)?;

        let mut expansions = Vec::with_capacity(eligible.len());
        for item in eligible {
            let d_n = compute_decay(
                item.doc.hierarchy,
                &item.doc.frontmatter,
                &item.path,
                n_git_times.get(&item.path).copied(),
                item.doc.mtime,
                ctx.now,
                self.decay,
            );
            let r_n =
                compute_access_reinforcement(n_access_stats.get(&item.path).copied(), ctx.now);
            let f_n = item.s_n * d_n * r_n;
            let snippet = generate_snippet(&item.doc.body, ctx.query, 200);

            expansions.push(SearchResult {
                path: item.path,
                title: item.doc.title,
                score: f_n,
                bm25_rank: None,
                vector_rank: None,
                graph_boost: 0.0,
                snippet,
                section: None,
                expanded_from: Some(item.parent_path.to_string()),
                scope: None,
            });
        }

        expansions.sort_by(compare_search_results);
        expansions.truncate((ctx.limit / 2).max(1));
        Ok(expansions)
    }
}

struct ExpansionContext<'b> {
    query: &'b str,
    limit: usize,
    now: i64,
    sorted_direct: &'b [SearchResult],
    candidates: &'b HashMap<String, CandidateEntry>,
    path_to_id: &'b HashMap<String, i64>,
    touching_edges: &'b [(i64, i64)],
}

struct CandidateEntry {
    path: String,
    title: String,
    body: String,
    hierarchy: HierarchyLevel,
    frontmatter: serde_json::Value,
    mtime: u64,
    bm25_rank: Option<usize>,
    vector_rank: Option<usize>,
    is_active: bool,
    superseded_label: Option<String>,
    rrf_score: f32,
}

#[derive(Clone, Copy)]
enum EdgeDirection {
    Outgoing,
    Incoming,
}

fn compare_search_results(a: &SearchResult, b: &SearchResult) -> std::cmp::Ordering {
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
}

fn compute_active_pool_neighbors_from_edges(
    candidates: &HashMap<String, CandidateEntry>,
    path_to_id: &HashMap<String, i64>,
    touching_edges: &[(i64, i64)],
) -> HashSet<String> {
    let mut id_to_active_path: HashMap<i64, &str> = HashMap::with_capacity(path_to_id.len());
    for (path, &doc_id) in path_to_id {
        if candidates.get(path).is_some_and(|c| c.is_active) {
            id_to_active_path.insert(doc_id, path.as_str());
        }
    }

    let mut connected = HashSet::new();
    for &(src, dst) in touching_edges {
        if src == dst {
            continue;
        }
        if let (Some(&src_path), Some(&dst_path)) =
            (id_to_active_path.get(&src), id_to_active_path.get(&dst))
        {
            connected.insert(src_path.to_string());
            connected.insert(dst_path.to_string());
        }
    }
    connected
}

fn has_legacy_active_pool_neighbor(
    storage: &SqliteStorage,
    cand: &CandidateEntry,
    candidates: &HashMap<String, CandidateEntry>,
    pool_paths: &HashSet<String>,
) -> bool {
    if let Ok(outgoing) = storage.get_outgoing_links(Path::new(&cand.path)) {
        for link in outgoing {
            for other_path in pool_paths {
                if other_path != &cand.path {
                    if let Some(other_cand) = candidates.get(other_path) {
                        if other_cand.is_active
                            && target_matches_candidate(
                                &link.target,
                                &other_cand.path,
                                &other_cand.title,
                            )
                        {
                            return true;
                        }
                    }
                }
            }
        }
    }

    if let Ok(backlinks) = storage.get_backlinks(&cand.title) {
        for src in backlinks {
            let s = src.to_string_lossy();
            if s != cand.path.as_str() && candidates.get(s.as_ref()).is_some_and(|c| c.is_active) {
                return true;
            }
        }
    }

    if let Some(stem) = Path::new(&cand.path).file_stem().and_then(|s| s.to_str()) {
        if stem != cand.title {
            if let Ok(stem_backlinks) = storage.get_backlinks(stem) {
                for src in stem_backlinks {
                    let s = src.to_string_lossy();
                    if s != cand.path.as_str()
                        && candidates.get(s.as_ref()).is_some_and(|c| c.is_active)
                    {
                        return true;
                    }
                }
            }
        }
    }

    false
}

pub(crate) fn format_doc_lifecycle_snippet(
    engine: &HybridSearchEngine<'_>,
    path_str: &str,
    doc: &Document,
    snippet: String,
) -> String {
    if doc.is_active() {
        return snippet;
    }
    let label = doc.superseded_by();
    apply_lifecycle_prefix(engine, path_str, label.as_deref(), snippet)
}

fn apply_lifecycle_prefix(
    engine: &HybridSearchEngine<'_>,
    path_str: &str,
    superseded_label: Option<&str>,
    snippet: String,
) -> String {
    if snippet.starts_with("[SUPERSEDED") || snippet.starts_with("[DEPRECATED") {
        return snippet;
    }
    if let Some(target) = superseded_label {
        let final_target = engine.resolve_supersession_chain(path_str, target);
        if snippet.is_empty() {
            format!("[SUPERSEDED by {}]", final_target)
        } else {
            format!("[SUPERSEDED by {}] {}", final_target, snippet)
        }
    } else if snippet.is_empty() {
        "[DEPRECATED]".to_string()
    } else {
        format!("[DEPRECATED] {}", snippet)
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
    cand_clean.ends_with(&format!("/{}", clean_target))
}

/// Generates a relevant text snippet centered around matching query terms or start of text.
pub(crate) fn generate_snippet(body: &str, query: &str, max_len: usize) -> String {
    let trimmed_body = body.trim();
    if trimmed_body.is_empty() || max_len == 0 {
        return String::new();
    }

    let char_indices: Vec<(usize, char)> = trimmed_body.char_indices().collect();
    if char_indices.len() <= max_len {
        return trimmed_body.replace('\n', " ").trim().to_string();
    }

    // Build lowercased text while recording a monotonic mapping from lowercase byte
    // offsets back to original `char_indices` positions (handles chars like 'İ' or 'ẞ').
    let mut lower_body = String::with_capacity(trimmed_body.len());
    let mut lower_byte_map: Vec<(usize, usize)> = Vec::with_capacity(char_indices.len());
    for (orig_char_idx, &(_, ch)) in char_indices.iter().enumerate() {
        lower_byte_map.push((lower_body.len(), orig_char_idx));
        for lc in ch.to_lowercase() {
            lower_body.push(lc);
        }
    }

    let match_char_idx = find_best_snippet_match_char_idx(&lower_body, &lower_byte_map, query);

    if let Some(char_idx) = match_char_idx {
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
        let end_byte = char_indices[max_len].0;
        let mut snippet = trimmed_body[..end_byte]
            .replace('\n', " ")
            .trim()
            .to_string();
        snippet.push_str("...");
        snippet
    }
}

fn map_lower_byte_to_orig_char(lower_byte_map: &[(usize, usize)], lower_byte_pos: usize) -> usize {
    let idx = lower_byte_map.partition_point(|&(lb, _)| lb <= lower_byte_pos);
    if idx > 0 {
        lower_byte_map[idx - 1].1
    } else {
        0
    }
}

fn find_best_snippet_match_char_idx(
    lower_body: &str,
    lower_byte_map: &[(usize, usize)],
    query: &str,
) -> Option<usize> {
    let trimmed_query = query.trim();
    if trimmed_query.is_empty() {
        return None;
    }

    let lower_query = trimmed_query.to_lowercase();
    if let Some(lb) = lower_body.find(&lower_query) {
        return Some(map_lower_byte_to_orig_char(lower_byte_map, lb));
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut candidates: Vec<String> = Vec::new();
    let mut push_cand = |s: String| {
        if !s.is_empty() && seen.insert(s.clone()) {
            candidates.push(s);
        }
    };

    for term in trimmed_query.split_whitespace() {
        push_cand(term.to_lowercase());
    }

    let segmented = crate::text::segment_for_index(trimmed_query);
    for tok in segmented.split_whitespace() {
        push_cand(tok.to_lowercase());
    }

    // Add contiguous CJK sub-windows (down to bigrams) so multi-character CJK queries
    // fall back to the longest matching CJK sub-phrase or bigram.
    let mut cjk_run: Vec<char> = Vec::new();
    let flush_cjk_windows = |run: &mut Vec<char>, push: &mut dyn FnMut(String)| {
        let max_w = run.len().min(12);
        for w in (2..=max_w).rev() {
            for start in 0..=run.len() - w {
                let s: String = run[start..start + w].iter().collect();
                push(s);
            }
        }
        run.clear();
    };
    for ch in trimmed_query.chars() {
        if crate::text::is_cjk(ch) {
            cjk_run.push(ch);
        } else if !cjk_run.is_empty() {
            flush_cjk_windows(&mut cjk_run, &mut push_cand);
        }
    }
    if !cjk_run.is_empty() {
        flush_cjk_windows(&mut cjk_run, &mut push_cand);
    }

    // Prefer longest matching token/bigram first, preserving discovery order on ties.
    candidates.sort_by_key(|c| std::cmp::Reverse(c.chars().count()));

    for cand in candidates {
        if let Some(lb) = lower_body.find(&cand) {
            return Some(map_lower_byte_to_orig_char(lower_byte_map, lb));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{Document, HierarchyLevel, WikiLink};
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn test_superseded_note_penalized_and_stripped_of_graph_boost() {
        let mut storage = SqliteStorage::in_memory().expect("in-memory db");
        storage.initialize().expect("initialize storage");

        let adr_old = Document {
            path: PathBuf::from("ADR-001.md"),
            title: "ADR 001 Old Architecture".to_string(),
            hierarchy: HierarchyLevel::L3Evergreen,
            frontmatter: json!({
                "status": "superseded",
                "superseded_by": "ADR-002.md"
            }),
            links: Vec::new(),
            tags: vec!["architecture".to_string()],
            content_hash: "h1".to_string(),
            mtime: 100,
            body: "Architectural decision: we use REST architecture for distributed communication."
                .to_string(),
            body_line_offset: 0,
        };

        let adr_new = Document {
            path: PathBuf::from("ADR-002.md"),
            title: "ADR 002 New Architecture".to_string(),
            hierarchy: HierarchyLevel::L3Evergreen,
            frontmatter: json!({
                "status": "active",
                "supersedes": ["ADR-001.md"]
            }),
            links: Vec::new(),
            tags: vec!["architecture".to_string()],
            content_hash: "h2".to_string(),
            mtime: 200,
            body: "Architectural decision: we migrate REST architecture to high throughput gRPC."
                .to_string(),
            body_line_offset: 0,
        };

        let overview = Document {
            path: PathBuf::from("Overview.md"),
            title: "System Overview".to_string(),
            hierarchy: HierarchyLevel::L1Resource,
            frontmatter: json!({}),
            links: vec![WikiLink {
                target: "ADR 001 Old Architecture".to_string(),
                alias: None,
                raw_text: "[[ADR 001 Old Architecture]]".to_string(),
            }],
            tags: Vec::new(),
            content_hash: "h3".to_string(),
            mtime: 300,
            body: "Overview of services. See [[ADR 001 Old Architecture]] for REST architecture."
                .to_string(),
            body_line_offset: 0,
        };

        storage.upsert_document(&adr_old).expect("upsert adr_old");
        storage.upsert_document(&adr_new).expect("upsert adr_new");
        storage.upsert_document(&overview).expect("upsert overview");

        let engine = HybridSearchEngine::new(&storage, None);
        let results = engine
            .search("REST architecture", SearchMode::Hybrid, 10)
            .expect("search");

        assert!(results.len() >= 2);

        let old_res = results
            .iter()
            .find(|r| r.path == "ADR-001.md")
            .expect("old adr found");
        let new_res = results
            .iter()
            .find(|r| r.path == "ADR-002.md")
            .expect("new adr found");

        // Old superseded ADR must be stripped of graph boost despite backlink from Overview.md
        assert_eq!(old_res.graph_boost, 0.0);

        // Active ADR-002 must outrank superseded ADR-001
        assert!(new_res.score > old_res.score);

        // Superseded ADR snippet must be prefixed with [SUPERSEDED by ...]
        assert!(old_res.snippet.starts_with("[SUPERSEDED by ADR-002.md]"));
    }

    #[test]
    fn test_hybrid_vector_hit_outranks_any_tokens_lexical_noise() {
        use crate::scanner::VectorSyncEngine;
        use crate::vector::{EmbeddingEngine, VectorError};

        struct TopicEmbedder;
        impl EmbeddingEngine for TopicEmbedder {
            fn embed(&self, text: &str) -> Result<Vec<f32>, VectorError> {
                let mut v = vec![0.0f32; 16];
                let lower = text.to_lowercase();
                if lower.contains("oauth2") || lower.contains("authentication") {
                    v[0] = 1.0;
                } else {
                    v[1] = 1.0;
                }
                Ok(v)
            }

            fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, VectorError> {
                texts.iter().map(|t| self.embed(t)).collect()
            }

            fn dimension(&self) -> usize {
                16
            }

            fn model_name(&self) -> &str {
                "topic-test-16"
            }
        }

        let mut storage = SqliteStorage::in_memory().expect("in-memory db");
        storage.initialize().expect("initialize storage");

        // Doc 1: pure semantic match for "oauth2", contains zero query lexical tokens
        let semantic_doc = Document {
            path: PathBuf::from("semantic-auth.md"),
            title: "Semantic Authentication".to_string(),
            hierarchy: HierarchyLevel::L3Evergreen,
            frontmatter: json!({}),
            links: Vec::new(),
            tags: Vec::new(),
            content_hash: "h_sem".to_string(),
            mtime: 100,
            body: "Delegated authentication workflow and bearer token exchange.".to_string(),
            body_line_offset: 0,
        };
        // Doc 2: weak 1-word AnyTokens noise hit on unrelated topic ("HS256" only)
        let lexical_noise = Document {
            path: PathBuf::from("unrelated-noise.md"),
            title: "Unrelated Checksum Notes".to_string(),
            hierarchy: HierarchyLevel::L3Evergreen,
            frontmatter: json!({}),
            links: Vec::new(),
            tags: Vec::new(),
            content_hash: "h_lex".to_string(),
            mtime: 100,
            body: "Legacy checksum comparison mentioning HS256 briefly.".to_string(),
            body_line_offset: 0,
        };

        storage.upsert_document(&semantic_doc).unwrap();
        // Fill top-20 vector pool with topic-aligned docs so `unrelated-noise.md`
        // (orthogonal vector) falls outside vector KNN pool and relies solely on AnyTokens BM25.
        for idx in 0..20 {
            let filler = Document {
                path: PathBuf::from(format!("auth-guide-{idx:02}.md")),
                title: format!("Auth Guide {idx}"),
                hierarchy: HierarchyLevel::L3Evergreen,
                frontmatter: json!({}),
                links: Vec::new(),
                tags: Vec::new(),
                content_hash: format!("h_fill_{idx}"),
                mtime: 100,
                body: "Delegated authentication guide for service clients.".to_string(),
                body_line_offset: 0,
            };
            storage.upsert_document(&filler).unwrap();
        }
        storage.upsert_document(&lexical_noise).unwrap();
        let embedder: Arc<dyn EmbeddingEngine> = Arc::new(TopicEmbedder);
        VectorSyncEngine::new()
            .sync_storage(&mut storage, &*embedder)
            .unwrap();

        let engine = HybridSearchEngine::new(&storage, Some(embedder));
        let results = engine
            .search("oauth2 jwt PyJWT HS256", SearchMode::Hybrid, 5)
            .expect("hybrid search");

        assert_eq!(results.len(), 5);
        assert_eq!(results[0].vector_rank, Some(1));
        assert!(
            !results.iter().any(|r| r.path == "unrelated-noise.md"),
            "weak 1-word AnyTokens noise hit must be outranked by top-5 semantic vector hits"
        );
    }
}
