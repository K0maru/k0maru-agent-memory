//! Zero-daemon FastMCP stdio server implementing JSON-RPC 2.0.
//!
//! Exposes five lean agent memory tools:
//! 1. `get_project_loadout`: Generates sub-300-token context pack
//! 2. `recall_memory`: Fast FTS5 BM25 search over notes and memories
//! 3. `offload_context`: Truncates long command logs into Mermaid state diagram and disk reference
//! 4. `inspect_log_node`: Retrieves full raw log by node ID
//! 5. `flush_session`: Crystallizes memory notes into the target vault according to conventions

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};

use crate::adapters::open_vault_adapter;
use crate::core::models::HierarchyLevel;
use crate::core::traits::CacheStorage;
use crate::loadout::LoadoutBuilder;
use crate::offload::{inspect_node, OffloadEngine};
use crate::scanner::IncrementalScanner;
use crate::storage::hybrid::SearchMode;
use crate::storage::sqlite::LiveDbHealth;
use crate::storage::{
    resolve_global_vault, FederatedSearch, ReadOnlyStorage, SqliteStorage, CURRENT_SCHEMA_VERSION,
};
use crate::vector::{resolve_embedding_engine, EmbeddingEngine};
use k0maru_core::gc::{ContextGcEngine, GcOptions, SessionTrajectory};

/// Tool-surface profile exposed by [`McpServer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum McpProfile {
    /// Full 11-tool MCP surface (default).
    #[default]
    Full,
    /// Orthogonal 8-tool sidecar surface (omits offload_context, inspect_log_node, gc_session_context).
    Sidecar,
    /// Ultra-compact 5-tool surface (<2.2 KB tools/list) optimized for 7B-14B consumer models.
    Minimal,
}

impl std::str::FromStr for McpProfile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "full" => Ok(Self::Full),
            "sidecar" => Ok(Self::Sidecar),
            "minimal" => Ok(Self::Minimal),
            other => Err(format!(
                "Invalid MCP profile '{}'. Supported profiles: full, sidecar, minimal",
                other
            )),
        }
    }
}

enum StorageLease<'a> {
    Cached(&'a mut SqliteStorage),
}

impl std::ops::Deref for StorageLease<'_> {
    type Target = SqliteStorage;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Cached(s) => s,
        }
    }
}

impl std::ops::DerefMut for StorageLease<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Cached(s) => s,
        }
    }
}

/// FastMCP stdio server running on top of JSON-RPC 2.0.
pub struct McpServer {
    vault_path: PathBuf,
    refs_dir: PathBuf,
    storage: Option<SqliteStorage>,
    global_vault_flag: Option<PathBuf>,
    global_storage: Option<(PathBuf, ReadOnlyStorage)>,
    global_init_attempted: bool,
    embedder: Option<Arc<dyn EmbeddingEngine>>,
    embed_model_name: Option<String>,
    sidecar: bool,
    profile: McpProfile,
}

impl McpServer {
    /// Creates a new `McpServer` targeting the given vault path.
    pub fn new(vault_path: impl Into<PathBuf>) -> Self {
        Self::new_with_model(vault_path, None)
    }

    /// Creates a new `McpServer` with an explicit embedding model.
    ///
    /// `Some("mock")` resolves straight to the deterministic mock engine, so
    /// tests and offline runs never touch (or attempt to download) the default
    /// `all-minilm-l6-v2` weights. Other models are resolved lazily when a
    /// vector-enabled vault is queried.
    pub fn new_with_model(vault_path: impl Into<PathBuf>, embed_model: Option<&str>) -> Self {
        let vault_path = vault_path.into();
        let refs_dir = vault_path.join(".k0maru").join("refs");

        let embedder = if embed_model == Some("mock") {
            resolve_embedding_engine(Some("mock"), None).ok()
        } else {
            None
        };

        Self {
            vault_path,
            refs_dir,
            storage: None,
            global_vault_flag: None,
            global_storage: None,
            global_init_attempted: false,
            embedder,
            embed_model_name: embed_model.map(|s| s.to_string()),
            sidecar: false,
            profile: McpProfile::Full,
        }
    }

    /// Configures an optional read-only global vault path (lazily opened on first federated tool call).
    pub fn with_global_vault(mut self, global_vault: Option<PathBuf>) -> Self {
        self.global_vault_flag = global_vault;
        self.global_storage = None;
        self.global_init_attempted = false;
        self
    }

    fn ensure_global_storage(&mut self) -> Option<&(PathBuf, ReadOnlyStorage)> {
        if !self.global_init_attempted {
            self.global_init_attempted = true;
            if let Some(global_path) =
                resolve_global_vault(self.global_vault_flag.as_deref(), &self.vault_path)
            {
                if let Some(ro) = crate::open_global_storage_with_warning(&global_path) {
                    self.global_storage = Some((global_path, ro));
                }
            }
        }
        self.global_storage.as_ref()
    }

    /// Enables or disables orthogonal sidecar mode (exposes 8 core tools in tools/list).
    pub fn with_sidecar(mut self, sidecar: bool) -> Self {
        self.sidecar = sidecar;
        self.profile = if sidecar {
            McpProfile::Sidecar
        } else {
            McpProfile::Full
        };
        self
    }

    /// Configures the exposed tool-surface profile (`full`, `sidecar`, `minimal`).
    pub fn with_profile(mut self, profile: McpProfile) -> Self {
        self.profile = profile;
        self.sidecar = profile != McpProfile::Full;
        self
    }

    /// Returns whether sidecar mode (or minimal profile) is enabled.
    pub fn is_sidecar(&self) -> bool {
        self.sidecar
    }

    /// Returns the active MCP tool-surface profile.
    pub fn profile(&self) -> McpProfile {
        self.profile
    }

    /// Sets custom refs directory for offloaded logs.
    pub fn with_refs_dir(mut self, refs_dir: impl Into<PathBuf>) -> Self {
        self.refs_dir = refs_dir.into();
        self
    }

    /// Pre-injects a configured [`SqliteStorage`].
    pub fn with_storage(mut self, storage: SqliteStorage) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Sets custom embedding engine for hybrid recall.
    pub fn with_embedder(mut self, embedder: Arc<dyn EmbeddingEngine>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    /// Sets custom embedding engine for hybrid recall by model name or canonical identifier.
    pub fn with_embed_model(mut self, model_name: Option<&str>) -> Self {
        self.embed_model_name = model_name.map(|s| s.to_string());
        let model_cache = if self.vault_path.join(".k0maru").join("models").exists() {
            Some(self.vault_path.join(".k0maru").join("models"))
        } else {
            None
        };
        if let Ok(embedder) = resolve_embedding_engine(model_name, model_cache) {
            self.embedder = Some(embedder);
        }
        self
    }

    fn resolve_or_init_embedder(
        embedder: &mut Option<Arc<dyn EmbeddingEngine>>,
        vault_path: &Path,
        embed_model_name: Option<&str>,
    ) -> Option<Arc<dyn EmbeddingEngine>> {
        if embedder.is_none() {
            let model_cache = if vault_path.join(".k0maru").join("models").exists() {
                Some(vault_path.join(".k0maru").join("models"))
            } else {
                None
            };
            *embedder = resolve_embedding_engine(embed_model_name, model_cache).ok();
        }
        embedder.clone()
    }

    /// Returns `true` if a persistent (or explicitly injected) storage handle is cached.
    pub fn has_cached_storage(&self) -> bool {
        self.storage.is_some()
    }

    /// Returns the target vault root directory.
    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }

    /// Returns the active refs directory.
    pub fn refs_dir(&self) -> &Path {
        &self.refs_dir
    }

    fn schema_mismatch_message(version: Option<i32>) -> String {
        match version {
            Some(v) => format!(
                "SQLite cache schema mismatch or upgraded by a newer k0maru binary (v{} > supported v{}). Restart the MCP server or run k0maru sync --force.",
                v, CURRENT_SCHEMA_VERSION
            ),
            None => format!(
                "SQLite cache schema mismatch or upgraded by a newer k0maru binary (supported v{}). Restart the MCP server or run k0maru sync --force.",
                CURRENT_SCHEMA_VERSION
            ),
        }
    }

    fn format_open_cache_error(err: &dyn std::fmt::Display) -> String {
        let msg = err.to_string();
        if let Some(rest) = msg.split("schema version ").nth(1) {
            if let Some(ver_str) = rest.split_whitespace().next() {
                if let Ok(v) = ver_str.parse::<i32>() {
                    return Self::schema_mismatch_message(Some(v));
                }
            }
        }
        if msg.contains("schema version")
            || msg.contains("no such table")
            || msg.contains("malformed")
        {
            return Self::schema_mismatch_message(None);
        }
        format!(
            "Failed to open SQLite cache database: {}. Restart the MCP server or run k0maru sync --force.",
            msg
        )
    }

    fn acquire_storage_from<'a>(
        cached: &'a mut Option<SqliteStorage>,
        vault_path: &Path,
    ) -> Result<StorageLease<'a>, Box<dyn std::error::Error>> {
        let cache_dir = vault_path.join(".k0maru");
        let cache_path = cache_dir.join("cache.sqlite");

        if cached.is_some() {
            let health = cached.as_ref().unwrap().check_live_health(&cache_path);
            match health {
                Ok(LiveDbHealth::Healthy) => {
                    return Ok(StorageLease::Cached(cached.as_mut().unwrap()));
                }
                Ok(LiveDbHealth::ReplacedOnDisk) => {
                    // Transparently drop stale handle and reopen below (handles doctor --compact / sync --force)
                    *cached = None;
                }
                Ok(LiveDbHealth::SchemaTooNew(v)) => {
                    *cached = None;
                    return Err(Self::schema_mismatch_message(Some(v)).into());
                }
                Ok(LiveDbHealth::CorruptedSchema) => {
                    *cached = None;
                    return Err(Self::schema_mismatch_message(None).into());
                }
                Err(e) => {
                    *cached = None;
                    return Err(Self::format_open_cache_error(&e).into());
                }
            }
        }

        let first_err = match SqliteStorage::open_rw(&cache_path) {
            Ok(s) => {
                *cached = Some(s);
                return Ok(StorageLease::Cached(cached.as_mut().unwrap()));
            }
            Err(e) => e,
        };

        let first_str = first_err.to_string();
        if first_str.contains("Unsupported SQLite cache schema version") {
            return Err(Self::format_open_cache_error(&first_err).into());
        }

        // Brief single retry only for transient lock contention; never fall back to in_memory().
        std::thread::sleep(std::time::Duration::from_millis(150));
        match SqliteStorage::open_rw(&cache_path) {
            Ok(s) => {
                *cached = Some(s);
                Ok(StorageLease::Cached(cached.as_mut().unwrap()))
            }
            Err(retry_err) => Err(Self::format_open_cache_error(&retry_err).into()),
        }
    }

    fn sync_cache(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let vault_path = self.vault_path.clone();
        let embedder = self.embedder.clone();
        let mut storage_lease = Self::acquire_storage_from(&mut self.storage, &vault_path)?;
        Self::sync_cache_with_storage(&vault_path, embedder.as_ref(), &mut storage_lease)
    }

    fn sync_cache_with_storage(
        vault_path: &Path,
        embedder: Option<&Arc<dyn EmbeddingEngine>>,
        storage: &mut SqliteStorage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let had_vectors = storage.has_vectors().unwrap_or(false);
        let migrated = if let Some(emb) = embedder {
            storage
                .ensure_vector_dimension(emb.dimension())
                .unwrap_or(false)
        } else {
            false
        };

        let should_sync_vectors = had_vectors || migrated;
        let adapter = open_vault_adapter(vault_path);
        sync_adapter_and_vectors(&adapter, storage, embedder, should_sync_vectors)?;
        Ok(())
    }

    fn detect_explicit_default_project(vault_path: &Path) -> Option<String> {
        for dir_name in &["projects", "10_Projects", "1_Projects"] {
            let projects_dir = vault_path.join(dir_name);
            if projects_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&projects_dir) {
                    let mut stems: Vec<String> = entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                        .filter_map(|e| {
                            e.path()
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .map(|s| s.to_string())
                        })
                        .collect();
                    stems.sort();
                    if let Some(first) = stems.into_iter().next() {
                        return Some(first);
                    }
                }
            }
        }
        None
    }

    fn call_get_project_loadout(&mut self, arguments: &Value) -> Result<String, String> {
        let explicit_project = arguments
            .get("project_name")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());
        let diff_requested = arguments
            .get("diff")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let branch = arguments.get("branch").and_then(|v| v.as_str());
        let explicit_repo_path = arguments
            .get("repo_path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        let vault_path = self.vault_path.clone();
        self.ensure_global_storage();
        let embedder = self.embedder.clone();
        let global_ro = self.global_storage.as_ref().map(|(_, ro)| ro);
        let mut storage_lease = Self::acquire_storage_from(&mut self.storage, &vault_path)
            .map_err(|e| e.to_string())?;

        if let Err(e) =
            Self::sync_cache_with_storage(&vault_path, embedder.as_ref(), &mut storage_lease)
        {
            return Err(format!("Cache sync failed: {}", e));
        }

        let storage: &SqliteStorage = &storage_lease;
        let adapter = open_vault_adapter(&vault_path);
        let mut builder = LoadoutBuilder::new(&adapter, storage);
        if let Some(ref emb) = embedder {
            builder = builder.with_embedder(emb.clone());
        }
        if let Some(ro) = global_ro {
            builder = builder.with_global(ro);
        }

        let modified_files_opt = if diff_requested || branch.is_some() {
            // Prefer explicit repo_path, then vault's enclosing git repo, then cwd.
            let diff_root = if let Some(ref explicit_repo) = explicit_repo_path {
                crate::guard::resolve_repo_root(explicit_repo)
            } else {
                let vault_repo = crate::guard::resolve_repo_root(&vault_path);
                if vault_repo.join(".git").exists() {
                    vault_repo
                } else {
                    std::env::current_dir().unwrap_or_else(|_| vault_path.clone())
                }
            };
            Some(crate::git::GitInspector::inspect_diff(&diff_root, branch).unwrap_or_default())
        } else {
            None
        };

        if let Some(project_name) = explicit_project {
            let loadout_opt = if let Some(ref modified_files) = modified_files_opt {
                builder
                    .build_with_diff(project_name, modified_files)
                    .map_err(|e| e.to_string())?
            } else {
                builder.build(project_name).map_err(|e| e.to_string())?
            };

            match loadout_opt {
                Some(loadout) => Ok(loadout.markdown),
                None => {
                    let available = builder.list_available_topics();
                    if available.is_empty() {
                        Err(format!(
                            "Project matching '{}' not found in vault.",
                            project_name
                        ))
                    } else {
                        Err(format!(
                            "Project matching '{}' not found in vault. Available projects/topics: {}",
                            project_name,
                            available.join(", ")
                        ))
                    }
                }
            }
        } else {
            // Omitted project_name: try explicit project card, then repo-name match, then Vault Overview
            let candidate_name = Self::detect_explicit_default_project(&vault_path).or_else(|| {
                let repo_root = crate::guard::resolve_repo_root(&vault_path);
                repo_root
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            });

            if let Some(ref proj) = candidate_name {
                let loadout_opt = if let Some(ref modified_files) = modified_files_opt {
                    builder
                        .build_with_diff(proj, modified_files)
                        .map_err(|e| e.to_string())?
                } else {
                    builder.build(proj).map_err(|e| e.to_string())?
                };
                if let Some(loadout) = loadout_opt {
                    return Ok(loadout.markdown);
                }
            }

            let overview = builder
                .build_vault_overview(modified_files_opt.as_deref())
                .map_err(|e| e.to_string())?;
            Ok(overview.markdown)
        }
    }

    fn call_recall_memory(&mut self, arguments: &Value) -> Result<String, String> {
        let query = extract_str_with_aliases(arguments, &["query", "q", "search", "keyword"])
            .ok_or_else(|| "Missing required parameter 'query'".to_string())?;

        let limit = extract_i64_flexible(arguments, "limit")
            .map(|v| v.clamp(1, 50) as usize)
            .unwrap_or(5);

        let full = arguments
            .get("full")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mode = match arguments.get("mode").and_then(|v| v.as_str()) {
            Some(raw) => match raw.to_lowercase().as_str() {
                "hybrid" => SearchMode::Hybrid,
                "bm25" => SearchMode::Bm25,
                "vector" => SearchMode::Vector,
                other => {
                    return Err(format!(
                        "Invalid mode '{}'. Supported modes: hybrid, bm25, vector",
                        other
                    ))
                }
            },
            None => SearchMode::Hybrid,
        };

        let vault_path = self.vault_path.clone();
        self.ensure_global_storage();
        let global_ro = self.global_storage.as_ref().map(|(_, ro)| ro);
        let mut storage_lease = Self::acquire_storage_from(&mut self.storage, &vault_path)
            .map_err(|e| e.to_string())?;

        if let Err(e) =
            Self::sync_cache_with_storage(&vault_path, self.embedder.as_ref(), &mut storage_lease)
        {
            return Err(format!("Cache sync failed: {}", e));
        }

        let has_vectors = {
            let primary_vecs = storage_lease.vector_count().unwrap_or(0);
            let global_vecs = global_ro
                .and_then(|ro| ro.storage().vector_count().ok())
                .unwrap_or(0);
            if mode == SearchMode::Vector && primary_vecs == 0 && global_vecs == 0 {
                return Err("Vector index is empty. Run 'k0maru sync --vector' first.".to_string());
            }
            storage_lease.has_vectors().unwrap_or(false) || global_vecs > 0
        };

        let effective_embedder = if has_vectors && mode != SearchMode::Bm25 {
            Self::resolve_or_init_embedder(
                &mut self.embedder,
                &self.vault_path,
                self.embed_model_name.as_deref(),
            )
        } else {
            None
        };
        let storage: &SqliteStorage = &storage_lease;

        // Direct path lookup when query looks like a vault-relative markdown path
        let direct_path_hit = lookup_direct_path_hit(storage, &vault_path, query);

        let engine = FederatedSearch::new(storage, global_ro, effective_embedder);
        let mut results = engine
            .search(query, mode, limit)
            .map_err(|e| format!("Search failed: {}", e))?;

        if let Some(direct) = direct_path_hit {
            results.retain(|r| r.path != direct.path);
            results.insert(0, direct);
            results.truncate(limit);
        }

        if results.is_empty() {
            return Ok(format!("No memories found matching query: '{}'", query));
        }

        let scope_storage = |res: &k0maru_core::storage::SearchResult| -> &SqliteStorage {
            if res.scope.as_deref() == Some("global") {
                if let Some(ro) = global_ro {
                    return ro.storage();
                }
            }
            storage
        };

        if full {
            let mut formatted = Vec::new();
            for res in &results {
                let target_storage = scope_storage(res);
                let mut item = format!("## {} ({})\n", res.title, res.locator());
                item.push_str(&format!("Score: {:.4}\n", res.score));

                let backlinks = retrieve_backlinks(target_storage, &res.title, &res.path);
                if !backlinks.is_empty() {
                    let bl_strs: Vec<String> =
                        backlinks.iter().map(|s| format!("[[{}]]", s)).collect();
                    item.push_str(&format!("Backlinks: {}\n", bl_strs.join(", ")));
                }

                if let Ok(Some(doc)) = target_storage.get_document(Path::new(&res.path)) {
                    if !doc.tags.is_empty() {
                        item.push_str(&format!("Tags: #{}\n", doc.tags.join(" #")));
                    }
                    item.push_str(&format!("Hierarchy: {}\n", doc.hierarchy));
                }

                item.push_str(&format!("\n{}", res.snippet));
                formatted.push(item);
            }

            Ok(formatted.join("\n\n---\n\n"))
        } else {
            let locators: Vec<String> = results.iter().map(|r| r.locator()).collect();
            let mut hits = Vec::with_capacity(results.len());
            for (i, res) in results.iter().enumerate() {
                let target_storage = scope_storage(res);
                let in_links = retrieve_backlinks(target_storage, &res.title, &res.path).len();
                let (tier, out_links) = match target_storage.get_document(Path::new(&res.path)) {
                    Ok(Some(doc)) => (hierarchy_to_tier(doc.hierarchy), doc.links.len()),
                    _ => (
                        infer_tier_from_path(&res.path),
                        target_storage
                            .get_outgoing_links(Path::new(&res.path))
                            .map(|l| l.len())
                            .unwrap_or(0),
                    ),
                };
                hits.push(CompactRecallHit {
                    tier,
                    path: locators[i].as_str(),
                    score: res.score,
                    in_links,
                    out_links,
                    expanded_from: res.expanded_from.as_deref(),
                    raw_snippet: res.snippet.as_str(),
                });
            }
            Ok(format_compact_kdsl_hits(&hits))
        }
    }
    fn call_timeline_memory(&mut self, arguments: &Value) -> Result<String, String> {
        let anchor = arguments
            .get("anchor")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let window = arguments
            .get("window")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10);
        let project = arguments
            .get("project")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let query = crate::hook::TimelineQuery {
            anchor,
            window,
            project,
        };

        let entries = crate::hook::build_timeline(&self.vault_path, &query)
            .map_err(|e| format!("Failed to build timeline: {}", e))?;

        if entries.is_empty() {
            return Ok("No timeline entries found.".to_string());
        }

        let mut out = String::new();
        out.push_str("| Date | Kind | Title | Path | Status |\n");
        out.push_str("| --- | --- | --- | --- | --- |\n");
        for entry in &entries {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                entry.date, entry.kind, entry.title, entry.relative_path, entry.status
            ));
        }
        Ok(out)
    }

    fn call_inspect_note(&mut self, arguments: &Value) -> Result<String, String> {
        let path = extract_str_with_aliases(arguments, &["path", "locator", "file", "note"])
            .ok_or_else(|| "Missing required parameter 'path'".to_string())?;
        let full_body = arguments
            .get("full_body")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let is_global = crate::hook::parse_locator(path)
            .map(|p| p.is_global)
            .unwrap_or_else(|_| path.trim().starts_with(k0maru_core::storage::GLOBAL_PREFIX));

        if !is_global {
            let cache_path = self.vault_path.join(".k0maru").join("cache.sqlite");
            if self.storage.is_some() || cache_path.exists() {
                let vault_path = self.vault_path.clone();
                let _ = Self::acquire_storage_from(&mut self.storage, &vault_path)
                    .map_err(|e| e.to_string())?;
            }
        }

        let (report, scope_is_global) = if is_global {
            self.ensure_global_storage();
            let Some((ref global_root, ref ro)) = self.global_storage else {
                return Err(
                    "Global vault is not configured; cannot inspect '@global:...' locator"
                        .to_string(),
                );
            };
            let stripped = path
                .trim()
                .strip_prefix(k0maru_core::storage::GLOBAL_PREFIX)
                .unwrap_or(path.trim());
            let rep = crate::hook::inspect_note_detail_ro_with_options(
                global_root,
                stripped,
                Some(ro),
                full_body,
            )
            .map_err(|e| format!("Failed to inspect global note: {}", e))?;
            (rep, true)
        } else {
            let rep =
                crate::hook::inspect_note_detail_with_options(&self.vault_path, path, full_body)
                    .map_err(|e| format!("Failed to inspect note: {}", e))?;
            (rep, false)
        };

        let mut out = format!("# {}\n\n", report.title);
        if scope_is_global {
            out.push_str(&format!(
                "- **Path**: {}{}\n",
                k0maru_core::storage::GLOBAL_PREFIX,
                report.relative_path
            ));
            out.push_str("- **Scope**: global\n");
        } else {
            out.push_str(&format!("- **Path**: {}\n", report.relative_path));
        }
        out.push_str(&format!("- **Hierarchy**: {}\n", report.hierarchy));
        out.push_str(&format!("- **Status**: {}\n", report.status));
        if let Some(sb) = &report.superseded_by {
            out.push_str(&format!("- **Superseded By**: {}\n", sb));
        }
        if !report.supersedes.is_empty() {
            out.push_str(&format!(
                "- **Supersedes**: {}\n",
                report.supersedes.join(", ")
            ));
        }
        if !report.tags.is_empty() {
            out.push_str(&format!("- **Tags**: {}\n", report.tags.join(", ")));
        }
        if !report.incoming_links.is_empty() {
            out.push_str(&format!(
                "- **Incoming Links**: {}\n",
                report.incoming_links.join(", ")
            ));
        }
        if !report.outgoing_links.is_empty() {
            out.push_str(&format!(
                "- **Outgoing Links**: {}\n",
                report.outgoing_links.join(", ")
            ));
        }
        out.push_str("\n## Body\n\n");
        out.push_str(&report.body);
        Ok(out)
    }

    fn call_offload_context(&self, arguments: &Value) -> Result<String, String> {
        let raw_text = arguments
            .get("raw_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required parameter 'raw_text'".to_string())?;

        let task_name = arguments
            .get("task_name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let mut engine = OffloadEngine::new(&self.refs_dir).with_threshold(50);
        if let Some(task) = task_name {
            engine = engine.with_task_id(task);
        }

        let result = engine
            .offload_text(raw_text)
            .map_err(|e| format!("Offload failed: {}", e))?;

        if result.truncated {
            Ok(format!(
                "{}\n\n{}",
                result.mermaid_graph, result.summary_text
            ))
        } else {
            Ok(result.summary_text)
        }
    }

    fn call_inspect_log_node(&self, arguments: &Value) -> Result<String, String> {
        let node_id = arguments
            .get("node_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required parameter 'node_id'".to_string())?;

        inspect_node(&self.refs_dir, node_id)
            .map_err(|e| format!("Inspection failed for node '{}': {}", node_id, e))
    }

    fn call_flush_session(&mut self, arguments: &Value) -> Result<String, String> {
        let title = extract_str_with_aliases(arguments, &["title", "name", "heading"])
            .ok_or_else(|| "Missing required parameter 'title'".to_string())?;

        let content = extract_str_with_aliases(
            arguments,
            &["content", "body", "markdown", "text", "note", "summary"],
        )
        .ok_or_else(|| "Missing required parameter 'content'".to_string())?;

        let summary = arguments
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let category = arguments
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("log")
            .to_string();

        let tags: Vec<String> = arguments
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let related_notes: Vec<String> = arguments
            .get("related_notes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let supersedes: Vec<String> = match arguments.get("supersedes") {
            Some(Value::String(single)) => vec![single.to_string()],
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect(),
            _ => Vec::new(),
        };

        let engine = crate::convention::FlushEngine::new(&self.vault_path)
            .map_err(|e| format!("FlushEngine initialization failed: {}", e))?;

        let request = crate::convention::FlushRequest {
            title: title.to_string(),
            summary,
            content: content.to_string(),
            category,
            tags,
            related_notes: related_notes.clone(),
            dry_run: false,
            supersedes,
        };

        let result = engine
            .flush(request)
            .map_err(|e| format!("Flush failed: {}", e))?;

        let mut sync_warn_line = result
            .sync_warning
            .as_ref()
            .map(|w| format!("\n[warning] {}", w))
            .unwrap_or_default();

        if result.sync_warning.is_none() {
            if let Err(e) = self.sync_cache() {
                sync_warn_line = format!(
                    "\n[warning] note written to disk, but cache sync failed: {}",
                    e
                );
            }
        }

        let links_str = if related_notes.is_empty() {
            "None".to_string()
        } else {
            related_notes
                .iter()
                .map(|r| {
                    let t = r.trim();
                    if t.starts_with("[[") && t.ends_with("]]") {
                        t.to_string()
                    } else {
                        format!("[[{}]]", t)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        };

        let superseded_str = if result.superseded_files.is_empty() {
            String::new()
        } else {
            let list = result
                .superseded_files
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!("\nSuperseded: {}", list)
        };

        if !result.created {
            Ok(format!(
                "Note already up to date with identical content.\nPath: {}\nCategory: {}{}{}",
                result.relative_path.display(),
                result.category,
                superseded_str,
                sync_warn_line
            ))
        } else {
            Ok(format!(
                "✓ Crystallized note into vault.\nTitle: {}\nRelative Path: {}\nCategory: {}\nWikiLinks: {}{}{}",
                title,
                result.relative_path.display(),
                result.category,
                links_str,
                superseded_str,
                sync_warn_line
            ))
        }
    }

    fn call_distill_session_skill(&mut self, arguments: &Value) -> Result<String, String> {
        let raw_trace = arguments.get("raw_trace").and_then(|v| v.as_str());
        let node_id = arguments.get("node_id").and_then(|v| v.as_str());

        if raw_trace.is_none() && node_id.is_none() {
            return Err("Either 'raw_trace' or 'node_id' must be provided".to_string());
        }

        let title = arguments
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let context_hint = arguments
            .get("context_hint")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let category = arguments
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("skill")
            .to_string();

        let dry_run = arguments
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let tags: Vec<String> = arguments
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let related_notes: Vec<String> = arguments
            .get("related_notes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let target = arguments
            .get("target")
            .and_then(|v| v.as_str())
            .map(|s| {
                std::str::FromStr::from_str(s)
                    .map_err(|e: String| format!("Invalid target parameter: {}", e))
            })
            .transpose()?
            .unwrap_or(crate::distill::DistillTarget::Default);

        let engine = crate::distill::DistillEngine::new(&self.vault_path, &self.refs_dir);
        let opts = crate::distill::DistillOptions {
            title,
            context_hint,
            category: Some(category),
            tags,
            related_notes: related_notes.clone(),
            dry_run,
            target,
            home_override: None,
        };

        let result = if let Some(trace) = raw_trace {
            engine
                .distill_text(trace, opts)
                .map_err(|e| format!("Distill failed: {}", e))?
        } else if let Some(nid) = node_id {
            engine
                .distill_node(nid, opts)
                .map_err(|e| format!("Distill failed: {}", e))?
        } else {
            unreachable!();
        };

        if !dry_run {
            let _ = self.sync_cache();
        }

        let links_str = if related_notes.is_empty() {
            "None".to_string()
        } else {
            related_notes
                .iter()
                .map(|r| {
                    let t = r.trim();
                    if t.starts_with("[[") && t.ends_with("]]") {
                        t.to_string()
                    } else {
                        format!("[[{}]]", t)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        };

        let export_line = result
            .export_path
            .as_ref()
            .map(|ep| format!("Export Path: {}\n", ep.display()))
            .unwrap_or_default();

        if dry_run {
            Ok(format!(
                "[Dry Run] Generated skill preview:\n\nTitle: {}\nCategory: {}\nRelative Path: {}\nWikiLinks: {}\n{}\n---\n{}",
                result.skill.title,
                result.flush_result.category,
                result.flush_result.relative_path.display(),
                links_str,
                export_line,
                result.preview_markdown
            ))
        } else if !result.flush_result.created {
            Ok(format!(
                "Skill note already up to date with identical content.\nPath: {}\nCategory: {}\n{}",
                result.flush_result.relative_path.display(),
                result.flush_result.category,
                export_line
            ))
        } else {
            Ok(format!(
                "✓ Crystallized skill note into vault.\nTitle: {}\nRelative Path: {}\nCategory: {}\nWikiLinks: {}\n{}",
                result.skill.title,
                result.flush_result.relative_path.display(),
                result.flush_result.category,
                links_str,
                export_line
            ))
        }
    }

    fn call_gc_session_context(&mut self, arguments: &Value) -> Result<String, String> {
        let trajectory: SessionTrajectory =
            if let Some(raw_str) = arguments.get("trajectory_json").and_then(|v| v.as_str()) {
                serde_json::from_str(raw_str)
                    .map_err(|e| format!("Failed to parse 'trajectory_json' string: {}", e))?
            } else if let Some(obj) = arguments.get("trajectory_json") {
                serde_json::from_value(obj.clone())
                    .map_err(|e| format!("Failed to parse 'trajectory_json' value: {}", e))?
            } else if let Some(obj) = arguments.get("trajectory") {
                serde_json::from_value(obj.clone())
                    .map_err(|e| format!("Failed to parse 'trajectory' value: {}", e))?
            } else {
                return Err("Missing required argument: 'trajectory_json'".to_string());
            };

        let threshold = arguments
            .get("threshold")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(20);

        let hot_window = arguments
            .get("hot_window")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(5);

        let context_window = arguments
            .get("context_window")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(128_000);

        let options = GcOptions {
            line_threshold: threshold,
            hot_window_turns: hot_window,
            context_window_tokens: context_window,
            refs_dir: Some(self.vault_path.join(".k0maru").join("refs")),
            ..Default::default()
        };

        let engine = ContextGcEngine::new();
        let result = engine
            .compact(&trajectory, &options)
            .map_err(|e| format!("Context GC compaction failed: {}", e))?;

        serde_json::to_string_pretty(&result)
            .map_err(|e| format!("Failed to serialize GC result: {}", e))
    }

    fn call_guard_check(&mut self, arguments: &Value) -> Result<String, String> {
        let raw_patch = resolve_guard_raw_patch(arguments);
        let project_name = arguments.get("project_name").and_then(|v| v.as_str());
        let branch = arguments.get("branch").and_then(|v| v.as_str());
        let strict = arguments
            .get("strict")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let files: Vec<PathBuf> = arguments
            .get("files")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str())
                    .map(PathBuf::from)
                    .collect()
            })
            .unwrap_or_default();

        let repo_dir = match arguments.get("repo_path").and_then(|v| v.as_str()) {
            Some(explicit) => crate::guard::resolve_repo_root(Path::new(explicit)),
            None => {
                // Prefer the repository the agent is actually working in over
                // the vault mount, which may live in an unrelated repo.
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                let cwd_root = crate::guard::resolve_repo_root(&cwd);
                let vault_root = crate::guard::resolve_repo_root(&self.vault_path);
                if cwd_root.join(".git").exists() && cwd_root != vault_root {
                    cwd_root
                } else if vault_root.join(".git").exists() {
                    vault_root
                } else {
                    cwd_root
                }
            }
        };

        let mut rules = crate::guard::resolve_guard_rules_with_repo(
            &self.vault_path,
            project_name,
            Some(&repo_dir),
        )
        .map_err(|e| format!("Failed to resolve guard rules: {}", e))?;

        let mut seen: std::collections::HashSet<(String, String)> = rules
            .iter()
            .map(|rule| (rule.id.clone(), format!("{:?}", rule.condition)))
            .collect();

        // A repository may own a `.k0maru/wiki` vault even when this MCP server
        // was launched against a global vault. Its Living ADRs must still be
        // enforced, so resolve and merge them without duplicating rules.
        let repo_vault = repo_dir.join(".k0maru").join("wiki");
        let self_canonical = self.vault_path.canonicalize();
        if repo_vault.is_dir() {
            let distinct_vault = match (&self_canonical, repo_vault.canonicalize()) {
                (Ok(current), Ok(repo)) => current != &repo,
                (Err(_), Ok(_)) => true,
                _ => false,
            };
            if distinct_vault {
                if let Ok(repo_rules) = crate::guard::resolve_guard_rules_with_repo(
                    &repo_vault,
                    project_name,
                    Some(&repo_dir),
                ) {
                    for rule in repo_rules {
                        if seen.insert((rule.id.clone(), format!("{:?}", rule.condition))) {
                            rules.push(rule);
                        }
                    }
                }
            }
        }

        let mut global_origins = std::collections::HashSet::new();
        if let Some(global_path) =
            resolve_global_vault(self.global_vault_flag.as_deref(), &self.vault_path)
        {
            if let Ok(global_rules) =
                crate::guard::resolve_guard_rules_with_repo(&global_path, project_name, None)
            {
                for rule in global_rules {
                    if seen.insert((rule.id.clone(), format!("{:?}", rule.condition))) {
                        global_origins.insert(crate::guard::GlobalRuleOrigin::from_rule(&rule));
                        rules.push(rule);
                    }
                }
            }
        }

        let report = if !files.is_empty() {
            let mut file_tuples: Vec<(PathBuf, String)> = Vec::new();
            for file in &files {
                // MCP clients usually send repository-relative paths.
                let path = if file.is_absolute() {
                    file.clone()
                } else {
                    repo_dir.join(file)
                };
                if !path.exists() {
                    return Err(format!("Target file does not exist: '{}'", path.display()));
                }
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;
                file_tuples.push((path, content));
            }
            crate::guard::GuardEngine::check_files(&file_tuples, &rules)
        } else {
            let diff_text = if let Some(d) = raw_patch {
                let trimmed = d.trim();
                if !trimmed.is_empty()
                    && !trimmed.contains("diff --git ")
                    && !trimmed.contains("@@ ")
                    && !trimmed.contains("+++ ")
                {
                    return Err("Invalid unified diff: missing diff headers or hunks".to_string());
                }
                d.to_string()
            } else {
                crate::guard::get_git_diff(&repo_dir, branch)
                    .map_err(|e| format!("Failed to inspect git diff: {}", e))?
            };
            let root = repo_dir.clone();
            crate::guard::GuardEngine::check_diff_with_reader(
                &diff_text,
                &rules,
                crate::guard::repo_content_reader(&root),
            )
        };

        // A non-empty diff that matched no file headers is malformed (or was
        // mangled in transit): fail closed instead of reporting a clean pass.
        if files.is_empty() {
            if let Some(d) = raw_patch {
                if !d.trim().is_empty() && report.checked_files == 0 {
                    return Err(
                        "Invalid unified diff: no file headers (diff --git or +++ b/) matched"
                            .to_string(),
                    );
                }
            }
        }

        let as_json = arguments
            .get("json")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let formatted = crate::guard::format_guard_report_with_options(
            &report,
            &global_origins,
            Some(&self.vault_path),
            true,
        );
        let output = if as_json {
            let json_report = serde_json::to_string_pretty(&report)
                .map_err(|e| format!("Failed to serialize GuardReport: {}", e))?;
            format!("{}\n\n```json\n{}\n```", formatted, json_report)
        } else {
            formatted
        };

        if !report.is_passed || (strict && !report.violations.is_empty()) {
            return Err(output);
        }

        Ok(output)
    }

    fn call_handoff_session(&self, arguments: &Value) -> Result<String, String> {
        let action = arguments
            .get("action")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .ok_or_else(|| "Missing required parameter 'action'".to_string())?;
        let as_json = arguments
            .get("json")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let store = crate::handoff::HandoffStore::new(&self.vault_path);

        match action {
            "create" => {
                let from_agent = arguments
                    .get("from_agent")
                    .or_else(|| arguments.get("from"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required parameter 'from_agent'".to_string())?
                    .to_string();
                let title = arguments
                    .get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required parameter 'title'".to_string())?
                    .to_string();
                let state = match arguments.get("state").and_then(|v| v.as_str()) {
                    Some(raw) => raw
                        .parse::<crate::handoff::HandoffState>()
                        .map_err(|e| e.to_string())?,
                    None => crate::handoff::HandoffState::InProgress,
                };
                let expires_at = match (
                    arguments.get("expires_at").and_then(|v| v.as_str()),
                    arguments.get("expires_in").and_then(|v| v.as_str()),
                ) {
                    (Some(_), Some(_)) => {
                        return Err(
                            "Provide either 'expires_at' or 'expires_in', not both".to_string()
                        );
                    }
                    (Some(at), None) => Some(at.to_string()),
                    (None, Some(dur)) => {
                        let secs =
                            crate::handoff::parse_duration_secs(dur).map_err(|e| e.to_string())?;
                        let clamped = secs.min(i64::MAX as u64) as i64;
                        let dt = chrono::Utc::now() + chrono::TimeDelta::seconds(clamped);
                        Some(dt.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true))
                    }
                    (None, None) => None,
                };
                let files = arguments
                    .get("files")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|item| {
                                if let Some(s) = item.as_str() {
                                    Some(parse_mcp_file_ref(s))
                                } else {
                                    serde_json::from_value::<crate::handoff::HandoffFileRef>(
                                        item.clone(),
                                    )
                                    .ok()
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let req = crate::handoff::CreateHandoffRequest {
                    from_agent,
                    to_agent: arguments
                        .get("to_agent")
                        .or_else(|| arguments.get("to"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    project: arguments
                        .get("project")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    title,
                    goal: arguments
                        .get("goal")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    state,
                    done: extract_string_array_with_aliases(arguments, &["done"]),
                    next_steps: extract_string_array_with_aliases(
                        arguments,
                        &["next_steps", "next"],
                    ),
                    open_questions: extract_string_array_with_aliases(
                        arguments,
                        &["open_questions", "questions", "question"],
                    ),
                    refs: extract_string_array_with_aliases(arguments, &["refs", "ref"]),
                    files,
                    branch: arguments
                        .get("branch")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    commit: arguments
                        .get("commit")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    expires_at,
                    repo_path: arguments
                        .get("repo_path")
                        .and_then(|v| v.as_str())
                        .map(PathBuf::from),
                };

                let record = store.create(req).map_err(|e| e.to_string())?;
                if as_json {
                    serde_json::to_string_pretty(&record)
                        .map_err(|e| format!("Failed to serialize handoff record: {}", e))
                } else {
                    let entry = crate::handoff::HandoffEntry {
                        status: crate::handoff::HandoffStatus::Pending,
                        record,
                    };
                    Ok(entry.to_markdown())
                }
            }
            "claim" => {
                let agent = arguments
                    .get("agent")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required parameter 'agent'".to_string())?
                    .to_string();
                let req = crate::handoff::ClaimHandoffRequest {
                    agent,
                    id: arguments
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    project: arguments
                        .get("project")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    latest: arguments
                        .get("latest")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                };
                let record = store.claim(req).map_err(|e| e.to_string())?;
                if as_json {
                    serde_json::to_string_pretty(&record)
                        .map_err(|e| format!("Failed to serialize handoff record: {}", e))
                } else {
                    let entry = crate::handoff::HandoffEntry {
                        status: crate::handoff::HandoffStatus::Claimed,
                        record,
                    };
                    Ok(entry.to_markdown())
                }
            }
            "show" => {
                let id_or_latest = if arguments
                    .get("latest")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    Some("latest")
                } else {
                    arguments.get("id").and_then(|v| v.as_str())
                };
                let project = arguments.get("project").and_then(|v| v.as_str());
                let entry = store
                    .show(id_or_latest, project)
                    .map_err(|e| e.to_string())?;
                if as_json {
                    serde_json::to_string_pretty(&entry)
                        .map_err(|e| format!("Failed to serialize handoff entry: {}", e))
                } else {
                    Ok(entry.to_markdown())
                }
            }
            "list" => {
                let project = arguments.get("project").and_then(|v| v.as_str());
                let include_claimed = arguments
                    .get("all")
                    .or_else(|| arguments.get("include_claimed"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let entries = store
                    .list(project, include_claimed)
                    .map_err(|e| e.to_string())?;
                if as_json {
                    serde_json::to_string_pretty(&entries)
                        .map_err(|e| format!("Failed to serialize handoff list: {}", e))
                } else {
                    Ok(crate::handoff::format_handoff_list_markdown(&entries))
                }
            }
            other => Err(format!(
                "Invalid handoff_session action '{}'. Expected create, claim, show, or list.",
                other
            )),
        }
    }

    fn handle_initialize(&self, id: Value, params: Option<&Value>) -> Value {
        // Echo versions we actually speak; anything unknown falls back to the
        // original 2024-11-05 revision.
        let requested = params
            .and_then(|p| p.get("protocolVersion"))
            .and_then(|v| v.as_str())
            .unwrap_or("2024-11-05");
        let protocol_version = match requested {
            "2024-11-05" | "2025-03-26" | "2025-06-18" => requested,
            _ => "2024-11-05",
        };

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": protocol_version,
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "k0maru",
                    "version": crate::VERSION
                },
                "instructions": "K0maru-Agent-Memory Protocol: 1) Call get_project_loadout(diff=true) at session start. 2) Call recall_memory for local vault knowledge before searching external docs. 3) Call guard_check before committing code changes. 4) On a guard DENY, either fix the code per Remedy or, if replacing the architecture, call flush_session with category='decision', supersedes=['<source_note>'], and '## Non-Negotiable Invariants'. Stage updated vault notes alongside code changes."
            }
        })
    }

    fn handle_tools_list(&self, id: Value) -> Value {
        if self.profile == McpProfile::Minimal {
            let minimal_tools = json!([
                {
                    "name": "get_project_loadout",
                    "description": "Fetch project context (<300 tokens) or vault overview.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_name": { "type": "string", "description": "Optional project name" },
                            "diff": { "type": "boolean", "description": "Include active git diff ADRs" },
                            "branch": { "type": "string", "description": "Optional base branch" }
                        }
                    }
                },
                {
                    "name": "recall_memory",
                    "description": "Search local vault notes, ADRs, and logs.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Search query or note path" },
                            "limit": { "type": "integer", "description": "Max hits (default 5)" },
                            "full": { "type": "boolean", "description": "Return full markdown" }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "inspect_note",
                    "description": "Read note metadata and body by vault-relative path or locator.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Vault-relative path or locator" },
                            "full_body": { "type": "boolean", "description": "Return entire note body" }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": "flush_session",
                    "description": "Save a decision, log, card, project, or skill note.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "title": { "type": "string", "description": "Note title" },
                            "content": { "type": "string", "description": "Markdown body" },
                            "category": { "type": "string", "description": "decision|card|log|project|skill" },
                            "supersedes": { "type": "array", "items": { "type": "string" }, "description": "Superseded note paths or IDs" }
                        },
                        "required": ["title", "content"]
                    }
                },
                {
                    "name": "guard_check",
                    "description": "Audit git diff or files against active ADR invariants.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "repo_path": { "type": "string", "description": "Optional git repo path" },
                            "diff": { "type": "boolean", "description": "Audit uncommitted git diff (default true)" },
                            "patch": { "type": "string", "description": "Optional raw unified diff patch" },
                            "files": { "type": "array", "items": { "type": "string" }, "description": "Optional file paths" },
                            "branch": { "type": "string", "description": "Optional base branch" }
                        }
                    }
                }
            ]);

            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": minimal_tools
                }
            });
        }

        let mut tools = json!([
            {
                "name": "get_project_loadout",
                "description": "Fetch the high-priority architectural blueprint, core invariants, project vision, and recent ADR decision logs (<300 tokens) from the vault, or a vault overview when project_name is omitted. Supports change-aware diff inspection.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "project_name": {
                            "type": "string",
                            "description": "Optional target project name or keyword (e.g. 'k0maru-agent-memory', 'payment-gateway'); returns vault overview when omitted"
                        },
                        "diff": {
                            "type": "boolean",
                            "description": "Inspect Git changes in workspace and dynamically activate module-relevant ADRs (<300 tokens)"
                        },
                        "branch": {
                            "type": "string",
                            "description": "Optional base branch for comparison (e.g. 'main', 'dev')"
                        },
                        "repo_path": {
                            "type": "string",
                            "description": "Optional git repository path for diff inspection (defaults to the vault's repository)"
                        }
                    }
                }
            },
            {
                "name": "recall_memory",
                "description": "Search and retrieve documentation, technical notes, project context, architectural decisions, configurations, and logs from the local knowledge vault (Obsidian Vault / LLM-Wiki). Call this when looking up project decisions, past runs, or domain docs.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query keywords or topic name to search in the knowledge base (e.g. 'redis idempotency', 'oauth token rotation', 'rate limiter', 'postgres pool')"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results to return (default: 5, max: 50)"
                        },
                        "full": {
                            "type": "boolean",
                            "description": "Whether to return full verbose markdown documents. When false (default), returns compact single-line K-DSL wire format (!μ:<tier>|<path>|score:<score>|rank:#<n>|links:<in>/<out> § <snippet>) for high token efficiency"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["hybrid", "bm25", "vector"],
                            "description": "Search mode (default: hybrid). 'vector' requires 'k0maru sync --vector'."
                        }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "timeline_memory",
                "description": "Chronological progressive-disclosure timeline of engineering logs, session observations, and ADR milestones around an optional anchor date/note.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "anchor": {
                            "type": "string",
                            "description": "Optional anchor date (YYYY-MM-DD), note title, or relative path to center the timeline window around"
                        },
                        "window": {
                            "type": "integer",
                            "description": "Number of timeline entries to return (default: 10)"
                        },
                        "project": {
                            "type": "string",
                            "description": "Optional project slug/filter"
                        }
                    }
                }
            },
            {
                "name": "inspect_note",
                "description": "Safely inspect note metadata, frontmatter, incoming/outgoing WikiLinks, supersession status, and markdown body (or exact line range from a locator) by relative path or locator.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path, stem, or locator to the markdown note within the vault (e.g. 'decisions/adr-001.md' or 'decisions/adr-001.md:10-25#Section')"
                        },
                        "full_body": {
                            "type": "boolean",
                            "description": "If true, return the full note body even when the locator includes a line range (default: false)"
                        }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "offload_context",
                "description": "Externalize and compress verbose terminal logs, build outputs, test traces, or error dumps (>30 lines) into a compact symbolic Mermaid state diagram on disk to prevent context window pollution.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "raw_text": {
                            "type": "string",
                            "description": "Raw log or output text to be offloaded"
                        },
                        "task_name": {
                            "type": "string",
                            "description": "Optional task identifier prefix"
                        }
                    },
                    "required": ["raw_text"]
                }
            },
            {
                "name": "inspect_log_node",
                "description": "Retrieve and inspect the full offloaded raw terminal log or compiler trace content by its node ID (e.g. node_1a2b3c4d).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node_id": {
                            "type": "string",
                            "description": "Offloaded log node identifier (e.g. node_1a2b3c4d)"
                        }
                    },
                    "required": ["node_id"]
                }
            },
            {
                "name": "flush_session",
                "description": "Crystallize technical decisions, verified bug fixes, architecture decisions (ADRs), or task milestones back into the local Markdown vault as permanent notes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Title of the decision or log note"
                        },
                        "content": {
                            "type": "string",
                            "description": "Detailed Markdown content"
                        },
                        "summary": {
                            "type": "string",
                            "description": "Brief one-line summary or executive conclusion"
                        },
                        "category": {
                            "type": "string",
                            "description": "Category: 'decision', 'card', 'concept', 'log', 'project', or 'skill' (default: 'log'). A 'card' whose content declares '## Non-Negotiable Invariants' is stored as a decision."
                        },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Associated tags"
                        },
                        "related_notes": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Related existing notes to link via WikiLinks"
                        },
                        "supersedes": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Existing note(s) or ADR(s) that this note supersedes (automatically marks old notes as status: superseded)."
                        }
                    },
                    "required": ["title", "content"]
                }
            },
            {
                "name": "distill_session_skill",
                "description": "Distill an execution trace, compilation error, or debugging session into a reusable troubleshooting playbook/skill note and save it to the vault or export to Hermes Agent.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "raw_trace": {
                            "type": "string",
                            "description": "Raw execution log, error stack trace, or debugging terminal output"
                        },
                        "node_id": {
                            "type": "string",
                            "description": "Optional offloaded node ID from offload_context (e.g. node_1a2b3c4d)"
                        },
                        "title": {
                            "type": "string",
                            "description": "Optional explicit skill title (auto-inferred if omitted)"
                        },
                        "context_hint": {
                            "type": "string",
                            "description": "Optional context hint or explanation describing what was happening"
                        },
                        "category": {
                            "type": "string",
                            "description": "Target category (defaults to 'skill')"
                        },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional tags array (e.g. ['topic/rust', 'topic/build'])"
                        },
                        "related_notes": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional related notes or WikiLinks array"
                        },
                        "dry_run": {
                            "type": "boolean",
                            "description": "If true, simulates distillation and returns preview without writing to vault"
                        },
                        "target": {
                            "type": "string",
                            "enum": ["default", "hermes", "omp"],
                            "description": "Optional target ecosystem schema: 'default' (Obsidian / Markdown Vault), 'hermes' (Nous Research Hermes dynamic skill), or 'omp' (Oh My Pi native skill://<name>)"
                        }
                    }
                }
            },
            {
                "name": "gc_session_context",
                "description": "Perform long-trajectory context garbage collection (Context GC) on a multi-turn agent session trajectory, folding verbose tool outputs and repetitive compiler error dumps into symbolic references to keep context within the cognitive Smart-Zone (15%-60%) with TRR >= 85%.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "trajectory_json": {
                            "type": "string",
                            "description": "JSON serialized SessionTrajectory containing session_id and turns array (or direct JSON object)"
                        },
                        "threshold": {
                            "type": "integer",
                            "description": "Line threshold before tool output is folded (default: 20)"
                        },
                        "hot_window": {
                            "type": "integer",
                            "description": "Number of recent turns preserved untouched in hot window (default: 5)"
                        },
                        "context_window": {
                            "type": "integer",
                            "description": "Context window size in tokens for Smart-Zone evaluation (default: 128000)"
                        }
                    },
                    "required": ["trajectory_json"]
                }
            },
            {
                "name": "guard_check",
                "description": "Audit git diff or code changes against active Living ADR architectural invariants and project rules (<10ms). Invoke before committing or after implementing changes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "diff": {
                            "type": "string",
                            "description": "Optional git unified diff text to audit. If omitted, automatically inspects working tree changes in current git repository."
                        },
                        "patch": {
                            "type": "string",
                            "description": "Optional raw unified diff patch text to audit."
                        },
                        "project_name": {
                            "type": "string",
                            "description": "Optional target project name to filter ADR invariants for."
                        },
                        "repo_path": {
                            "type": "string",
                            "description": "Optional path to the git repository to audit (defaults to the vault's repository or the current directory)."
                        },
                        "branch": {
                            "type": "string",
                            "description": "Optional base branch/revision for comparison (e.g. 'main', 'dev')."
                        },
                        "strict": {
                            "type": "boolean",
                            "description": "Treat advisory warnings as blocking failures."
                        },
                        "files": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional explicit file paths to audit instead of a diff."
                        },
                        "json": {
                            "type": "boolean",
                            "description": "Append structured JSON GuardReport in addition to the text report (default: false)."
                        }
                    }
                }
            },
            {
                "name": "handoff_session",
                "description": "Create, atomically claim, show, or list typed session handoff records stored in the primary vault (.k0maru/handoffs/).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["create", "claim", "show", "list"],
                            "description": "Handoff action to execute: create, claim, show, or list"
                        },
                        "from_agent": {
                            "type": "string",
                            "description": "Originating agent name (required for create; alias: 'from')"
                        },
                        "to_agent": {
                            "type": "string",
                            "description": "Optional target agent name (alias: 'to')"
                        },
                        "agent": {
                            "type": "string",
                            "description": "Claiming agent name (required for claim)"
                        },
                        "id": {
                            "type": "string",
                            "description": "Specific handoff record ID (for claim or show)"
                        },
                        "latest": {
                            "type": "boolean",
                            "description": "Target the newest matching handoff (for claim or show)"
                        },
                        "project": {
                            "type": "string",
                            "description": "Optional project filter or tag"
                        },
                        "title": {
                            "type": "string",
                            "description": "Handoff title (required for create)"
                        },
                        "goal": {
                            "type": "string",
                            "description": "Goal description (for create)"
                        },
                        "state": {
                            "type": "string",
                            "enum": ["in_progress", "blocked", "done"],
                            "description": "Workflow state (default: in_progress)"
                        },
                        "done": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Completed items (for create)"
                        },
                        "next_steps": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Recommended next steps (for create; alias: 'next')"
                        },
                        "open_questions": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Open questions or blockers (for create; alias: 'question')"
                        },
                        "refs": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Vault-relative note locators (for create; alias: 'ref')"
                        },
                        "files": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string" },
                                    "lines": { "type": "string" }
                                },
                                "required": ["path"]
                            },
                            "description": "Referenced relative file paths with optional line ranges"
                        },
                        "branch": {
                            "type": "string",
                            "description": "Optional git branch override (for create)"
                        },
                        "commit": {
                            "type": "string",
                            "description": "Optional git commit override (for create)"
                        },
                        "expires_at": {
                            "type": "string",
                            "description": "Optional RFC3339 UTC expiration timestamp (for create)"
                        },
                        "expires_in": {
                            "type": "string",
                            "description": "Optional relative expiration duration such as '24h' or '7d' (for create)"
                        },
                        "all": {
                            "type": "boolean",
                            "description": "Include claimed and expired records when action='list'"
                        },
                        "json": {
                            "type": "boolean",
                            "description": "Return raw JSON instead of formatted Markdown"
                        }
                    },
                    "required": ["action"]
                }
            }
        ]);

        if self.sidecar {
            if let Some(tool_list) = tools.as_array_mut() {
                tool_list.retain(|tool| {
                    !matches!(
                        tool["name"].as_str().unwrap_or(""),
                        "offload_context" | "inspect_log_node" | "gc_session_context"
                    )
                });
            }
        }

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": tools
            }
        })
    }

    /// Dispatches and executes an MCP tool call given request ID and tool parameters.
    pub fn handle_tool_call(&mut self, id: Value, params: &Value) -> Value {
        let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let empty_args = Value::Object(serde_json::Map::new());
        let arguments = params.get("arguments").unwrap_or(&empty_args);

        let (content_text, is_error) = match tool_name {
            "get_project_loadout" => match self.call_get_project_loadout(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "recall_memory" => match self.call_recall_memory(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "timeline_memory" => match self.call_timeline_memory(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "inspect_note" => match self.call_inspect_note(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "offload_context" => match self.call_offload_context(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "inspect_log_node" => match self.call_inspect_log_node(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "flush_session" => match self.call_flush_session(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "distill_session_skill" => match self.call_distill_session_skill(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "gc_session_context" => match self.call_gc_session_context(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "guard_check" => match self.call_guard_check(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            "handoff_session" => match self.call_handoff_session(arguments) {
                Ok(text) => (text, false),
                Err(e) => (e, true),
            },
            unknown => {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": format!("Unknown tool: '{}'", unknown)
                    }
                });
            }
        };

        let mut result_obj = serde_json::Map::new();
        result_obj.insert(
            "content".to_string(),
            json!([
                {
                    "type": "text",
                    "text": content_text
                }
            ]),
        );
        if is_error {
            result_obj.insert("isError".to_string(), Value::Bool(true));
        }

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": Value::Object(result_obj)
        })
    }

    /// Alias for [`Self::handle_tool_call`].
    pub fn handle_call_tool(&mut self, id: Value, params: &Value) -> Value {
        self.handle_tool_call(id, params)
    }

    /// Handles a single incoming JSON-RPC 2.0 request string.
    pub fn handle_request(&mut self, request: &str) -> Option<String> {
        self.handle_line(request)
    }

    /// Handles a single incoming JSON-RPC 2.0 request or notification string.
    ///
    /// Returns `Some(response_json)` for requests, or `None` for notifications (silent acknowledgment).
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let parsed: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    }
                });
                return Some(err_resp.to_string());
            }
        };

        let obj = match parsed.as_object() {
            Some(o) => o,
            None => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {
                        "code": -32600,
                        "message": "Invalid Request"
                    }
                });
                return Some(err_resp.to_string());
            }
        };

        let id = obj.get("id").cloned();
        let is_notification = !obj.contains_key("id");

        let method = match obj.get("method").and_then(|m| m.as_str()) {
            Some(m) => m,
            None => {
                if is_notification {
                    return None;
                }
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": id.unwrap_or(Value::Null),
                    "error": {
                        "code": -32600,
                        "message": "Invalid Request"
                    }
                });
                return Some(err_resp.to_string());
            }
        };

        let params = obj.get("params").unwrap_or(&Value::Null);

        match method {
            "initialize" => {
                let params_opt = obj.get("params");
                let resp = self.handle_initialize(id.unwrap_or(Value::Null), params_opt);
                Some(resp.to_string())
            }
            "notifications/initialized" | "initialized" => {
                // Silent acknowledgment according to MCP specification
                None
            }
            "tools/list" => {
                let resp = self.handle_tools_list(id.unwrap_or(Value::Null));
                Some(resp.to_string())
            }
            "tools/call" => {
                let resp = self.handle_tool_call(id.unwrap_or(Value::Null), params);
                Some(resp.to_string())
            }
            "ping" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id.unwrap_or(Value::Null),
                    "result": {}
                });
                Some(resp.to_string())
            }
            _ => {
                if is_notification {
                    None
                } else {
                    let err_resp = json!({
                        "jsonrpc": "2.0",
                        "id": id.unwrap_or(Value::Null),
                        "error": {
                            "code": -32601,
                            "message": "Method not found"
                        }
                    });
                    Some(err_resp.to_string())
                }
            }
        }
    }

    /// Runs the stdio JSON-RPC loop reading lines from `reader` and writing responses to `writer`.
    pub fn run_stdio<R: BufRead, W: Write>(
        &mut self,
        mut reader: R,
        mut writer: W,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        if let Some(resp) = self.handle_line(trimmed) {
                            if let Err(e) = writer.write_all(resp.as_bytes()) {
                                if e.kind() == std::io::ErrorKind::BrokenPipe {
                                    break;
                                }
                                return Err(Box::new(e));
                            }
                            if let Err(e) = writer.write_all(b"\n") {
                                if e.kind() == std::io::ErrorKind::BrokenPipe {
                                    break;
                                }
                                return Err(Box::new(e));
                            }
                            if let Err(e) = writer.flush() {
                                if e.kind() == std::io::ErrorKind::BrokenPipe {
                                    break;
                                }
                                return Err(Box::new(e));
                            }
                        }
                    }
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::BrokenPipe {
                        break;
                    }
                    return Err(Box::new(e));
                }
            }
        }
        Ok(())
    }
}

fn extract_str_with_aliases<'a>(args: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        args.get(*key)
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
    })
}

fn extract_i64_flexible(args: &Value, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_u64().and_then(|u| i64::try_from(u).ok()))
            .or_else(|| v.as_str().and_then(|s| s.trim().parse::<i64>().ok()))
    })
}

/// Resolves an explicit raw unified diff patch from `patch` or `diff`,
/// treating booleans, nulls, and boolean/mode tokens in `diff` as workspace
/// git-diff mode (`None`).
fn resolve_guard_raw_patch(arguments: &Value) -> Option<&str> {
    if let Some(patch) = arguments
        .get("patch")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
    {
        return Some(patch);
    }

    match arguments.get("diff") {
        Some(Value::String(raw)) => {
            let trimmed = raw.trim();
            let is_mode_token = matches!(
                trimmed.to_ascii_lowercase().as_str(),
                "" | "true" | "false" | "1" | "0" | "yes" | "auto" | "head" | "staged"
            );
            if is_mode_token {
                None
            } else {
                Some(raw.as_str())
            }
        }
        Some(Value::Bool(_)) | Some(Value::Null) | None | Some(_) => None,
    }
}

fn extract_string_array_with_aliases(args: &Value, keys: &[&str]) -> Vec<String> {
    for key in keys {
        if let Some(val) = args.get(*key) {
            if let Some(arr) = val.as_array() {
                return arr
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect();
            }
            if let Some(single) = val.as_str() {
                if !single.trim().is_empty() {
                    return vec![single.to_string()];
                }
            }
        }
    }
    Vec::new()
}

fn lookup_direct_path_hit(
    storage: &SqliteStorage,
    vault_path: &Path,
    query: &str,
) -> Option<k0maru_core::storage::SearchResult> {
    let trimmed = query.trim().trim_matches(['`', '"', '\'']);
    if trimmed.is_empty() || trimmed.contains(' ') {
        return None;
    }
    let clean = trimmed.replace('\\', "/");
    let clean = clean.strip_prefix("./").unwrap_or(&clean);
    let mut candidates = vec![clean.to_string()];
    if let Some(vault_name) = vault_path.file_name().and_then(|s| s.to_str()) {
        if let Some(stripped) = clean
            .strip_prefix(vault_name)
            .and_then(|s| s.strip_prefix('/'))
        {
            candidates.push(stripped.to_string());
        }
    }
    for cand in candidates {
        let with_ext = if cand.ends_with(".md") || cand.ends_with(".MD") {
            cand
        } else if cand.contains('/') {
            format!("{}.md", cand)
        } else {
            continue;
        };
        if let Ok(Some(doc)) = storage.get_document(Path::new(&with_ext)) {
            let snippet: String = doc.body.chars().take(220).collect();
            return Some(k0maru_core::storage::SearchResult {
                path: doc.path.to_string_lossy().replace('\\', "/"),
                title: doc.title,
                snippet,
                score: 1.0,
                bm25_rank: Some(1),
                vector_rank: None,
                graph_boost: 0.0,
                section: None,
                expanded_from: None,
                scope: None,
            });
        }
    }
    None
}

fn parse_mcp_file_ref(raw: &str) -> crate::handoff::HandoffFileRef {
    let trimmed = raw.trim();
    if let Some((path_part, suffix)) = trimmed.rsplit_once(':') {
        if !path_part.is_empty()
            && !suffix.is_empty()
            && suffix
                .bytes()
                .all(|b| b.is_ascii_digit() || b == b'-' || b == b',')
            && suffix.bytes().any(|b| b.is_ascii_digit())
        {
            return crate::handoff::HandoffFileRef {
                path: path_part.to_string(),
                lines: Some(suffix.to_string()),
            };
        }
    }
    crate::handoff::HandoffFileRef {
        path: trimmed.to_string(),
        lines: None,
    }
}

/// Helper function to retrieve all backlinks matching document title, file stem, or whitespace-normalized variants.
fn sync_adapter_and_vectors<A: crate::VaultAdapter>(
    adapter: &A,
    storage: &mut SqliteStorage,
    embedder: Option<&Arc<dyn EmbeddingEngine>>,
    should_sync_vectors: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scanner = IncrementalScanner::new(adapter, storage);
    let _ = scanner.sync(false)?;
    if should_sync_vectors {
        if let Some(emb) = embedder {
            let _ = crate::scanner::VectorSyncEngine::sync(scanner.storage_mut(), &**emb);
            let _ = crate::scanner::VectorSyncEngine::new().sync_sections(
                scanner.storage_mut(),
                &**emb,
                Some(64),
            );
        }
    }
    Ok(())
}

fn retrieve_backlinks(storage: &SqliteStorage, title: &str, path_str: &str) -> Vec<String> {
    let mut all = Vec::new();
    if let Ok(bls) = storage.get_backlinks(title) {
        for b in bls {
            let s = b.to_string_lossy().to_string();
            if !all.contains(&s) {
                all.push(s);
            }
        }
    }
    if let Some(stem) = Path::new(path_str).file_stem().and_then(|s| s.to_str()) {
        if stem != title {
            if let Ok(bls) = storage.get_backlinks(stem) {
                for b in bls {
                    let s = b.to_string_lossy().to_string();
                    if !all.contains(&s) {
                        all.push(s);
                    }
                }
            }
        }
    }
    let under = title.replace(' ', "_");
    if under != title {
        if let Ok(bls) = storage.get_backlinks(&under) {
            for b in bls {
                let s = b.to_string_lossy().to_string();
                if !all.contains(&s) {
                    all.push(s);
                }
            }
        }
    }
    let space = title.replace('_', " ");
    if space != title {
        if let Ok(bls) = storage.get_backlinks(&space) {
            for b in bls {
                let s = b.to_string_lossy().to_string();
                if !all.contains(&s) {
                    all.push(s);
                }
            }
        }
    }
    all.sort();
    all
}

/// Helper struct representing a search hit to be formatted into compact K-DSL wire format.
#[derive(Debug, Clone)]
pub struct CompactRecallHit<'a> {
    pub tier: &'a str,
    pub path: &'a str,
    pub score: f32,
    pub in_links: usize,
    pub out_links: usize,
    pub expanded_from: Option<&'a str>,
    pub raw_snippet: &'a str,
}

/// Converts a HierarchyLevel into its standard compact K-DSL tier code (L0..L3).
pub fn hierarchy_to_tier(hierarchy: HierarchyLevel) -> &'static str {
    match hierarchy {
        HierarchyLevel::L0Ephemeral => "L0",
        HierarchyLevel::L1Resource => "L1",
        HierarchyLevel::L2Log => "L2",
        HierarchyLevel::L3Evergreen => "L3",
    }
}

/// Fallback tier inference based on document path conventions.
pub fn infer_tier_from_path(path: &str) -> &'static str {
    let p = path.replace('\\', "/");
    if p.starts_with("00_") || p.contains("/00_") || p.contains("Daily") {
        "L0"
    } else if p.starts_with("10_") || p.contains("/10_") || p.contains("Projects") {
        "L1"
    } else if p.starts_with("01_") || p.contains("/01_") || p.contains("AI_Logs") {
        "L2"
    } else {
        "L3"
    }
}

/// Flattens multi-line snippet into a clean single line with collapsed whitespace.
pub fn clean_single_line_snippet(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Formats search hits into compact single-line K-DSL wire format:
/// `!μ:<tier>|<path>|score:<score>|links:<in>/<out>[|via:<parent_stem>] § <snippet_single_line_max_220_chars>`
/// ensuring each line is a single line, snippets do not exceed 220 characters,
/// and total output length is strictly under 1200 characters.
pub fn format_compact_kdsl_hits(hits: &[CompactRecallHit]) -> String {
    if hits.is_empty() {
        return String::new();
    }

    let n = hits.len();
    let normalized_paths: Vec<String> = hits.iter().map(|h| h.path.replace('\\', "/")).collect();

    let headers: Vec<String> = hits
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let via_segment = match h.expanded_from {
                Some(parent) => {
                    let stem = Path::new(parent)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(parent)
                        .replace('|', "/");
                    format!("|via:{}", stem)
                }
                None => String::new(),
            };
            format!(
                "!μ:{}|{}|score:{:.4}|rank:#{}|links:{}/{}{} § ",
                h.tier,
                normalized_paths[i],
                h.score,
                i + 1,
                h.in_links,
                h.out_links,
                via_segment
            )
        })
        .collect();

    let total_headers_len: usize = headers.iter().map(|h| h.len()).sum();
    let newlines_len = n.saturating_sub(1);
    let target_max_total: usize = 1190;
    let available_for_snippets = target_max_total.saturating_sub(total_headers_len + newlines_len);
    let max_per_snippet = (available_for_snippets / n).clamp(40, 220);

    let mut lines = Vec::with_capacity(n);
    for (i, hit) in hits.iter().enumerate() {
        let cleaned = clean_single_line_snippet(hit.raw_snippet);
        let snippet: String = cleaned.chars().take(max_per_snippet).collect();
        lines.push(format!("{}{}", headers[i], snippet.trim()));
    }

    let output = lines.join("\n");
    if output.len() >= 1200 {
        output.chars().take(1190).collect()
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchy_to_tier_mapping() {
        assert_eq!(hierarchy_to_tier(HierarchyLevel::L0Ephemeral), "L0");
        assert_eq!(hierarchy_to_tier(HierarchyLevel::L1Resource), "L1");
        assert_eq!(hierarchy_to_tier(HierarchyLevel::L2Log), "L2");
        assert_eq!(hierarchy_to_tier(HierarchyLevel::L3Evergreen), "L3");
    }

    #[test]
    fn test_clean_single_line_snippet() {
        let raw = "  # Title\n\n  Line 1   with   spaces\r\n\tLine 2  ";
        let cleaned = clean_single_line_snippet(raw);
        assert_eq!(cleaned, "# Title Line 1 with spaces Line 2");
        assert!(!cleaned.contains('\n'));
    }

    #[test]
    fn test_format_compact_kdsl_hits_wire_and_length_budget() {
        let long_body = "word ".repeat(100);
        let hits = vec![
            CompactRecallHit {
                tier: "L3",
                path: "20_Cards/Arch.md",
                score: 0.8542,
                in_links: 4,
                out_links: 2,
                expanded_from: None,
                raw_snippet: &long_body,
            },
            CompactRecallHit {
                tier: "L2",
                path: "01_AI_Logs/Log1.md",
                score: 0.1234,
                in_links: 1,
                out_links: 0,
                expanded_from: Some("10_Projects/PaymentGateway.md"),
                raw_snippet: "Short log snippet",
            },
        ];

        let formatted = format_compact_kdsl_hits(&hits);
        let lines: Vec<&str> = formatted.lines().collect();
        assert_eq!(lines.len(), 2);

        // Check line 1 format
        assert!(lines[0].starts_with("!μ:L3|20_Cards/Arch.md|score:0.8542|rank:#1|links:4/2 § "));
        assert!(!lines[0].contains('\n'));

        // Check line 2 format (includes |via:<parent_stem> after |links:<in>/<out>)
        assert!(lines[1].starts_with(
            "!μ:L2|01_AI_Logs/Log1.md|score:0.1234|rank:#2|links:1/0|via:PaymentGateway § Short log snippet"
        ));

        // Check length budget < 1200
        assert!(formatted.len() < 1200);

        // Verify with 5 hits
        let mut five_hits = Vec::new();
        for i in 0..5 {
            five_hits.push(CompactRecallHit {
                tier: "L3",
                path: "20_Cards/Doc.md",
                score: 0.5,
                in_links: i,
                out_links: i + 1,
                expanded_from: None,
                raw_snippet: &long_body,
            });
        }
        let five_formatted = format_compact_kdsl_hits(&five_hits);
        assert!(five_formatted.len() < 1200);
        for l in five_formatted.lines() {
            assert!(l.starts_with("!μ:"));
            let parts: Vec<&str> = l.split(" § ").collect();
            assert_eq!(parts.len(), 2);
            assert!(parts[1].chars().count() <= 220);
        }
    }
}
