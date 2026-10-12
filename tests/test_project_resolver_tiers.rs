use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

use k0maru::adapters::ObsidianAdapter;
use k0maru::loadout::LoadoutBuilder;
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::SqliteStorage;
use k0maru::vector::MockEmbeddingEngine;

fn setup_adversarial_vault() -> (TempDir, ObsidianAdapter, SqliteStorage) {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let root = temp_dir.path();

    let projects_dir = root.join("10_Projects");
    let logs_dir = root.join("01_AI_Logs");
    let cards_dir = root.join("20_Cards");

    fs::create_dir_all(&projects_dir).unwrap();
    fs::create_dir_all(&logs_dir).unwrap();
    fs::create_dir_all(&cards_dir).unwrap();

    // 1. Target project note in Chinese with English tags and slug
    let project_content = r#"---
title: OKX资金费率套利与跨生态对冲系统
status: active
slug: okx-funding-arbitrage
aliases:
  - funding-arbitrage
  - okx-arbitrage
tags:
  - topic/funding-arbitrage
---
# OKX资金费率套利与跨生态对冲系统

> 跨交易所与去中心化资金费率套利与自动调仓引擎。

相关架构设计见 [[20_Cards/ArbitrageArchitecture]]。
"#;
    fs::write(
        projects_dir.join("OKX资金费率套利与跨生态对冲系统.md"),
        project_content,
    )
    .unwrap();

    // 2. Card note
    fs::write(
        cards_dir.join("ArbitrageArchitecture.md"),
        "# Arbitrage Architecture\n## 📌 核心概念\n> 跨生态对冲不变量。",
    )
    .unwrap();

    // 3. Insert 20 daily/AI logs with identical high keyword frequency to clutter global search
    for i in 1..=20 {
        let log_content = format!(
            r#"---
title: 2026-10-{:02}-okx-funding-arbitrage-phase-5-live-tui-monitor
hierarchy: L2Log
---
# 2026-10-{:02}-okx-funding-arbitrage-phase-5-live-tui-monitor

okx funding arbitrage okx funding arbitrage okx funding arbitrage.
Investigating [[OKX资金费率套利与跨生态对冲系统]] funding arbitrage performance.
"#,
            i, i,
        );
        fs::write(
            logs_dir.join(format!(
                "2026-10-{:02}-okx-funding-arbitrage-phase-5-live-tui-monitor.md",
                i
            )),
            log_content,
        )
        .unwrap();
    }

    let adapter = ObsidianAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    (temp_dir, adapter, storage)
}

#[test]
fn test_tier1_normalized_exact_and_slug_and_tag_match() {
    let (_fixture, adapter, storage) = setup_adversarial_vault();
    let builder = LoadoutBuilder::new(&adapter, &storage);

    // Kebab-case slug query matching frontmatter slug
    let found = builder.find_project("okx-funding-arbitrage").unwrap();
    assert_eq!(
        found,
        Some(PathBuf::from(
            "10_Projects/OKX资金费率套利与跨生态对冲系统.md"
        )),
        "Expected okx-funding-arbitrage to resolve to Chinese project file"
    );

    // English alias match
    let found_alias = builder.find_project("funding-arbitrage").unwrap();
    assert_eq!(
        found_alias,
        Some(PathBuf::from(
            "10_Projects/OKX资金费率套利与跨生态对冲系统.md"
        ))
    );

    // Chinese partial keyword match
    let found_cn = builder.find_project("资金费率").unwrap();
    assert_eq!(
        found_cn,
        Some(PathBuf::from(
            "10_Projects/OKX资金费率套利与跨生态对冲系统.md"
        ))
    );

    // Exact stem match
    let found_exact = builder
        .find_project("OKX资金费率套利与跨生态对冲系统")
        .unwrap();
    assert_eq!(
        found_exact,
        Some(PathBuf::from(
            "10_Projects/OKX资金费率套利与跨生态对冲系统.md"
        ))
    );
}

#[test]
fn test_tier2_scoped_fts_prevents_log_clutter_truncation() {
    let temp_dir = TempDir::new().expect("Failed to create tempdir");
    let root = temp_dir.path();
    let projects_dir = root.join("10_Projects");
    let logs_dir = root.join("01_AI_Logs");
    fs::create_dir_all(&projects_dir).unwrap();
    fs::create_dir_all(&logs_dir).unwrap();

    // 20 high-frequency logs for term "DerivativeMatrix"
    for i in 1..=20 {
        fs::write(
            logs_dir.join(format!("2026-10-{:02}-run.md", i)),
            format!(
                "DerivativeMatrix DerivativeMatrix DerivativeMatrix DerivativeMatrix run {:02}",
                i
            ),
        )
        .unwrap();
    }

    // 1 project document without aliases, but mentioning DerivativeMatrix in body
    fs::write(
        projects_dir.join("MatrixProject.md"),
        "# Unrelated Project Title\n\nThis project implements the core DerivativeMatrix engine.",
    )
    .unwrap();

    let adapter = ObsidianAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    // In unconstrained global FTS with limit 5, the 20 logs would push MatrixProject out.
    // But with Tier 2 scoped FTS push-down, it resolves MatrixProject!
    let found = builder.find_project("DerivativeMatrix").unwrap();
    assert_eq!(found, Some(PathBuf::from("10_Projects/MatrixProject.md")));
}

#[test]
fn test_tier3_vector_semantic_fallback() {
    let temp_dir = TempDir::new().expect("Failed to create tempdir");
    let root = temp_dir.path();
    let projects_dir = root.join("10_Projects");
    fs::create_dir_all(&projects_dir).unwrap();

    fs::write(
        projects_dir.join("AlgorithmicTrader.md"),
        "# High-Frequency Trading Infrastructure\n\nAutomated statistical market execution framework.",
    )
    .unwrap();

    let adapter = ObsidianAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().unwrap();
    let embedder = Arc::new(MockEmbeddingEngine::new(384));

    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    // Sync vector embeddings
    k0maru::scanner::VectorSyncEngine::sync(scanner.storage_mut(), &*embedder).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage).with_embedder(embedder);

    // Query with semantically related terms matching vector embedding
    let found = builder
        .find_project("Automated statistical market execution")
        .unwrap();
    assert_eq!(
        found,
        Some(PathBuf::from("10_Projects/AlgorithmicTrader.md"))
    );
}

#[test]
fn test_mcp_get_project_loadout_adversarial_resolution() {
    let (fixture, _adapter, _storage) = setup_adversarial_vault();
    let mut server = k0maru::mcp::McpServer::new(fixture.path());

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
            "name": "get_project_loadout",
            "arguments": {
                "project_name": "okx-funding-arbitrage"
            }
        }
    });

    let resp_str = server
        .handle_line(&req.to_string())
        .expect("Should respond to get_project_loadout");
    let resp: serde_json::Value = serde_json::from_str(&resp_str).expect("Valid JSON");

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_ne!(
        resp["result"]["isError"], true,
        "Response was error: {:?}",
        resp
    );

    let content = resp["result"]["content"].as_array().expect("content array");
    let text = content[0]["text"].as_str().expect("text");
    assert!(
        text.contains("OKX资金费率套利与跨生态对冲系统"),
        "Loadout must resolve target project, got: {}",
        text
    );
    assert!(text.contains("ArbitrageArchitecture"));
}
