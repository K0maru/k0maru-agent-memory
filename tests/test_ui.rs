use std::fs;
use std::path::Path;
use std::sync::Arc;

use axum::body::to_bytes;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use tempfile::tempdir;
use tower::ServiceExt;

use k0maru::storage::SqliteStorage;
use k0maru::ui::routes::{
    create_router, AppState, DashboardAssets, GraphResponse, StatusResponse, SyncResponse,
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
High priority project description linking to [[Architecture]].
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

    let log_content = (1..=20)
        .map(|i| format!("Line {}: Build step executing", i))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(refs_dir.join("node_11223344.log"), log_content).unwrap();
}

#[test]
fn test_embedded_assets_bundle_integrity() {
    assert!(DashboardAssets::get("index.html").is_some());
    assert!(DashboardAssets::get("style.css").is_some());
    assert!(DashboardAssets::get("app.js").is_some());

    let index_file = DashboardAssets::get("index.html").unwrap();
    let index_html = std::str::from_utf8(&index_file.data).unwrap();
    assert!(index_html.contains("K0maru Agent Memory Hub"));
    assert!(index_html.contains("Search Debugger"));
    assert!(index_html.contains("Graph Explorer"));
}

#[tokio::test]
async fn test_ui_static_embed_root() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path, storage, None);
    let app = create_router(state);

    let req = Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let content_type = res
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(content_type.starts_with("text/html"));
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

    // Initial status
    let req = Request::builder()
        .uri("/api/status")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let status: StatusResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(status.total_documents, 0);

    // Sync
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
}

#[tokio::test]
async fn test_ui_api_graph_and_search() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    setup_mock_vault(&vault_path);

    let storage = SqliteStorage::in_memory().unwrap();
    let embedder = Arc::new(MockEmbeddingEngine::new(384));
    let state = AppState::from_storage(vault_path.clone(), storage, Some(embedder));
    let app = create_router(state);

    // Sync vault
    let sync_req = Request::builder()
        .method("POST")
        .uri("/api/sync")
        .body(axum::body::Body::empty())
        .unwrap();
    app.clone().oneshot(sync_req).await.unwrap();

    // Query Graph API
    let graph_req = Request::builder()
        .uri("/api/graph")
        .body(axum::body::Body::empty())
        .unwrap();
    let graph_res = app.clone().oneshot(graph_req).await.unwrap();
    assert_eq!(graph_res.status(), StatusCode::OK);
    let graph_body = to_bytes(graph_res.into_body(), usize::MAX).await.unwrap();
    let graph_json: GraphResponse = serde_json::from_slice(&graph_body).unwrap();
    assert_eq!(graph_json.nodes.len(), 2);
    assert_eq!(graph_json.edges.len(), 2);

    // Query Search API
    let search_req = Request::builder()
        .uri("/api/search?q=Alpha&mode=hybrid&limit=5")
        .body(axum::body::Body::empty())
        .unwrap();
    let search_res = app.clone().oneshot(search_req).await.unwrap();
    assert_eq!(search_res.status(), StatusCode::OK);
}
