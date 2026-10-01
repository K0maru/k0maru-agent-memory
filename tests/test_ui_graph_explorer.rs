//! Integration tests for WikiLinks Graph Explorer and Hierarchy Visualizer (Ticket 22).

use std::fs;
use std::path::Path;

use axum::body::to_bytes;
use axum::http::{Request, StatusCode};
use tempfile::tempdir;
use tower::ServiceExt;

use k0maru::storage::SqliteStorage;
use k0maru::ui::routes::{create_router, AppState, DashboardAssets, GraphResponse};

fn setup_hierarchy_mock_vault(vault_dir: &Path) {
    let proj_dir = vault_dir.join("10_Projects");
    let cards_dir = vault_dir.join("20_Cards");
    let daily_dir = vault_dir.join("00_Daily");
    let resources_dir = vault_dir.join("05_Resources");
    let logs_dir = vault_dir.join("01_AI_Logs");

    fs::create_dir_all(&proj_dir).unwrap();
    fs::create_dir_all(&cards_dir).unwrap();
    fs::create_dir_all(&daily_dir).unwrap();
    fs::create_dir_all(&resources_dir).unwrap();
    fs::create_dir_all(&logs_dir).unwrap();

    // 1. Project note (Project / L3Evergreen) with outlinks to Architecture and Database
    let project_content = r#"---
title: Project Alpha
tags: [project, core]
---
# Project Alpha
Architecture design [[Architecture]] and relational storage [[Database]].
"#;
    fs::write(proj_dir.join("Alpha.md"), project_content).unwrap();

    // 2. Evergreen Card (L3) with backlink to Alpha
    let card_content = r#"---
title: System Architecture
tags: [arch, principle]
---
# System Architecture
Permanent evergreen architectural pattern referenced by [[Project Alpha]].
"#;
    fs::write(cards_dir.join("Architecture.md"), card_content).unwrap();

    // 3. AI Log (L2)
    let log_content = r#"---
title: Evaluation Run 42
tags: [eval, trace]
---
# Evaluation Run 42
Performance evaluation trace for [[Project Alpha]].
"#;
    fs::write(logs_dir.join("EvalRun.md"), log_content).unwrap();

    // 4. Resource note (L1) - Orphan note with 0 links
    let resource_content = r#"---
title: Rust Reference Manual
tags: [reference, manual]
---
# Rust Reference Manual
Standard cheatsheet documentation with zero links.
"#;
    fs::write(resources_dir.join("Cheatsheet.md"), resource_content).unwrap();

    // 5. Daily note (L0) - Orphan note with 0 links
    let daily_content = r#"---
title: Daily 2026-10-01
tags: [daily, standup]
---
# Daily 2026-10-01
Quick daily scratchpad notes.
"#;
    fs::write(daily_dir.join("2026-10-01.md"), daily_content).unwrap();
}

#[test]
fn test_embedded_bundle_graph_explorer_elements() {
    let index_file = DashboardAssets::get("index.html").expect("index.html must exist");
    let index_html = std::str::from_utf8(&index_file.data).expect("valid utf-8");

    // Graph canvas element
    assert!(
        index_html.contains("id=\"graph-canvas\""),
        "index.html must contain #graph-canvas"
    );

    // Side inspector drawer (#graph-inspector or #graph-drawer)
    assert!(
        index_html.contains("id=\"graph-inspector\"") || index_html.contains("id=\"graph-drawer\""),
        "index.html must contain #graph-inspector or #graph-drawer"
    );

    // Search and filter controls
    assert!(
        index_html.contains("id=\"graph-search\""),
        "index.html must contain #graph-search input"
    );
    assert!(
        index_html.contains("id=\"toggle-orphans\""),
        "index.html must contain #toggle-orphans switch"
    );
    assert!(
        index_html.contains("id=\"graph-counter\""),
        "index.html must contain #graph-counter pill"
    );

    // Hierarchy filter pills
    assert!(
        index_html.contains("Project"),
        "index.html must contain Project hierarchy toggle"
    );
    assert!(
        index_html.contains("L3 Evergreen"),
        "index.html must contain L3 Evergreen hierarchy toggle"
    );
    assert!(
        index_html.contains("L2 Log"),
        "index.html must contain L2 Log hierarchy toggle"
    );
    assert!(
        index_html.contains("L1 Resource"),
        "index.html must contain L1 Resource hierarchy toggle"
    );
    assert!(
        index_html.contains("L0 Daily"),
        "index.html must contain L0 Daily hierarchy toggle"
    );

    // Zoom controls
    assert!(
        index_html.contains("btn-zoom-in"),
        "index.html must contain zoom in button"
    );
    assert!(
        index_html.contains("btn-zoom-out"),
        "index.html must contain zoom out button"
    );
    assert!(
        index_html.contains("btn-zoom-fit"),
        "index.html must contain zoom fit button"
    );

    // Drawer sections
    assert!(
        index_html.contains("drawer-outlinks") || index_html.contains("drawer-links"),
        "index.html must contain drawer out-links section"
    );
    assert!(
        index_html.contains("drawer-backlinks"),
        "index.html must contain drawer backlinks section"
    );

    // Style bundle checks
    let style_file = DashboardAssets::get("style.css").expect("style.css must exist");
    let style_css = std::str::from_utf8(&style_file.data).expect("valid utf-8");

    assert!(
        style_css.contains("graph-canvas") || style_css.contains("graph-viewport"),
        "style.css must contain graph canvas or viewport styling"
    );
    assert!(
        style_css.contains("graph-drawer") || style_css.contains("graph-inspector"),
        "style.css must contain drawer styling"
    );
    assert!(
        style_css.contains("backdrop-filter"),
        "style.css must contain floating blur backdrop filter"
    );

    // App JS bundle checks
    let js_file = DashboardAssets::get("app.js").expect("app.js must exist");
    let js_code = std::str::from_utf8(&js_file.data).expect("valid utf-8");

    assert!(
        js_code.contains("/api/graph"),
        "app.js must fetch /api/graph"
    );
    assert!(
        js_code.contains("graph-canvas"),
        "app.js must reference graph-canvas"
    );
}

#[test]
fn test_graph_tab_and_panel_temporarily_hidden_in_ui() {
    let style_file = DashboardAssets::get("style.css").expect("style.css must exist");
    let style_css = std::str::from_utf8(&style_file.data).expect("valid utf-8");

    // Verify #tab-btn-graph and #panel-graph are hidden via CSS
    assert!(
        style_css.contains("#tab-btn-graph"),
        "style.css must contain #tab-btn-graph selector"
    );
    assert!(
        style_css.contains("#panel-graph"),
        "style.css must contain #panel-graph selector"
    );
    assert!(
        style_css.contains("display: none"),
        "style.css must contain display: none rule"
    );

    // Verify app.js fallback for graph tab routing
    let js_file = DashboardAssets::get("app.js").expect("app.js must exist");
    let js_code = std::str::from_utf8(&js_file.data).expect("valid utf-8");

    assert!(
        js_code.contains("target === 'graph'") || js_code.contains("tabId === 'graph'"),
        "app.js must check for graph tab redirection"
    );
    assert!(
        js_code.contains("isGraphPanelVisible"),
        "app.js must guard simulation loops when graph panel is hidden"
    );
}

#[tokio::test]
async fn test_api_graph_payload_with_hierarchy_and_tags() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    setup_hierarchy_mock_vault(&vault_path);

    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path.clone(), storage, None);
    let app = create_router(state);

    // 1. Sync vault to populate documents, links, and tags
    let sync_req = Request::builder()
        .method("POST")
        .uri("/api/sync")
        .body(axum::body::Body::empty())
        .unwrap();
    let sync_res = app.clone().oneshot(sync_req).await.unwrap();
    assert_eq!(sync_res.status(), StatusCode::OK);

    // 2. Query GET /api/graph
    let req = Request::builder()
        .uri("/api/graph")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let graph: GraphResponse = serde_json::from_slice(&body).unwrap();

    // Verify all 5 documents are present as graph nodes
    assert_eq!(graph.nodes.len(), 5, "Expected 5 documents in graph");

    // Verify Project note
    let proj_node = graph
        .nodes
        .iter()
        .find(|n| n.title == "Project Alpha")
        .expect("Project Alpha must be in graph");
    assert!(
        proj_node.id.contains("Alpha.md"),
        "Project id must reference file"
    );
    assert!(proj_node.tags.contains(&"project".to_string()));
    assert!(proj_node.tags.contains(&"core".to_string()));

    // Verify L3 Evergreen card
    let card_node = graph
        .nodes
        .iter()
        .find(|n| n.title == "System Architecture")
        .expect("System Architecture must be in graph");
    assert_eq!(card_node.hierarchy, "L3Evergreen");
    assert!(card_node.tags.contains(&"arch".to_string()));

    // Verify L2 Log note
    let log_node = graph
        .nodes
        .iter()
        .find(|n| n.title == "Evaluation Run 42")
        .expect("Evaluation Run 42 must be in graph");
    assert_eq!(log_node.hierarchy, "L2Log");
    assert!(log_node.tags.contains(&"eval".to_string()));

    // Verify L1 Resource note
    let res_node = graph
        .nodes
        .iter()
        .find(|n| n.title == "Rust Reference Manual")
        .expect("Rust Reference Manual must be in graph");
    assert_eq!(res_node.hierarchy, "L1Resource");
    assert!(res_node.tags.contains(&"manual".to_string()));

    // Verify L0 Daily note
    let daily_node = graph
        .nodes
        .iter()
        .find(|n| n.title == "Daily 2026-10-01")
        .expect("Daily note must be in graph");
    assert_eq!(daily_node.hierarchy, "L0Ephemeral");
    assert!(daily_node.tags.contains(&"daily".to_string()));

    // Verify edges
    assert!(
        !graph.edges.is_empty(),
        "Graph edges should be populated from WikiLinks"
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.source.contains("Alpha.md") && e.target == "Architecture"),
        "Alpha should link to Architecture"
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.source.contains("Alpha.md") && e.target == "Database"),
        "Alpha should link to Database"
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.source.contains("Architecture.md") && e.target == "Project Alpha"),
        "Architecture should link to Project Alpha"
    );

    // Verify orphans (Cheatsheet and Daily have 0 incoming and outgoing edges)
    let orphan_ids: Vec<String> = graph
        .nodes
        .iter()
        .filter(|n| {
            let has_out = graph.edges.iter().any(|e| e.source == n.id);
            let has_in = graph
                .edges
                .iter()
                .any(|e| e.target == n.id || e.target == n.title);
            !has_out && !has_in
        })
        .map(|n| n.title.clone())
        .collect();

    assert!(
        orphan_ids.contains(&"Rust Reference Manual".to_string()),
        "Rust Reference Manual must be recognized as an orphan note"
    );
    assert!(
        orphan_ids.contains(&"Daily 2026-10-01".to_string()),
        "Daily note must be recognized as an orphan note"
    );
}
