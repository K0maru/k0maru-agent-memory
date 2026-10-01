use std::fs;
use std::path::Path;
use std::sync::Arc;

use assert_cmd::Command;
use axum::body::to_bytes;
use axum::http::{Request, StatusCode};
use tempfile::tempdir;
use tower::ServiceExt;

use k0maru::storage::SqliteStorage;
use k0maru::ui::routes::{
    create_router, AppState, GraphResponse, LogDetailResponse, LogItemResponse, StatusResponse,
    SyncResponse,
};
use k0maru::vector::MockEmbeddingEngine;

fn setup_mock_vault(vault_dir: &Path) {
    let proj_dir = vault_dir.join("10_Projects");
    let cards_dir = vault_dir.join("20_Cards");
    let refs_dir = vault_dir.join(".k0maru").join("refs");

    fs::create_dir_all(&proj_dir).unwrap();
    fs::create_dir_all(&cards_dir).unwrap();
    fs::create_dir_all(&refs_dir).unwrap();

    let alpha_content = r#"---
title: Project Alpha
tags: [project, core]
---
# Project Alpha
High priority project description linking to [[Architecture]] and [[Database]].
"#;
    fs::write(proj_dir.join("Alpha.md"), alpha_content).unwrap();

    let arch_content = r#"---
title: Architecture Spec
tags: [architecture, design]
---
# Architecture Spec
Foundational architectural decisions for [[Alpha]].
"#;
    fs::write(cards_dir.join("Architecture.md"), arch_content).unwrap();

    let log1_content = (1..=20)
        .map(|i| format!("Line {}: Build step executing successfully", i))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(refs_dir.join("node_11223344.log"), log1_content).unwrap();

    let log2_content = (1..=15)
        .map(|i| format!("Line {}: Test run log output", i))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(refs_dir.join("task_verify_node_55667788.log"), log2_content).unwrap();
}

#[tokio::test]
async fn test_ui_api_status_and_sync() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    setup_mock_vault(&vault_path);

    let storage = SqliteStorage::in_memory().unwrap();
    let embedder = Arc::new(MockEmbeddingEngine::new(384));
    let state = AppState::from_storage(vault_path.clone(), storage, Some(embedder));
    let app = create_router(state);

    // Initial status before sync
    let req = Request::builder()
        .uri("/api/status")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let status: StatusResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(status.total_documents, 0);
    assert_eq!(status.total_logs, 2);

    // Trigger POST /api/sync
    let sync_req = Request::builder()
        .method("POST")
        .uri("/api/sync")
        .body(axum::body::Body::empty())
        .unwrap();
    let sync_res = app.clone().oneshot(sync_req).await.unwrap();
    assert_eq!(sync_res.status(), StatusCode::OK);
    let sync_body = to_bytes(sync_res.into_body(), usize::MAX).await.unwrap();
    let sync_json: SyncResponse = serde_json::from_slice(&sync_body).unwrap();
    assert_eq!(sync_json.sync_stats.added, 2);
    assert_eq!(sync_json.vector_stats.embedded_count, 2);

    // Status after sync
    let req = Request::builder()
        .uri("/api/status")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let status_after: StatusResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(status_after.total_documents, 2);
    assert_eq!(status_after.total_vectors, 2);
    assert!(status_after.last_sync_time.is_some());
}

#[tokio::test]
async fn test_ui_api_search_and_modes() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    setup_mock_vault(&vault_path);

    let storage = SqliteStorage::in_memory().unwrap();
    let embedder = Arc::new(MockEmbeddingEngine::new(384));
    let state = AppState::from_storage(vault_path.clone(), storage, Some(embedder));
    let app = create_router(state);

    // Sync vault first
    let sync_req = Request::builder()
        .method("POST")
        .uri("/api/sync")
        .body(axum::body::Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(sync_req).await.unwrap();

    // Hybrid search
    let req = Request::builder()
        .uri("/api/search?q=Alpha&mode=hybrid&limit=5")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let results: Vec<k0maru::storage::SearchResult> = serde_json::from_slice(&body).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].title, "Project Alpha");

    // BM25 search
    let req = Request::builder()
        .uri("/api/search?q=Architecture&mode=bm25")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let results: Vec<k0maru::storage::SearchResult> = serde_json::from_slice(&body).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].title, "Architecture Spec");

    // Empty query returns empty array
    let req = Request::builder()
        .uri("/api/search?q=")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let results: Vec<k0maru::storage::SearchResult> = serde_json::from_slice(&body).unwrap();
    assert!(results.is_empty());

    // Invalid mode returns 400 Bad Request
    let req = Request::builder()
        .uri("/api/search?q=Alpha&mode=unsupported")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_ui_api_graph_and_links() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    setup_mock_vault(&vault_path);

    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path.clone(), storage, None);
    let app = create_router(state);

    // Sync vault
    let sync_req = Request::builder()
        .method("POST")
        .uri("/api/sync")
        .body(axum::body::Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(sync_req).await.unwrap();

    // Query graph
    let req = Request::builder()
        .uri("/api/graph")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let graph: GraphResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(graph.nodes.len(), 2);
    let alpha_node = graph
        .nodes
        .iter()
        .find(|n| n.title == "Project Alpha")
        .unwrap();
    assert!(alpha_node.tags.contains(&"project".to_string()));
    assert!(alpha_node.tags.contains(&"core".to_string()));

    // Links: Alpha links to Architecture and Database, Architecture links to Alpha
    assert!(!graph.edges.is_empty());
    assert!(graph
        .edges
        .iter()
        .any(|e| e.source.contains("Alpha.md") && e.target == "Architecture"));
}

#[tokio::test]
async fn test_ui_api_logs_listing_and_inspection() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    setup_mock_vault(&vault_path);

    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path.clone(), storage, None);
    let app = create_router(state);

    // List logs
    let req = Request::builder()
        .uri("/api/logs")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let logs: Vec<LogItemResponse> = serde_json::from_slice(&body).unwrap();
    assert_eq!(logs.len(), 2);

    let node1 = logs.iter().find(|l| l.id == "node_11223344").unwrap();
    assert_eq!(node1.line_count, 20);
    assert_eq!(node1.task_id, None);

    let node2 = logs.iter().find(|l| l.id == "node_55667788").unwrap();
    assert_eq!(node2.line_count, 15);
    assert_eq!(node2.task_id, Some("task_verify".to_string()));

    // Inspect node 1
    let req = Request::builder()
        .uri("/api/logs/node_11223344")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let detail: LogDetailResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(detail.id, "node_11223344");
    assert!(detail.mermaid.contains("stateDiagram-v2"));
    assert!(detail.mermaid.contains("node_11223344"));
    assert!(detail
        .content
        .contains("Line 1: Build step executing successfully"));

    // Inspect node 2
    let req = Request::builder()
        .uri("/api/logs/node_55667788")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let detail2: LogDetailResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(detail2.id, "node_55667788");
    assert!(detail2.mermaid.contains("task_verify"));

    // Inspect non-existent node returns 404
    let req = Request::builder()
        .uri("/api/logs/node_notfound")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_ui_root_fallback() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path, storage, None);
    let app = create_router(state);

    // Root GET /
    let req = Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("<h1>K0maru Dashboard API Active</h1>"));

    // Fallback GET /dashboard/explorer
    let req = Request::builder()
        .uri("/dashboard/explorer")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("<h1>K0maru Dashboard API Active</h1>"));
}

#[test]
fn test_cli_ui_help() {
    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.arg("ui").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains(
            "Launch the local developer dashboard",
        ))
        .stdout(predicates::str::contains("--vault"))
        .stdout(predicates::str::contains("--port"))
        .stdout(predicates::str::contains("--open"));
}

#[tokio::test]
async fn test_ui_api_vector_search() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    setup_mock_vault(&vault_path);

    let storage = SqliteStorage::in_memory().unwrap();
    let embedder = Arc::new(MockEmbeddingEngine::new(384));
    let state = AppState::from_storage(vault_path.clone(), storage, Some(embedder));
    let app = create_router(state);

    // Sync vault to embed
    let sync_req = Request::builder()
        .method("POST")
        .uri("/api/sync")
        .body(axum::body::Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(sync_req).await.unwrap();

    // Vector search mode
    let req = Request::builder()
        .uri("/api/search?q=Architecture&mode=vector&limit=5")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let results: Vec<k0maru::storage::SearchResult> = serde_json::from_slice(&body).unwrap();
    assert!(!results.is_empty());
    assert!(results[0].vector_rank.is_some());
}

#[tokio::test]
async fn test_ui_api_logs_with_extension() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    setup_mock_vault(&vault_path);

    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path.clone(), storage, None);
    let app = create_router(state);

    // Inspect with .log extension in url
    let req = Request::builder()
        .uri("/api/logs/node_11223344.log")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let detail: LogDetailResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(detail.id, "node_11223344.log");
    assert!(detail
        .content
        .contains("Line 1: Build step executing successfully"));
}

#[tokio::test]
async fn test_ui_server_loopback_and_port_conflict() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    setup_mock_vault(&vault_path);

    // Pick an available port
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = std_listener.local_addr().unwrap().port();
    drop(std_listener);

    let state = AppState::new(vault_path).unwrap();
    let router = create_router(state);
    let bind_addr = format!("127.0.0.1:{}", port);
    let tcp_listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });

    // Verify port conflict detection
    let second_bind = tokio::net::TcpListener::bind(&bind_addr).await;
    assert!(
        second_bind.is_err(),
        "Binding occupied port must return error"
    );

    // Send HTTP GET request via TcpStream
    let mut stream = tokio::net::TcpStream::connect(&bind_addr).await.unwrap();
    stream
        .write_all(b"GET /api/status HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();

    let mut response_buf = Vec::new();
    stream.read_to_end(&mut response_buf).await.unwrap();
    let response_str = String::from_utf8_lossy(&response_buf);
    assert!(response_str.starts_with("HTTP/1.1 200 OK"));
    assert!(response_str.contains("\"total_logs\":2"));

    // Gracefully shut down
    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
}
