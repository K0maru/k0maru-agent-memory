//! REST API routes and application state for K0maru developer dashboard.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

use crate::core::models::SyncStats;
use crate::scanner::VectorSyncStats;
use crate::storage::{HybridSearchEngine, SearchMode, SearchResult, SqliteStorage};
use crate::vector::EmbeddingEngine;

/// Shared application state for dashboard HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    pub vault_path: PathBuf,
    pub storage: Arc<Mutex<SqliteStorage>>,
    pub embedder: Option<Arc<dyn EmbeddingEngine>>,
    pub last_sync_time: Arc<AtomicU64>,
}

impl AppState {
    /// Initializes application state by opening SQLite cache in the vault.
    pub fn new(vault_path: PathBuf) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cache_dir = vault_path.join(".k0maru");
        let cache_path = cache_dir.join("cache.sqlite");
        let storage = match SqliteStorage::open(&cache_path) {
            Ok(s) => s,
            Err(_) => SqliteStorage::in_memory().map_err(
                |e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() },
            )?,
        };

        let model_cache = if cache_dir.join("models").exists() {
            Some(cache_dir.join("models"))
        } else {
            None
        };
        let embedder = crate::vector::default_embedding_engine(model_cache).ok();

        Ok(Self {
            vault_path,
            storage: Arc::new(Mutex::new(storage)),
            embedder,
            last_sync_time: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Initializes application state with an explicit storage and embedder instance.
    pub fn from_storage(
        vault_path: PathBuf,
        storage: SqliteStorage,
        embedder: Option<Arc<dyn EmbeddingEngine>>,
    ) -> Self {
        Self {
            vault_path,
            storage: Arc::new(Mutex::new(storage)),
            embedder,
            last_sync_time: Arc::new(AtomicU64::new(0)),
        }
    }
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

/// Creates the Axum router with all API routes, CORS layer, and root HTML fallback.
pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get(handle_root))
        .route("/api/status", get(handle_status))
        .route("/api/search", get(handle_search))
        .route("/api/graph", get(handle_graph))
        .route("/api/logs", get(handle_logs))
        .route("/api/logs/:id", get(handle_log_by_id))
        .route("/api/sync", post(handle_sync))
        .fallback(handle_root)
        .layer(cors)
        .with_state(state)
}

async fn handle_root() -> Html<&'static str> {
    Html("<!DOCTYPE html><html><head><title>K0maru Dashboard</title></head><body><h1>K0maru Dashboard API Active</h1></body></html>")
}

async fn handle_status(
    State(state): State<AppState>,
) -> Result<Json<StatusResponse>, (StatusCode, Json<serde_json::Value>)> {
    let (total_docs, total_vecs) = {
        let storage = state.storage.lock().await;
        let total_docs = storage.document_count().unwrap_or(0);
        let total_vecs = storage.vector_count().unwrap_or(0);
        (total_docs, total_vecs)
    };

    let cache_file = state.vault_path.join(".k0maru").join("cache.sqlite");
    let cache_size_bytes = std::fs::metadata(&cache_file).map(|m| m.len()).unwrap_or(0);

    let logs = collect_log_files(&state.vault_path);
    let total_logs = logs.len();

    let last_sync = state.last_sync_time.load(Ordering::Relaxed);
    let last_sync_time = if last_sync > 0 { Some(last_sync) } else { None };

    Ok(Json(StatusResponse {
        vault_path: state.vault_path.to_string_lossy().to_string(),
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
            ).into_response();
        }
    };

    let limit = params.limit.unwrap_or(5);

    let storage = state.storage.lock().await;
    let engine = HybridSearchEngine::new(&storage, state.embedder.clone());

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
    let storage = state.storage.lock().await;
    let conn = storage.connection();

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
    let logs = collect_log_files(&state.vault_path);
    Json(logs)
}

async fn handle_log_by_id(
    State(state): State<AppState>,
    AxumPath(node_id): AxumPath<String>,
) -> Result<Json<LogDetailResponse>, (StatusCode, Json<serde_json::Value>)> {
    let candidate_dirs = candidate_log_dirs(&state.vault_path);

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
    let task_id = collect_log_files(&state.vault_path)
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
    let vault_path = state.vault_path.clone();
    let embedder = if state.embedder.is_some() {
        state.embedder.clone()
    } else {
        let model_cache = if vault_path.join(".k0maru").join("models").exists() {
            Some(vault_path.join(".k0maru").join("models"))
        } else {
            None
        };
        crate::vector::default_embedding_engine(model_cache).ok()
    };

    let mut storage = state.storage.lock().await;
    let res = if vault_path.join("10_Projects").is_dir() || vault_path.join(".obsidian").exists() {
        let adapter = crate::adapters::ObsidianAdapter::new(&vault_path);
        let mut scanner = crate::scanner::IncrementalScanner::new(&adapter, &mut *storage);
        scanner.sync_vault_with_vector(&vault_path, embedder)
    } else {
        let adapter = crate::adapters::GenericWikiAdapter::new(&vault_path);
        let mut scanner = crate::scanner::IncrementalScanner::new(&adapter, &mut *storage);
        scanner.sync_vault_with_vector(&vault_path, embedder)
    };

    match res {
        Ok((sync_stats, vector_stats)) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            state.last_sync_time.store(now, Ordering::Relaxed);
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
