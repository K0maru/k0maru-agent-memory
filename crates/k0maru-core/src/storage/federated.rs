//! Federated search across a read-write primary vault and an optional read-only global vault.
//!
//! Ensures the global vault cache is only ever opened via `SQLITE_OPEN_READ_ONLY`,
//! gates vector execution per scope on `embed_model` provenance, deduplicates hits
//! by `content_hash` (primary wins), and fuses scope ranks via weighted RRF.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::params_from_iter;

use crate::storage::hybrid::{HybridSearchEngine, ScopeSearchResult, SearchMode, SearchResult};
pub use crate::storage::locate::vector_scope_enabled;
use crate::storage::SqliteStorage;
use crate::text::FtsQueryTier;
use crate::vector::EmbeddingEngine;

/// Locator prefix rendered on search hits originating from the global vault scope.
pub const GLOBAL_PREFIX: &str = "@global:";

/// Read-only wrapper around [`SqliteStorage`] for global vault access.
///
/// Guarantees that no mutable reference (`&mut SqliteStorage`) or write-capable
/// connection can be obtained from a global vault handle.
pub struct ReadOnlyStorage(SqliteStorage);

impl ReadOnlyStorage {
    /// Opens an existing SQLite cache database in strict read-only mode (`SQLITE_OPEN_READ_ONLY`).
    pub fn open(cache_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self(SqliteStorage::open_read_only(cache_path)?))
    }

    /// Returns an immutable reference to the underlying read-only [`SqliteStorage`].
    pub fn storage(&self) -> &SqliteStorage {
        &self.0
    }
}

/// Resolves the optional global vault root path from CLI flag and environment variables.
///
/// Precedence:
/// 1. `K0MARU_NO_GLOBAL=1` kill switch returns `None`.
/// 2. `flag` if provided, otherwise non-empty `K0MARU_GLOBAL_VAULT`.
/// 3. Canonicalizes candidate and `primary_vault` when possible; returns `None` if equal.
/// 4. Emits a stderr warning (without dropping) if one vault path contains the other.
/// 5. Requires `<global>/.k0maru/cache.sqlite` to exist as a file; never creates or syncs it.
pub fn resolve_global_vault(flag: Option<&Path>, primary_vault: &Path) -> Option<PathBuf> {
    if std::env::var("K0MARU_NO_GLOBAL").as_deref() == Ok("1") {
        return None;
    }

    let candidate = match flag {
        Some(p) => {
            if p.as_os_str().is_empty() {
                return None;
            }
            p.to_path_buf()
        }
        None => {
            let env_val = std::env::var_os("K0MARU_GLOBAL_VAULT")?;
            if env_val.is_empty() || env_val.to_str().is_some_and(|s| s.trim().is_empty()) {
                return None;
            }
            PathBuf::from(env_val)
        }
    };

    let canon_candidate = std::fs::canonicalize(&candidate).unwrap_or(candidate);
    let canon_primary =
        std::fs::canonicalize(primary_vault).unwrap_or_else(|_| primary_vault.to_path_buf());

    if canon_candidate == canon_primary {
        return None;
    }

    if canon_candidate.starts_with(&canon_primary) || canon_primary.starts_with(&canon_candidate) {
        eprintln!(
            "[k0maru] warning: global vault ({}) overlaps with primary vault ({}); overlapping notes will be deduplicated by content hash",
            canon_candidate.display(),
            canon_primary.display()
        );
    }

    let cache_path = canon_candidate.join(".k0maru").join("cache.sqlite");
    if !cache_path.is_file() {
        eprintln!(
            "[k0maru] warning: global vault cache not found at {}; skipping global scope (run `k0maru sync --vault {}`)",
            cache_path.display(),
            canon_candidate.display()
        );
        return None;
    }

    Some(canon_candidate)
}

/// Federated retrieval engine across a primary vault and an optional read-only global vault.
pub struct FederatedSearch<'a> {
    primary: &'a SqliteStorage,
    global: Option<&'a ReadOnlyStorage>,
    embedder: Option<Arc<dyn EmbeddingEngine>>,
    global_weight: f32,
    rrf_k: usize,
}

impl<'a> FederatedSearch<'a> {
    /// Default RRF weight applied to global vault ranks (`0.7`).
    pub const DEFAULT_GLOBAL_WEIGHT: f32 = 0.7;
    /// Default RRF smoothing constant (`60`).
    pub const DEFAULT_RRF_K: usize = HybridSearchEngine::DEFAULT_RRF_K;

    /// Creates a new [`FederatedSearch`] over `primary` and optional `global` storage.
    pub fn new(
        primary: &'a SqliteStorage,
        global: Option<&'a ReadOnlyStorage>,
        embedder: Option<Arc<dyn EmbeddingEngine>>,
    ) -> Self {
        Self {
            primary,
            global,
            embedder,
            global_weight: Self::DEFAULT_GLOBAL_WEIGHT,
            rrf_k: Self::DEFAULT_RRF_K,
        }
    }

    /// Sets the fusion weight multiplier applied to global scope RRF scores (default `0.7`).
    pub fn with_global_weight(mut self, w: f32) -> Self {
        self.global_weight = w;
        self
    }

    /// Sets the RRF smoothing constant `k` (default `60`).
    pub fn with_rrf_k(mut self, rrf_k: usize) -> Self {
        self.rrf_k = rrf_k;
        self
    }

    /// Executes federated retrieval across primary and (when configured) global scopes.
    pub fn search(
        &self,
        q: &str,
        mode: SearchMode,
        limit: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let Some(global_ro) = self.global else {
            return HybridSearchEngine::new(self.primary, self.embedder.clone())
                .with_rrf_k(self.rrf_k)
                .search(q, mode, limit);
        };

        if q.trim().is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let pool = (limit * 2).max(10);
        let primary_out = search_scope(
            self.primary,
            self.embedder.as_ref(),
            self.rrf_k,
            q,
            mode,
            pool,
        )?;
        let global_out = search_scope(
            global_ro.storage(),
            self.embedder.as_ref(),
            self.rrf_k,
            q,
            mode,
            pool,
        )?;

        let primary_paths: Vec<&str> = primary_out
            .results
            .iter()
            .map(|r| r.path.as_str())
            .collect();
        let primary_hashes: HashSet<String> =
            fetch_content_hashes_by_paths(self.primary, &primary_paths)?
                .into_values()
                .collect();

        let global_paths: Vec<&str> = global_out.results.iter().map(|r| r.path.as_str()).collect();
        let global_hashes = fetch_content_hashes_by_paths(global_ro.storage(), &global_paths)?;

        let k = self.rrf_k as f32;
        let mut fused = Vec::with_capacity(primary_out.results.len() + global_out.results.len());

        for (idx, mut hit) in primary_out.results.into_iter().enumerate() {
            let rank_p = (idx + 1) as f32;
            let quality_mult = scope_hit_quality_multiplier(
                &hit,
                primary_out.bm25_tier,
                &primary_out.inactive_paths,
            );
            hit.score = quality_mult / (k + rank_p);
            fused.push(hit);
        }

        for (idx, mut hit) in global_out.results.into_iter().enumerate() {
            if let Some(hash) = global_hashes.get(&hit.path) {
                if primary_hashes.contains(hash) {
                    continue;
                }
            }
            let rank_g = (idx + 1) as f32;
            let quality_mult = scope_hit_quality_multiplier(
                &hit,
                global_out.bm25_tier,
                &global_out.inactive_paths,
            );
            hit.score = (self.global_weight * quality_mult) / (k + rank_g);
            hit.scope = Some("global".to_string());
            fused.push(hit);
        }

        fused.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.scope.is_some().cmp(&b.scope.is_some()))
                .then_with(|| a.path.cmp(&b.path))
        });

        fused.truncate(limit);
        Ok(fused)
    }
}

fn scope_hit_quality_multiplier(
    hit: &SearchResult,
    bm25_tier: Option<FtsQueryTier>,
    inactive_paths: &HashSet<String>,
) -> f32 {
    let mut mult = 1.0f32;
    if bm25_tier == Some(FtsQueryTier::AnyTokens)
        && hit.bm25_rank.is_some()
        && hit.vector_rank.is_none()
    {
        mult *= HybridSearchEngine::ANY_TOKENS_LEXICAL_MULTIPLIER;
    }
    if inactive_paths.contains(&hit.path) {
        mult *= HybridSearchEngine::SUPERSEDED_PENALTY_MULTIPLIER;
    }
    mult
}

fn search_scope(
    storage: &SqliteStorage,
    embedder: Option<&Arc<dyn EmbeddingEngine>>,
    rrf_k: usize,
    query: &str,
    mode: SearchMode,
    pool: usize,
) -> Result<ScopeSearchResult, Box<dyn std::error::Error>> {
    let scoped_embedder = match embedder {
        Some(emb) => {
            if vector_scope_enabled(storage, emb.as_ref()) {
                Some(emb.clone())
            } else if mode == SearchMode::Vector {
                return Ok(ScopeSearchResult {
                    results: Vec::new(),
                    bm25_tier: None,
                    inactive_paths: HashSet::new(),
                });
            } else {
                None
            }
        }
        None => None,
    };

    HybridSearchEngine::new(storage, scoped_embedder)
        .with_rrf_k(rrf_k)
        .search_with_meta(query, mode, pool)
}

fn fetch_content_hashes_by_paths(
    storage: &SqliteStorage,
    paths: &[&str],
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    if paths.is_empty() {
        return Ok(HashMap::new());
    }

    let mut map = HashMap::with_capacity(paths.len());
    for chunk in paths.chunks(500) {
        let placeholders: Vec<String> = (1..=chunk.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "SELECT path, content_hash FROM documents WHERE path IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = storage.connection().prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(chunk.iter().copied()), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (path, hash) = row?;
            map.insert(path, hash);
        }
    }
    Ok(map)
}
