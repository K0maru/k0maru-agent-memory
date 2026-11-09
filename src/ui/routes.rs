//! REST API routes and application state for K0maru developer dashboard.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

use crate::adapters::{GenericWikiAdapter, ObsidianAdapter};
use crate::core::models::SyncStats;
use crate::core::traits::{CacheStorage, VaultAdapter};
use crate::guard::GuardEngine;
use crate::scanner::VectorSyncStats;
use crate::storage::{HybridSearchEngine, SearchMode, SearchResult, SqliteStorage};
use crate::vector::EmbeddingEngine;
use k0maru_core::guard::models::{GuardRule, GuardViolation};

/// Shared runtime holding mutable vault state, active SQLite cache, and config.
pub struct VaultRuntime {
    pub vault_path: PathBuf,
    pub storage: SqliteStorage,
    pub embedder: Option<Arc<dyn EmbeddingEngine>>,
    pub config: crate::config::K0maruConfig,
}

/// Shared application state for dashboard HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    pub runtime: Arc<Mutex<VaultRuntime>>,
    pub sync_lock: Arc<Mutex<()>>,
    pub last_sync_epoch: Arc<AtomicU64>,
}

impl AppState {
    /// Initializes application state by opening SQLite cache in the vault.
    pub fn new(vault_path: PathBuf) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::with_embedder(vault_path, None)
    }

    /// Initializes application state with an optional embedding engine.
    pub fn with_embedder(
        vault_path: PathBuf,
        embedder: Option<Arc<dyn EmbeddingEngine>>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cache_dir = vault_path.join(".k0maru");
        let cache_path = cache_dir.join("cache.sqlite");
        let storage = match SqliteStorage::open(&cache_path) {
            Ok(s) => s,
            Err(_) => SqliteStorage::in_memory().map_err(
                |e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() },
            )?,
        };

        let embedder = if embedder.is_some() {
            embedder
        } else {
            let model_cache = if cache_dir.join("models").exists() {
                Some(cache_dir.join("models"))
            } else {
                None
            };
            crate::vector::default_embedding_engine(model_cache).ok()
        };

        let mut config = crate::config::K0maruConfig::load();
        config.record_vault(&vault_path);

        let runtime = VaultRuntime {
            vault_path,
            storage,
            embedder,
            config,
        };

        Ok(Self {
            runtime: Arc::new(Mutex::new(runtime)),
            sync_lock: Arc::new(Mutex::new(())),
            last_sync_epoch: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Initializes application state with an explicit storage and embedder instance.
    pub fn from_storage(
        vault_path: PathBuf,
        storage: SqliteStorage,
        embedder: Option<Arc<dyn EmbeddingEngine>>,
    ) -> Self {
        let mut config = crate::config::K0maruConfig::load();
        config.record_vault(&vault_path);

        let runtime = VaultRuntime {
            vault_path,
            storage,
            embedder,
            config,
        };

        Self {
            runtime: Arc::new(Mutex::new(runtime)),
            sync_lock: Arc::new(Mutex::new(())),
            last_sync_epoch: Arc::new(AtomicU64::new(0)),
        }
    }
}

/// Helper that checks whether a path corresponds to a forbidden system directory or bare $HOME.
pub fn is_forbidden_system_vault_path(path: &Path) -> bool {
    let raw = path.to_string_lossy();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return true;
    }

    let forbidden_roots = [
        "/", "/etc", "/usr", "/bin", "/sbin", "/System", "/var", "/Library",
    ];

    let clean_trimmed = trimmed.trim_end_matches('/');
    let to_check = if clean_trimmed.is_empty() {
        "/"
    } else {
        clean_trimmed
    };
    if forbidden_roots.contains(&to_check) {
        return true;
    }

    if std::env::var_os("HOME").map(PathBuf::from).as_deref() == Some(path) {
        return true;
    }

    if let Some(home_os) = std::env::var_os("HOME") {
        let home_path = PathBuf::from(home_os);
        if let Ok(canon_home) = home_path.canonicalize() {
            if let Ok(canon_path) = path.canonicalize() {
                if canon_path == canon_home {
                    return true;
                }
            }
        }
    }

    if let Ok(canon) = path.canonicalize() {
        let canon_str = canon.to_string_lossy();
        let canon_trimmed = canon_str.trim_end_matches('/');
        let canon_to_check = if canon_trimmed.is_empty() {
            "/"
        } else {
            canon_trimmed
        };
        if forbidden_roots.contains(&canon_to_check) {
            return true;
        }
    }

    false
}

fn is_loopback_host(host: &str) -> bool {
    let clean = host.trim_start_matches('[').trim_end_matches(']');
    clean == "localhost" || clean == "127.0.0.1" || clean == "::1"
}

fn is_allowed_origin_header_value(origin: &str) -> bool {
    let trimmed = origin.trim();
    if trimmed.is_empty() {
        return false;
    }
    if let Ok(uri) = trimmed.parse::<Uri>() {
        if let Some(host) = uri.host() {
            return is_loopback_host(host);
        }
    }
    let after_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed);
    let host_part = after_scheme.split('/').next().unwrap_or("");
    let host = host_part.split(':').next().unwrap_or("");
    is_loopback_host(host)
}

async fn guard_origin_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if req.method() == axum::http::Method::POST
        || req.method() == axum::http::Method::PUT
        || req.method() == axum::http::Method::PATCH
        || req.method() == axum::http::Method::DELETE
    {
        if let Some(origin_val) = req.headers().get(axum::http::header::ORIGIN) {
            if let Ok(origin_str) = origin_val.to_str() {
                if !is_allowed_origin_header_value(origin_str) {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(serde_json::json!({
                            "error": "Cross-origin request forbidden"
                        })),
                    )
                        .into_response();
                }
            } else {
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({
                        "error": "Invalid Origin header"
                    })),
                )
                    .into_response();
            }
        }
    }
    next.run(req).await
}

/// Status payload returned by `GET /api/status`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusResponse {
    pub vault_path: String,
    pub total_documents: usize,
    pub total_vectors: usize,
    pub cache_size_bytes: u64,
    pub total_logs: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_time: Option<u64>,
}

/// Query parameters for `GET /api/search`.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub mode: Option<String>,
    pub limit: Option<usize>,
}

/// Node representing a document in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub hierarchy: String,
    pub tags: Vec<String>,
}

/// Directed edge representing a WikiLink between documents in the graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
}

/// Graph payload returned by `GET /api/graph`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Log node metadata summary returned by `GET /api/logs`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogItemResponse {
    pub id: String,
    pub line_count: usize,
    pub task_id: Option<String>,
    pub created_at: String,
}

/// Detailed log inspection payload returned by `GET /api/logs/:id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogDetailResponse {
    pub id: String,
    pub mermaid: String,
    pub content: String,
}

/// Sync payload returned by `POST /api/sync`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncResponse {
    pub sync_stats: SyncStats,
    pub vector_stats: VectorSyncStats,
}

/// Cockpit config returned by `GET /api/config` and `POST /api/config`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigResponse {
    pub active_vault_path: String,
    pub recent_vaults: Vec<String>,
    pub embedding_model: String,
    pub default_search_mode: String,
    pub loadout_token_budget: usize,
    pub guard_strict: bool,
    pub passive_capture: bool,
    pub ui_port: u16,
    pub is_llm_wiki: bool,
    pub is_obsidian: bool,
}

/// Partial or full cockpit configuration update request payload.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateConfigRequest {
    pub vault_path: Option<String>,
    pub embedding_model: Option<String>,
    pub default_search_mode: Option<String>,
    pub loadout_token_budget: Option<usize>,
    pub guard_strict: Option<bool>,
    pub passive_capture: Option<bool>,
    pub ui_port: Option<u16>,
}

/// Zero-vault cold-start bootstrap request payload.
#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapRequest {
    pub path: String,
    pub project_name: Option<String>,
    pub install_hooks: Option<bool>,
}

/// Response returned after successfully bootstrapping a repository vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapApiResponse {
    pub status: String,
    pub vault_path: String,
    pub project_slug: String,
    pub created_files: Vec<String>,
    pub indexed_docs: usize,
}

/// Parameters for `GET /api/timeline`.
#[derive(Debug, Clone, Deserialize)]
pub struct TimelineParams {
    pub anchor: Option<String>,
    pub window: Option<usize>,
    pub project: Option<String>,
}

pub type TimelineQueryRequest = TimelineParams;

/// Response returned by `GET /api/timeline`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineResponse {
    pub total: usize,
    pub entries: Vec<crate::hook::TimelineEntry>,
}

/// Parameters for `GET /api/note`.
#[derive(Debug, Clone, Deserialize)]
pub struct NoteParams {
    pub path: String,
}

pub type NoteQueryRequest = NoteParams;

/// Note detail inspection payload returned by `GET /api/note`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteResponse {
    pub relative_path: String,
    pub title: String,
    pub hierarchy: String,
    pub status: String,
    pub frontmatter: serde_json::Value,
    pub superseded_by: Option<String>,
    pub supersedes: Vec<String>,
    pub outgoing_links: Vec<String>,
    pub incoming_links: Vec<String>,
    pub tags: Vec<String>,
    pub guard_rules_count: usize,
    pub body: String,
}

/// Response returned by `GET /api/guard/rules`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardRulesResponse {
    pub total: usize,
    pub rules: Vec<GuardRule>,
}

/// Request payload for `POST /api/guard/check`.
#[derive(Debug, Clone, Deserialize)]
pub struct GuardCheckRequest {
    pub diff: String,
    pub project: Option<String>,
}

pub type GuardCheckPayload = GuardCheckRequest;

/// Response returned by `POST /api/guard/check`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardCheckResponse {
    pub is_clean: bool,
    pub checked_files: usize,
    pub checked_diff_hunks: usize,
    pub violations: Vec<GuardViolation>,
}

/// Response returned by `GET /api/clients`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientsResponse {
    pub clients: Vec<crate::ecosystem::ClientConfigStatus>,
}

/// Request payload for `POST /api/clients/install`.
#[derive(Debug, Clone, Deserialize)]
pub struct InstallClientsRequest {
    #[serde(default)]
    pub target: String,
    pub client_id: Option<String>,
    pub all: Option<bool>,
    pub dry_run: Option<bool>,
    pub home_override: Option<String>,
}

/// Embedded static assets for the developer cockpit dashboard.
#[derive(RustEmbed)]
#[folder = "src/ui/assets/"]
pub struct DashboardAssets;

/// Creates the Axum router with all API routes, CORS layer, and embedded static asset serving.
pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::predicate(|origin_val, _| {
            if let Ok(s) = origin_val.to_str() {
                is_allowed_origin_header_value(s)
            } else {
                false
            }
        }))
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/status", get(handle_status))
        .route("/api/search", get(handle_search))
        .route("/api/graph", get(handle_graph))
        .route("/api/logs", get(handle_logs))
        .route("/api/logs/:id", get(handle_log_by_id))
        .route("/api/sync", post(handle_sync))
        .route(
            "/api/config",
            get(handle_get_config).post(handle_post_config),
        )
        .route("/api/vault/bootstrap", post(handle_vault_bootstrap))
        .route("/api/timeline", get(handle_timeline))
        .route("/api/note", get(handle_note))
        .route("/api/guard/rules", get(handle_guard_rules))
        .route("/api/guard/check", post(handle_guard_check))
        .route("/api/clients", get(handle_clients))
        .route("/api/clients/install", post(handle_clients_install))
        .layer(axum::middleware::from_fn(guard_origin_middleware))
        .fallback(static_handler)
        .layer(cors)
        .with_state(state)
}

/// Serves compile-time embedded static assets with MIME deduction and SPA fallback.
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let raw_path = uri.path();

    // If an unknown /api/* route was requested, return 404 JSON instead of HTML
    if raw_path.starts_with("/api") {
        return (
            StatusCode::NOT_FOUND,
            [(CONTENT_TYPE, "application/json")],
            serde_json::json!({
                "error": format!("API route '{}' not found", raw_path)
            })
            .to_string(),
        )
            .into_response();
    }

    let trimmed = raw_path.trim_start_matches('/');
    let lookup_path = if trimmed.is_empty() {
        "index.html".to_string()
    } else {
        trimmed.to_string()
    };

    match DashboardAssets::get(&lookup_path) {
        Some(content) => {
            let mime = mime_guess::from_path(&lookup_path).first_or_octet_stream();
            ([(CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            // SPA fallback to index.html for client-side routing
            if let Some(index) = DashboardAssets::get("index.html") {
                ([(CONTENT_TYPE, "text/html; charset=utf-8")], index.data).into_response()
            } else {
                (
                    StatusCode::NOT_FOUND,
                    [(CONTENT_TYPE, "text/plain")],
                    "404 Not Found",
                )
                    .into_response()
            }
        }
    }
}

async fn handle_status(
    State(state): State<AppState>,
) -> Result<Json<StatusResponse>, (StatusCode, Json<serde_json::Value>)> {
    let (vault_path, total_docs, total_vecs) = {
        let runtime = state.runtime.lock().await;
        let total_docs = runtime.storage.document_count().unwrap_or(0);
        let total_vecs = runtime.storage.vector_count().unwrap_or(0);
        (runtime.vault_path.clone(), total_docs, total_vecs)
    };

    let cache_file = vault_path.join(".k0maru").join("cache.sqlite");
    let cache_size_bytes = std::fs::metadata(&cache_file).map(|m| m.len()).unwrap_or(0);

    let logs = collect_log_files(&vault_path);
    let total_logs = logs.len();

    let last_sync = state.last_sync_epoch.load(Ordering::Relaxed);
    let last_sync_time = if last_sync > 0 { Some(last_sync) } else { None };

    Ok(Json(StatusResponse {
        vault_path: vault_path.to_string_lossy().to_string(),
        total_documents: total_docs,
        total_vectors: total_vecs,
        cache_size_bytes,
        total_logs,
        last_sync_time,
    }))
}

async fn handle_search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Response {
    let query_str = params.q.unwrap_or_default();
    if query_str.trim().is_empty() {
        return Json(Vec::<SearchResult>::new()).into_response();
    }

    let mode_str = params.mode.unwrap_or_else(|| "hybrid".to_string());
    let mode = match mode_str.to_lowercase().as_str() {
        "hybrid" => SearchMode::Hybrid,
        "bm25" => SearchMode::Bm25,
        "vector" => SearchMode::Vector,
        other => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Invalid search mode '{}'. Supported modes: hybrid, bm25, vector", other)
                })),
            )
                .into_response();
        }
    };

    let limit = params.limit.unwrap_or(5);

    let runtime = state.runtime.lock().await;
    let engine = HybridSearchEngine::new(&runtime.storage, runtime.embedder.clone());

    match engine.search(&query_str, mode, limit) {
        Ok(results) => Json(results).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn handle_graph(
    State(state): State<AppState>,
) -> Result<Json<GraphResponse>, (StatusCode, Json<serde_json::Value>)> {
    let runtime = state.runtime.lock().await;
    let conn = runtime.storage.connection();

    // 1. Fetch tags per document
    let mut tags_stmt = conn
        .prepare("SELECT document_path, tag FROM tags ORDER BY document_path ASC")
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    let mut tags_map: HashMap<String, Vec<String>> = HashMap::new();
    let tag_rows = tags_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    for (p, t) in tag_rows.flatten() {
        tags_map.entry(p).or_default().push(t);
    }

    // 2. Fetch all documents
    let mut doc_stmt = conn
        .prepare("SELECT path, title, hierarchy FROM documents ORDER BY path ASC")
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    let nodes = doc_stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let title: String = row.get(1)?;
            let hierarchy: String = row.get(2)?;
            let tags = tags_map.remove(&path).unwrap_or_default();
            Ok(GraphNode {
                id: path,
                title,
                hierarchy,
                tags,
            })
        })
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?
        .collect::<Result<Vec<GraphNode>, _>>()
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    // 3. Fetch all links
    let mut link_stmt = conn
        .prepare("SELECT DISTINCT source_path, target FROM links ORDER BY source_path, target")
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    let edges = link_stmt
        .query_map([], |row| {
            Ok(GraphEdge {
                source: row.get(0)?,
                target: row.get(1)?,
            })
        })
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?
        .collect::<Result<Vec<GraphEdge>, _>>()
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    Ok(Json(GraphResponse { nodes, edges }))
}

async fn handle_logs(State(state): State<AppState>) -> Json<Vec<LogItemResponse>> {
    let vault_path = {
        let runtime = state.runtime.lock().await;
        runtime.vault_path.clone()
    };
    let logs = collect_log_files(&vault_path);
    Json(logs)
}

async fn handle_log_by_id(
    State(state): State<AppState>,
    AxumPath(node_id): AxumPath<String>,
) -> Result<Json<LogDetailResponse>, (StatusCode, Json<serde_json::Value>)> {
    let vault_path = {
        let runtime = state.runtime.lock().await;
        runtime.vault_path.clone()
    };
    let candidate_dirs = candidate_log_dirs(&vault_path);

    let mut found_content: Option<String> = None;
    for dir in &candidate_dirs {
        if dir.is_dir() {
            if let Ok(content) = crate::offload::inspect_node(dir, &node_id) {
                found_content = Some(content);
                break;
            }
        }
    }

    let content = match found_content {
        Some(c) => c,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Log node '{}' not found", node_id)
                })),
            ));
        }
    };

    let total_lines = content.lines().count();
    let task_id = collect_log_files(&vault_path)
        .into_iter()
        .find(|l| l.id == node_id)
        .and_then(|l| l.task_id);

    let task_label = task_id.unwrap_or_else(|| "Running".to_string());
    let mermaid = format!(
        "stateDiagram-v2\n    direction TB\n    [*] --> {task_label}\n    {task_label} --> Offloaded : Log Captured ({total_lines} lines total)\n    state Offloaded {{\n        [*] --> Captured\n        Captured : Total {total_lines} lines captured\n        Captured : NodeID {node_id}\n        Captured : k0maru inspect {node_id}\n    }}\n    Offloaded --> [*]"
    );

    Ok(Json(LogDetailResponse {
        id: node_id,
        mermaid,
        content,
    }))
}

async fn handle_sync(
    State(state): State<AppState>,
) -> Result<Json<SyncResponse>, (StatusCode, Json<serde_json::Value>)> {
    let _sync_guard = state.sync_lock.lock().await;

    let (vault_path, embedder) = {
        let runtime = state.runtime.lock().await;
        let v = runtime.vault_path.clone();
        let emb = if runtime.embedder.is_some() {
            runtime.embedder.clone()
        } else {
            let model_cache = if v.join(".k0maru").join("models").exists() {
                Some(v.join(".k0maru").join("models"))
            } else {
                None
            };
            crate::vector::default_embedding_engine(model_cache).ok()
        };
        (v, emb)
    };

    let mut runtime = state.runtime.lock().await;

    let is_structured = vault_path.join(".obsidian").exists()
        || vault_path.join("projects").is_dir()
        || vault_path.join("decisions").is_dir()
        || vault_path.join("cards").is_dir()
        || vault_path.join("logs").is_dir()
        || vault_path.join("10_Projects").is_dir()
        || vault_path.join("20_Cards").is_dir();

    let res = if is_structured {
        let adapter = ObsidianAdapter::new(&vault_path);
        let mut scanner = crate::scanner::IncrementalScanner::new(&adapter, &mut runtime.storage);
        scanner.sync_vault_with_vector(&vault_path, embedder)
    } else {
        let adapter = GenericWikiAdapter::new(&vault_path);
        let mut scanner = crate::scanner::IncrementalScanner::new(&adapter, &mut runtime.storage);
        scanner.sync_vault_with_vector(&vault_path, embedder)
    };

    match res {
        Ok((sync_stats, vector_stats)) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            state.last_sync_epoch.store(now, Ordering::Relaxed);
            Ok(Json(SyncResponse {
                sync_stats,
                vector_stats,
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )),
    }
}

async fn handle_get_config(State(state): State<AppState>) -> Json<ConfigResponse> {
    let runtime = state.runtime.lock().await;
    let is_obsidian = runtime.vault_path.join(".obsidian").exists();
    let is_llm_wiki = !is_obsidian;

    Json(ConfigResponse {
        active_vault_path: runtime.vault_path.to_string_lossy().to_string(),
        recent_vaults: runtime
            .config
            .recent_vaults
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
        embedding_model: runtime.config.embedding_model.clone(),
        default_search_mode: runtime.config.default_search_mode.clone(),
        loadout_token_budget: runtime.config.loadout_token_budget,
        guard_strict: runtime.config.guard_strict,
        passive_capture: runtime.config.passive_capture,
        ui_port: runtime.config.ui_port,
        is_llm_wiki,
        is_obsidian,
    })
}

async fn handle_post_config(
    State(state): State<AppState>,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<Json<ConfigResponse>, (StatusCode, Json<serde_json::Value>)> {
    let mut runtime = state.runtime.lock().await;

    if let Some(v_str) = &req.vault_path {
        let v_trimmed = v_str.trim();
        if v_trimmed.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Vault path cannot be empty" })),
            ));
        }

        let raw_path = PathBuf::from(v_trimmed);
        if is_forbidden_system_vault_path(&raw_path) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Forbidden system vault path" })),
            ));
        }

        let mut target_vault = raw_path;
        if !target_vault.exists() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Vault path does not exist" })),
            ));
        }
        if !target_vault.is_dir() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Vault path is not a directory" })),
            ));
        }

        if target_vault.join(".k0maru/wiki").is_dir()
            && !target_vault.join("INDEX.md").is_file()
            && !target_vault.join("projects").is_dir()
            && !target_vault.join("10_Projects").is_dir()
        {
            target_vault = target_vault.join(".k0maru/wiki");
        }

        let canonical_vault = match target_vault.canonicalize() {
            Ok(c) => c,
            Err(_) => target_vault,
        };

        if is_forbidden_system_vault_path(&canonical_vault) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Forbidden system vault path" })),
            ));
        }

        let cache_dir = canonical_vault.join(".k0maru");
        let cache_path = cache_dir.join("cache.sqlite");
        let mut new_storage = match SqliteStorage::open(&cache_path) {
            Ok(s) => s,
            Err(_) => SqliteStorage::in_memory().map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?,
        };

        let is_structured = canonical_vault.join(".obsidian").exists()
            || canonical_vault.join("projects").is_dir()
            || canonical_vault.join("decisions").is_dir()
            || canonical_vault.join("cards").is_dir()
            || canonical_vault.join("logs").is_dir()
            || canonical_vault.join("10_Projects").is_dir()
            || canonical_vault.join("20_Cards").is_dir();

        if is_structured {
            let adapter = ObsidianAdapter::new(&canonical_vault);
            let mut scanner = crate::scanner::IncrementalScanner::new(&adapter, &mut new_storage);
            let _ = scanner.sync(false);
        } else {
            let adapter = GenericWikiAdapter::new(&canonical_vault);
            let mut scanner = crate::scanner::IncrementalScanner::new(&adapter, &mut new_storage);
            let _ = scanner.sync(false);
        }

        runtime.vault_path = canonical_vault.clone();
        runtime.storage = new_storage;
        runtime.config.record_vault(&canonical_vault);
    }

    if let Some(m) = &req.embedding_model {
        runtime.config.embedding_model = m.clone();
    }
    if let Some(s) = &req.default_search_mode {
        runtime.config.default_search_mode = s.clone();
    }
    if let Some(b) = req.loadout_token_budget {
        runtime.config.loadout_token_budget = b;
    }
    if let Some(s) = req.guard_strict {
        runtime.config.guard_strict = s;
    }
    if let Some(p) = req.passive_capture {
        runtime.config.passive_capture = p;
    }
    if let Some(p) = req.ui_port {
        runtime.config.ui_port = p;
    }

    let _ = runtime.config.save();

    let is_obsidian = runtime.vault_path.join(".obsidian").exists();
    let is_llm_wiki = !is_obsidian;

    Ok(Json(ConfigResponse {
        active_vault_path: runtime.vault_path.to_string_lossy().to_string(),
        recent_vaults: runtime
            .config
            .recent_vaults
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
        embedding_model: runtime.config.embedding_model.clone(),
        default_search_mode: runtime.config.default_search_mode.clone(),
        loadout_token_budget: runtime.config.loadout_token_budget,
        guard_strict: runtime.config.guard_strict,
        passive_capture: runtime.config.passive_capture,
        ui_port: runtime.config.ui_port,
        is_llm_wiki,
        is_obsidian,
    }))
}

async fn handle_vault_bootstrap(
    State(state): State<AppState>,
    Json(req): Json<BootstrapRequest>,
) -> Result<Json<BootstrapApiResponse>, (StatusCode, Json<serde_json::Value>)> {
    let repo_path = PathBuf::from(&req.path);
    if is_forbidden_system_vault_path(&repo_path) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Forbidden system directory path" })),
        ));
    }

    let is_code_repo = repo_path.join(".git").exists()
        || repo_path.join("Cargo.toml").exists()
        || repo_path.join("package.json").exists()
        || repo_path.join("pyproject.toml").exists()
        || repo_path.join("go.mod").exists();
    let report = crate::bootstrap::run_bootstrap(crate::bootstrap::BootstrapOptions {
        repo_path: repo_path.clone(),
        vault_path: if is_code_repo { None } else { Some(repo_path) },
        project_name: req.project_name,
        install_clients: req.install_hooks.unwrap_or(false),
        home_override: None,
    })
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let cache_dir = report.vault_path.join(".k0maru");
    let cache_path = cache_dir.join("cache.sqlite");
    let new_storage = match SqliteStorage::open(&cache_path) {
        Ok(s) => s,
        Err(_) => SqliteStorage::in_memory().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?,
    };

    let mut runtime = state.runtime.lock().await;
    runtime.vault_path = report.vault_path.clone();
    runtime.storage = new_storage;
    runtime.config.record_vault(&report.vault_path);
    let _ = runtime.config.save();

    Ok(Json(BootstrapApiResponse {
        status: "ok".to_string(),
        vault_path: report.vault_path.to_string_lossy().to_string(),
        project_slug: report.project_slug,
        created_files: report
            .created_files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
        indexed_docs: report.indexed_docs,
    }))
}

async fn handle_timeline(
    State(state): State<AppState>,
    Query(params): Query<TimelineParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let vault_path = {
        let runtime = state.runtime.lock().await;
        runtime.vault_path.clone()
    };

    let window = params.window.unwrap_or(20);
    let query = crate::hook::TimelineQuery {
        anchor: params.anchor.clone(),
        window,
        project: params.project.clone(),
    };

    let built_entries = crate::hook::build_timeline(&vault_path, &query)
        .ok()
        .filter(|e| !e.is_empty());
    let entries = if let Some(e) = built_entries {
        e
    } else {
        let docs_info = {
            let runtime = state.runtime.lock().await;
            let conn = runtime.storage.connection();
            let stmt = conn
                .prepare("SELECT path, title, hierarchy FROM documents ORDER BY mtime DESC")
                .ok();
            if let Some(mut s) = stmt {
                s.query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map(|r| r.flatten().collect::<Vec<_>>())
                .unwrap_or_default()
            } else {
                Vec::new()
            }
        };
        let mut fallback = Vec::new();
        for (rel_path, title, hier) in docs_info.into_iter().take(window) {
            let id = Path::new(&rel_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&rel_path)
                .to_string();
            fallback.push(crate::hook::TimelineEntry {
                id,
                timestamp: "".to_string(),
                date: "".to_string(),
                kind: hier.clone(),
                hierarchy: hier,
                title,
                relative_path: rel_path,
                summary: "".to_string(),
                status: "active".to_string(),
                project: params.project.clone(),
            });
        }
        fallback
    };

    Ok(Json(serde_json::json!({
        "total": entries.len(),
        "entries": entries,
    })))
}

async fn handle_note(
    State(state): State<AppState>,
    Query(params): Query<NoteParams>,
) -> Result<Json<NoteResponse>, (StatusCode, Json<serde_json::Value>)> {
    let rel_path_str = params.path.trim();
    if rel_path_str.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Path parameter cannot be empty" })),
        ));
    }

    let rel_path = Path::new(rel_path_str);
    if rel_path.is_absolute() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Path must be relative" })),
        ));
    }

    for comp in rel_path.components() {
        if matches!(comp, std::path::Component::ParentDir) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Path traversal (..) is forbidden" })),
            ));
        }
    }

    if !rel_path_str.ends_with(".md") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Only .md files are accessible" })),
        ));
    }

    let runtime = state.runtime.lock().await;
    let vault_path = &runtime.vault_path;

    let canon_vault = match vault_path.canonicalize() {
        Ok(c) => c,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            ));
        }
    };

    let target_file = vault_path.join(rel_path);
    let canon_note = match target_file.canonicalize() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Note not found" })),
            ));
        }
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid note path" })),
            ));
        }
    };

    if !canon_note.starts_with(&canon_vault) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Access denied: path escapes vault root" })),
        ));
    }

    if !canon_note.is_file() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Target path is not a file" })),
        ));
    }

    let is_structured = vault_path.join(".obsidian").exists()
        || vault_path.join("projects").is_dir()
        || vault_path.join("decisions").is_dir()
        || vault_path.join("cards").is_dir()
        || vault_path.join("logs").is_dir()
        || vault_path.join("10_Projects").is_dir()
        || vault_path.join("20_Cards").is_dir();

    let doc = if is_structured {
        let adapter = ObsidianAdapter::new(vault_path);
        adapter.read_document(rel_path).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?
    } else {
        let adapter = GenericWikiAdapter::new(vault_path);
        adapter.read_document(rel_path).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?
    };

    let mut outgoing_links: Vec<String> = runtime
        .storage
        .get_outgoing_links(rel_path)
        .map(|links| links.into_iter().map(|l| l.target).collect())
        .unwrap_or_default();
    if outgoing_links.is_empty() {
        outgoing_links = doc.links.iter().map(|l| l.target.clone()).collect();
    }

    let mut incoming_links: Vec<String> = runtime
        .storage
        .get_backlinks(&doc.title)
        .map(|paths| {
            paths
                .into_iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();

    let file_stem = rel_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if !file_stem.is_empty() && file_stem != doc.title {
        if let Ok(extra) = runtime.storage.get_backlinks(file_stem) {
            for p in extra {
                let p_str = p.to_string_lossy().to_string();
                if !incoming_links.contains(&p_str) {
                    incoming_links.push(p_str);
                }
            }
        }
    }

    let guard_rules = k0maru_core::guard::extractor::extract_rules_from_document(&doc);
    let guard_rules_count = guard_rules.len();

    let status = doc.lifecycle_status().to_string();
    let superseded_by = doc.superseded_by();
    let supersedes = doc.supersedes();

    Ok(Json(NoteResponse {
        relative_path: rel_path.to_string_lossy().replace('\\', "/"),
        title: doc.title,
        hierarchy: doc.hierarchy.to_string(),
        status,
        frontmatter: doc.frontmatter,
        superseded_by,
        supersedes,
        outgoing_links,
        incoming_links,
        tags: doc.tags,
        guard_rules_count,
        body: doc.body,
    }))
}

async fn handle_guard_rules(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let vault_path = {
        let runtime = state.runtime.lock().await;
        runtime.vault_path.clone()
    };

    let rules = crate::guard::resolve_guard_rules(&vault_path, None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(serde_json::json!({
        "total": rules.len(),
        "rules": rules,
    })))
}

async fn handle_guard_check(
    State(state): State<AppState>,
    Json(payload): Json<GuardCheckRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let vault_path = {
        let runtime = state.runtime.lock().await;
        runtime.vault_path.clone()
    };

    let rules = crate::guard::resolve_guard_rules(&vault_path, payload.project.as_deref())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    let report = GuardEngine::check_diff(&payload.diff, &rules);

    Ok(Json(serde_json::json!({
        "is_clean": report.violations.is_empty(),
        "checked_files": report.checked_files,
        "checked_diff_hunks": report.checked_diff_hunks,
        "violations": report.violations,
    })))
}

async fn handle_clients() -> Json<serde_json::Value> {
    let clients = crate::ecosystem::inspect_all_clients(None);
    Json(serde_json::json!({ "clients": clients }))
}

async fn handle_clients_install(
    State(state): State<AppState>,
    Json(req): Json<InstallClientsRequest>,
) -> Result<Json<crate::install::InstallReport>, (StatusCode, Json<serde_json::Value>)> {
    let resolved_target = if !req.target.trim().is_empty() {
        req.target.trim()
    } else if let Some(cid) = &req.client_id {
        cid.trim()
    } else if req.all == Some(true) {
        "all"
    } else {
        ""
    };
    let target_str = resolved_target;
    if target_str.is_empty()
        || !target_str
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ',')
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid client target parameter" })),
        ));
    }

    let install_target: crate::install::InstallTarget =
        target_str.parse().map_err(|e: String| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            )
        })?;

    let vault_path = {
        let runtime = state.runtime.lock().await;
        runtime.vault_path.clone()
    };

    let home_override = req.home_override.as_ref().map(PathBuf::from);

    let report = crate::install::run_install(crate::install::InstallOptions {
        vault_path,
        target: install_target,
        dry_run: req.dry_run.unwrap_or(false),
        home_override,
    })
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(report))
}

fn candidate_log_dirs(vault_path: &Path) -> Vec<PathBuf> {
    let vault_k0maru = vault_path.join(".k0maru").join("refs");
    let vault_scratch = vault_path.join(".scratch").join("refs");
    let mut dirs = Vec::new();

    if vault_k0maru.is_dir() {
        dirs.push(vault_k0maru);
    }
    if vault_scratch.is_dir() {
        dirs.push(vault_scratch);
    }

    if dirs.is_empty() {
        let global_scratch = PathBuf::from(".scratch/refs");
        if global_scratch.is_dir() {
            dirs.push(global_scratch);
        }
    }

    dirs
}

/// Discovers and summarizes offloaded log files in candidate reference directories.
pub fn collect_log_files(vault_path: &Path) -> Vec<LogItemResponse> {
    let mut log_map: HashMap<String, LogItemResponse> = HashMap::new();
    let dirs = candidate_log_dirs(vault_path);

    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                if ext != "log" {
                    continue;
                }

                let file_name = match path.file_name().and_then(|s| s.to_str()) {
                    Some(n) => n,
                    None => continue,
                };

                let stem = file_name.strip_suffix(".log").unwrap_or(file_name);
                let (id, task_id) = if let Some(idx) = stem.find("node_") {
                    let node_part = stem[idx..].to_string();
                    let prefix = stem[..idx].trim_end_matches('_');
                    let task = if prefix.is_empty() {
                        None
                    } else {
                        Some(prefix.to_string())
                    };
                    (node_part, task)
                } else {
                    (stem.to_string(), None)
                };

                let line_count = File::open(&path)
                    .map(|f| BufReader::new(f).lines().count())
                    .unwrap_or(0);

                let created_at = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.created().or_else(|_| m.modified()).ok())
                    .map(format_system_time)
                    .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

                log_map.entry(id.clone()).or_insert(LogItemResponse {
                    id,
                    line_count,
                    task_id,
                    created_at,
                });
            }
        }
    }

    let mut logs: Vec<LogItemResponse> = log_map.into_values().collect();
    logs.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    logs
}

fn format_system_time(st: std::time::SystemTime) -> String {
    let dur = st.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let z = (secs / 86400) as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1020 + doe / 1461 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let sec_of_day = secs % 86400;
    let h = sec_of_day / 3600;
    let min = (sec_of_day % 3600) / 60;
    let s = sec_of_day % 60;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, h, min, s)
}
