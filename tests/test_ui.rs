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
    create_router, is_forbidden_system_vault_path, AppState, ConfigResponse, DashboardAssets,
    GraphResponse, NoteResponse, StatusResponse, SyncResponse,
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

#[tokio::test]
async fn test_ui_api_config_hotswap() {
    let temp_a = tempdir().unwrap();
    let vault_a = temp_a.path().to_path_buf();
    setup_mock_vault(&vault_a);

    let temp_b = tempdir().unwrap();
    let vault_b = temp_b.path().to_path_buf();
    let proj_b = vault_b.join("10_Projects");
    fs::create_dir_all(&proj_b).unwrap();
    fs::write(
        proj_b.join("Beta.md"),
        "---\ntitle: Project Beta\nstatus: active\n---\n# Beta Note\n\nContent for Beta project",
    )
    .unwrap();

    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_a.clone(), storage, None);
    let app = create_router(state);

    // Initial sync on vault A
    let sync_req = Request::builder()
        .method("POST")
        .uri("/api/sync")
        .body(axum::body::Body::empty())
        .unwrap();
    let sync_res = app.clone().oneshot(sync_req).await.unwrap();
    assert_eq!(sync_res.status(), StatusCode::OK);

    // Verify status reflects vault A
    let status_req = Request::builder()
        .uri("/api/status")
        .body(axum::body::Body::empty())
        .unwrap();
    let status_res = app.clone().oneshot(status_req).await.unwrap();
    let status_body = to_bytes(status_res.into_body(), usize::MAX).await.unwrap();
    let status: StatusResponse = serde_json::from_slice(&status_body).unwrap();
    assert_eq!(status.vault_path, vault_a.to_string_lossy());
    assert_eq!(status.total_documents, 2);

    // Search for Alpha in vault A
    let search_a = Request::builder()
        .uri("/api/search?q=Alpha&mode=hybrid")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_a = app.clone().oneshot(search_a).await.unwrap();
    let body_a = to_bytes(res_a.into_body(), usize::MAX).await.unwrap();
    let results_a: Vec<k0maru::storage::SearchResult> = serde_json::from_slice(&body_a).unwrap();
    assert!(!results_a.is_empty());

    // GET /api/config
    let get_cfg_req = Request::builder()
        .uri("/api/config")
        .body(axum::body::Body::empty())
        .unwrap();
    let get_cfg_res = app.clone().oneshot(get_cfg_req).await.unwrap();
    assert_eq!(get_cfg_res.status(), StatusCode::OK);
    let cfg_body = to_bytes(get_cfg_res.into_body(), usize::MAX).await.unwrap();
    let cfg: ConfigResponse = serde_json::from_slice(&cfg_body).unwrap();
    assert_eq!(cfg.active_vault_path, vault_a.to_string_lossy());

    // POST /api/config hot-swapping to vault B
    let update_body = serde_json::json!({
        "vault_path": vault_b.to_string_lossy().to_string(),
        "default_search_mode": "bm25"
    });
    let post_cfg_req = Request::builder()
        .method("POST")
        .uri("/api/config")
        .header(CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&update_body).unwrap(),
        ))
        .unwrap();
    let post_cfg_res = app.clone().oneshot(post_cfg_req).await.unwrap();
    assert_eq!(post_cfg_res.status(), StatusCode::OK);

    // Verify /api/status now immediately reflects vault B
    let status_b_req = Request::builder()
        .uri("/api/status")
        .body(axum::body::Body::empty())
        .unwrap();
    let status_b_res = app.clone().oneshot(status_b_req).await.unwrap();
    let status_b_body = to_bytes(status_b_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let status_b: StatusResponse = serde_json::from_slice(&status_b_body).unwrap();
    assert_eq!(
        status_b.vault_path,
        vault_b.canonicalize().unwrap().to_string_lossy()
    );
    assert_eq!(status_b.total_documents, 1);

    // Verify /api/search immediately finds Beta
    let search_b = Request::builder()
        .uri("/api/search?q=Beta&mode=bm25")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_b = app.clone().oneshot(search_b).await.unwrap();
    let body_b = to_bytes(res_b.into_body(), usize::MAX).await.unwrap();
    let results_b: Vec<k0maru::storage::SearchResult> = serde_json::from_slice(&body_b).unwrap();
    assert_eq!(results_b.len(), 1);
    assert!(results_b[0].path.contains("Beta"));

    // Alpha is no longer in vault B's index
    let search_a2 = Request::builder()
        .uri("/api/search?q=Alpha&mode=bm25")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_a2 = app.clone().oneshot(search_a2).await.unwrap();
    let body_a2 = to_bytes(res_a2.into_body(), usize::MAX).await.unwrap();
    let results_a2: Vec<k0maru::storage::SearchResult> = serde_json::from_slice(&body_a2).unwrap();
    assert!(results_a2.is_empty());
}

#[tokio::test]
async fn test_ui_api_vault_bootstrap() {
    let repo_temp = tempdir().unwrap();
    let repo_path = repo_temp.path().to_path_buf();

    // Create a mock repo
    fs::write(
        repo_path.join("Cargo.toml"),
        "[package]\nname = \"demo-service\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let src_dir = repo_path.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("main.rs"), "fn main() {}\n").unwrap();

    let init_temp = tempdir().unwrap();
    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(init_temp.path().to_path_buf(), storage, None);
    let app = create_router(state);

    let bootstrap_payload = serde_json::json!({
        "path": repo_path.to_string_lossy().to_string(),
        "project_name": "Demo Service",
        "install_hooks": false
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/vault/bootstrap")
        .header(CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&bootstrap_payload).unwrap(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp["status"], "ok");
    assert_eq!(resp["project_slug"], "demo-service");

    // Verify status has hot-swapped
    let status_req = Request::builder()
        .uri("/api/status")
        .body(axum::body::Body::empty())
        .unwrap();
    let status_res = app.clone().oneshot(status_req).await.unwrap();
    let status_body = to_bytes(status_res.into_body(), usize::MAX).await.unwrap();
    let status: StatusResponse = serde_json::from_slice(&status_body).unwrap();
    assert!(status.vault_path.contains("demo-service") || status.vault_path.contains(".k0maru"));
}

#[tokio::test]
async fn test_ui_api_note_inspection_and_traversal_guard() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    setup_mock_vault(&vault_path);

    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path.clone(), storage, None);
    let app = create_router(state);

    // Initial sync
    let sync_req = Request::builder()
        .method("POST")
        .uri("/api/sync")
        .body(axum::body::Body::empty())
        .unwrap();
    app.clone().oneshot(sync_req).await.unwrap();

    // 1. Valid note inspection
    let note_req = Request::builder()
        .uri("/api/note?path=10_Projects/Alpha.md")
        .body(axum::body::Body::empty())
        .unwrap();
    let note_res = app.clone().oneshot(note_req).await.unwrap();
    assert_eq!(note_res.status(), StatusCode::OK);
    let note_body = to_bytes(note_res.into_body(), usize::MAX).await.unwrap();
    let note_json: NoteResponse = serde_json::from_slice(&note_body).unwrap();
    assert_eq!(note_json.title, "Project Alpha");
    assert_eq!(note_json.relative_path, "10_Projects/Alpha.md");
    assert_eq!(note_json.status, "active");
    assert!(note_json
        .outgoing_links
        .contains(&"Architecture".to_string()));

    // 2. Reject path traversal with ..
    let traversal_req = Request::builder()
        .uri("/api/note?path=../Cargo.toml")
        .body(axum::body::Body::empty())
        .unwrap();
    let traversal_res = app.clone().oneshot(traversal_req).await.unwrap();
    assert_eq!(traversal_res.status(), StatusCode::BAD_REQUEST);

    // 3. Reject non-markdown file
    let non_md_req = Request::builder()
        .uri("/api/note?path=10_Projects/Alpha.txt")
        .body(axum::body::Body::empty())
        .unwrap();
    let non_md_res = app.clone().oneshot(non_md_req).await.unwrap();
    assert_eq!(non_md_res.status(), StatusCode::BAD_REQUEST);

    // 4. Reject absolute path
    let abs_req = Request::builder()
        .uri("/api/note?path=/etc/passwd")
        .body(axum::body::Body::empty())
        .unwrap();
    let abs_res = app.clone().oneshot(abs_req).await.unwrap();
    assert_eq!(abs_res.status(), StatusCode::BAD_REQUEST);

    // 5. 404 for missing note
    let missing_req = Request::builder()
        .uri("/api/note?path=10_Projects/NonExistent.md")
        .body(axum::body::Body::empty())
        .unwrap();
    let missing_res = app.clone().oneshot(missing_req).await.unwrap();
    assert_eq!(missing_res.status(), StatusCode::NOT_FOUND);

    // 6. Symlink escape check
    let outside_temp = tempdir().unwrap();
    let secret_file = outside_temp.path().join("Secret.md");
    fs::write(&secret_file, "# Secret Outside Vault\n").unwrap();

    #[cfg(unix)]
    {
        let symlink_path = vault_path.join("10_Projects").join("Escaped.md");
        let _ = std::os::unix::fs::symlink(&secret_file, &symlink_path);
        if symlink_path.exists() {
            let esc_req = Request::builder()
                .uri("/api/note?path=10_Projects/Escaped.md")
                .body(axum::body::Body::empty())
                .unwrap();
            let esc_res = app.clone().oneshot(esc_req).await.unwrap();
            assert!(
                esc_res.status() == StatusCode::FORBIDDEN
                    || esc_res.status() == StatusCode::BAD_REQUEST,
                "Expected 403 or 400 for symlink escaping vault root, got {}",
                esc_res.status()
            );
        }
    }
}

#[tokio::test]
async fn test_ui_api_guard_rules_and_diff_check() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    let cards_dir = vault_path.join("20_Cards");
    fs::create_dir_all(&cards_dir).unwrap();

    let adr_content = r#"---
title: ADR-001 Zero Raw Pointers
status: active
---
# ADR-001 Zero Raw Pointers

## Non-Negotiable Invariants
- [rule-no-raw-pointers] (deny) [*.rs] forbidden: "std::ptr::null" | remediation: "Use Option<&T> instead of raw null pointers"
"#;
    fs::write(cards_dir.join("adr-001.md"), adr_content).unwrap();

    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path.clone(), storage, None);
    let app = create_router(state);

    // Initial sync
    let sync_req = Request::builder()
        .method("POST")
        .uri("/api/sync")
        .body(axum::body::Body::empty())
        .unwrap();
    app.clone().oneshot(sync_req).await.unwrap();

    // GET /api/guard/rules
    let rules_req = Request::builder()
        .uri("/api/guard/rules")
        .body(axum::body::Body::empty())
        .unwrap();
    let rules_res = app.clone().oneshot(rules_req).await.unwrap();
    assert_eq!(rules_res.status(), StatusCode::OK);
    let rules_body = to_bytes(rules_res.into_body(), usize::MAX).await.unwrap();
    let rules_json: serde_json::Value = serde_json::from_slice(&rules_body).unwrap();
    assert!(rules_json["total"].as_u64().unwrap_or(0) >= 1);

    // POST /api/guard/check with clean diff
    let clean_diff = r#"--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,3 @@
+pub fn safe_function() -> Option<u32> { Some(42) }
"#;
    let clean_payload = serde_json::json!({ "diff": clean_diff });
    let clean_req = Request::builder()
        .method("POST")
        .uri("/api/guard/check")
        .header(CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&clean_payload).unwrap(),
        ))
        .unwrap();
    let clean_res = app.clone().oneshot(clean_req).await.unwrap();
    assert_eq!(clean_res.status(), StatusCode::OK);
    let clean_body = to_bytes(clean_res.into_body(), usize::MAX).await.unwrap();
    let clean_json: serde_json::Value = serde_json::from_slice(&clean_body).unwrap();
    assert_eq!(clean_json["is_clean"], true);
    assert_eq!(clean_json["violations"].as_array().unwrap().len(), 0);

    // POST /api/guard/check with violating diff
    let violating_diff = r#"--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,3 @@
+let ptr = std::ptr::null();
"#;
    let bad_payload = serde_json::json!({ "diff": violating_diff });
    let bad_req = Request::builder()
        .method("POST")
        .uri("/api/guard/check")
        .header(CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&bad_payload).unwrap(),
        ))
        .unwrap();
    let bad_res = app.clone().oneshot(bad_req).await.unwrap();
    assert_eq!(bad_res.status(), StatusCode::OK);
    let bad_body = to_bytes(bad_res.into_body(), usize::MAX).await.unwrap();
    let bad_json: serde_json::Value = serde_json::from_slice(&bad_body).unwrap();
    assert_eq!(bad_json["is_clean"], false);
    let violations = bad_json["violations"].as_array().unwrap();
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0]["rule_id"], "rule-no-raw-pointers");
}

#[tokio::test]
async fn test_ui_api_cors_security_origin_header() {
    let temp = tempdir().unwrap();
    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(temp.path().to_path_buf(), storage, None);
    let app = create_router(state);

    // 1. Cross-origin request from an external site must be rejected with 403 Forbidden
    let evil_payload = serde_json::json!({ "default_search_mode": "bm25" });
    let evil_req = Request::builder()
        .method("POST")
        .uri("/api/config")
        .header(CONTENT_TYPE, "application/json")
        .header("Origin", "https://evil.example.com")
        .body(axum::body::Body::from(
            serde_json::to_vec(&evil_payload).unwrap(),
        ))
        .unwrap();
    let evil_res = app.clone().oneshot(evil_req).await.unwrap();
    assert_eq!(evil_res.status(), StatusCode::FORBIDDEN);

    // 2. Same-origin request from localhost is allowed
    let local_req = Request::builder()
        .method("POST")
        .uri("/api/config")
        .header(CONTENT_TYPE, "application/json")
        .header("Origin", "http://localhost:3000")
        .body(axum::body::Body::from(
            serde_json::to_vec(&evil_payload).unwrap(),
        ))
        .unwrap();
    let local_res = app.clone().oneshot(local_req).await.unwrap();
    assert_eq!(local_res.status(), StatusCode::OK);

    // 3. Same-origin request from 127.0.0.1 is allowed
    let ip_req = Request::builder()
        .method("POST")
        .uri("/api/config")
        .header(CONTENT_TYPE, "application/json")
        .header("Origin", "http://127.0.0.1:8080")
        .body(axum::body::Body::from(
            serde_json::to_vec(&evil_payload).unwrap(),
        ))
        .unwrap();
    let ip_res = app.clone().oneshot(ip_req).await.unwrap();
    assert_eq!(ip_res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_ui_api_timeline_and_clients() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    setup_mock_vault(&vault_path);

    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path.clone(), storage, None);
    let app = create_router(state);

    let sync_req = Request::builder()
        .method("POST")
        .uri("/api/sync")
        .body(axum::body::Body::empty())
        .unwrap();
    app.clone().oneshot(sync_req).await.unwrap();

    // Timeline endpoint
    let timeline_req = Request::builder()
        .uri("/api/timeline?window=10")
        .body(axum::body::Body::empty())
        .unwrap();
    let timeline_res = app.clone().oneshot(timeline_req).await.unwrap();
    assert_eq!(timeline_res.status(), StatusCode::OK);
    let timeline_body = to_bytes(timeline_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let timeline_json: serde_json::Value = serde_json::from_slice(&timeline_body).unwrap();
    assert!(timeline_json["total"].as_u64().unwrap_or(0) >= 1);

    // Clients endpoint
    let clients_req = Request::builder()
        .uri("/api/clients")
        .body(axum::body::Body::empty())
        .unwrap();
    let clients_res = app.clone().oneshot(clients_req).await.unwrap();
    assert_eq!(clients_res.status(), StatusCode::OK);
    let clients_body = to_bytes(clients_res.into_body(), usize::MAX).await.unwrap();
    let clients_json: serde_json::Value = serde_json::from_slice(&clients_body).unwrap();
    assert!(clients_json["clients"].as_array().is_some());
}

#[test]
fn test_forbidden_system_vault_paths() {
    assert!(is_forbidden_system_vault_path(Path::new("/")));
    assert!(is_forbidden_system_vault_path(Path::new("/etc")));
    assert!(is_forbidden_system_vault_path(Path::new("/usr")));
    assert!(is_forbidden_system_vault_path(Path::new("/bin")));
    assert!(is_forbidden_system_vault_path(Path::new("/sbin")));
    assert!(is_forbidden_system_vault_path(Path::new("/var")));
    assert!(is_forbidden_system_vault_path(Path::new("")));

    if let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        assert!(is_forbidden_system_vault_path(&home));
    }

    let temp = tempdir().unwrap();
    assert!(!is_forbidden_system_vault_path(temp.path()));
}
