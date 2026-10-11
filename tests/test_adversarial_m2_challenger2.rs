use std::fs;
use std::process::Command;
use tempfile::TempDir;

use k0maru::adapters::ObsidianAdapter;
use k0maru::git::{FileChangeStatus, GitInspector, ModifiedFile};
use k0maru::loadout::{estimate_tokens, LoadoutBuilder};
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::SqliteStorage;
use k0maru::McpServer;
use serde_json::json;

/// Creates a test vault containing specific ADR cards for storage, vector, mcp, and parser modules.
fn create_test_vault_with_all_module_adrs() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory for test vault");
    let root = temp_dir.path();

    let projects_dir = root.join("10_Projects");
    let cards_dir = root.join("20_Cards");
    let logs_dir = root.join("01_AI_Logs");

    fs::create_dir_all(&projects_dir).unwrap();
    fs::create_dir_all(&cards_dir).unwrap();
    fs::create_dir_all(&logs_dir).unwrap();

    // 1. Storage ADR Card
    let storage_card = r#"---
title: 单文件SQLite缓存契约
hierarchy: L3Evergreen
tags:
  - storage
  - sqlite
  - adr
---
# 单文件SQLite缓存契约

## 📌 核心概念
> Markdown is source of truth; cache.sqlite is disposable and rebuildable in milliseconds.

## Details
Cache file must reside in .k0maru/cache.sqlite.
"#;
    fs::write(cards_dir.join("单文件SQLite缓存契约.md"), storage_card).unwrap();

    // 2. Vector ADR Card
    let vector_card = r#"---
title: 向量检索与ONNX嵌入契约
hierarchy: L3Evergreen
tags:
  - vector
  - embedding
  - adr
---
# 向量检索与ONNX嵌入契约

## 📌 核心概念
> Hybrid search combines BM25 and ONNX vector embeddings via Reciprocal Rank Fusion k=60.

## Details
Zero-copy cosine distance ordering.
"#;
    fs::write(cards_dir.join("向量检索与ONNX嵌入契约.md"), vector_card).unwrap();

    // 3. MCP ADR Card
    let mcp_card = r#"---
title: FastMCP协议与工具契约
hierarchy: L3Evergreen
tags:
  - mcp
  - fastmcp
  - server
  - adr
---
# FastMCP协议与工具契约

## 📌 核心概念
> FastMCP JSON-RPC tools with strictly bounded sub-300-token context loadout payloads.

## Details
Stateless STDIO streaming with NDJSON support.
"#;
    fs::write(cards_dir.join("FastMCP协议与工具契约.md"), mcp_card).unwrap();

    // 4. Parser ADR Card
    let parser_card = r#"---
title: Markdown解析与WikiLinks语法契约
hierarchy: L3Evergreen
tags:
  - parser
  - markdown
  - wikilink
  - ast
  - adr
---
# Markdown解析与WikiLinks语法契约

## 📌 核心概念
> Zero-regex streaming pulldown-cmark parser extracting bare and aliased WikiLinks.

## Details
Strict whitespace trimming and codeblock escaping.
"#;
    fs::write(
        cards_dir.join("Markdown解析与WikiLinks语法契约.md"),
        parser_card,
    )
    .unwrap();

    // 5. General Architecture Card (Baseline Invariant)
    let general_card = r#"---
title: 零守护进程与纯静态二进制
hierarchy: L3Evergreen
tags:
  - architecture
  - invariant
---
# 零守护进程与纯静态二进制

## 📌 核心概念
> Single static binary with zero daemon background processes, compiled under 10MB limit.
"#;
    fs::write(cards_dir.join("零守护进程与纯静态二进制.md"), general_card).unwrap();

    // Project Note linking all rules
    let project_note = r#"---
title: K0maru Agent Memory
status: active
tags:
  - project
  - memory
---
# K0maru Agent Memory

> Cleanroom, zero-daemon agent memory hub mounting Obsidian SecondBrain and LLM-Wiki.

## Linked Rules
- [[20_Cards/零守护进程与纯静态二进制]]
- [[20_Cards/单文件SQLite缓存契约]]
- [[20_Cards/向量检索与ONNX嵌入契约]]
- [[20_Cards/FastMCP协议与工具契约]]
- [[20_Cards/Markdown解析与WikiLinks语法契约]]
"#;
    fs::write(projects_dir.join("k0maru-memory.md"), project_note).unwrap();

    temp_dir
}

// ---------------------------------------------------------------------------
// GROUP 1: FastMCP get_project_loadout with diff: true and branch: main
// ---------------------------------------------------------------------------

#[test]
fn test_mcp_loadout_live_git_repo_with_branch_main() {
    let git_dir = TempDir::new().unwrap();
    let root = git_dir.path();

    let run = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .expect("Failed to run git command");
        assert!(status.success(), "Git command failed: {:?}", args);
    };

    run(&["init"]);
    run(&["config", "user.name", "Challenger 2"]);
    run(&["config", "user.email", "challenger2@example.com"]);

    // Initial commit on main branch
    fs::write(root.join("README.md"), "# Mock Project").unwrap();
    run(&["add", "README.md"]);
    run(&["commit", "-m", "initial commit"]);
    run(&["branch", "-M", "main"]);

    // Create feature branch
    run(&["checkout", "-b", "feature/mcp-upgrade"]);

    // Commit a change to src/mcp/protocol.rs on feature branch
    fs::create_dir_all(root.join("src/mcp")).unwrap();
    fs::write(root.join("src/mcp/protocol.rs"), "// updated protocol").unwrap();
    run(&["add", "src/mcp/protocol.rs"]);
    run(&["commit", "-m", "feature: add mcp protocol"]);

    // Add an unstaged modification to src/storage/sqlite.rs
    fs::create_dir_all(root.join("src/storage")).unwrap();
    fs::write(
        root.join("src/storage/sqlite.rs"),
        "// unstaged sqlite change",
    )
    .unwrap();

    // Inspect diff against main branch
    let changes = GitInspector::inspect_diff(root, Some("main")).unwrap();
    assert!(
        !changes.is_empty(),
        "Expected changes between main and feature branch"
    );

    let paths: Vec<String> = changes
        .iter()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();

    assert!(
        paths.iter().any(|p| p.contains("protocol.rs")),
        "Expected protocol.rs from branch diff: {:?}",
        paths
    );
    assert!(
        paths.iter().any(|p| p.contains("sqlite.rs")),
        "Expected sqlite.rs from working tree diff: {:?}",
        paths
    );

    // Build loadout using these changes against our rich test vault
    let vault = create_test_vault_with_all_module_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let loadout = builder
        .build_with_diff("k0maru-memory", &changes)
        .unwrap()
        .expect("Loadout must be generated");

    assert!(loadout.markdown.contains("[Diff-Aware]"));
    assert!(loadout.markdown.contains("Active Changes"));
    assert!(
        loadout.estimated_tokens < 300,
        "Token budget must be < 300, got {}",
        loadout.estimated_tokens
    );
}

#[test]
fn test_mcp_loadout_empty_git_repo_no_commits() {
    let git_dir = TempDir::new().unwrap();
    let root = git_dir.path();

    let status = Command::new("git")
        .arg("init")
        .current_dir(root)
        .status()
        .expect("Failed to git init");
    assert!(status.success());

    // Branch diff on repo with 0 commits must not crash
    let changes = GitInspector::inspect_diff(root, Some("main")).unwrap();
    // In empty git repo, should return Ok without error
    assert!(changes.is_empty() || !changes.is_empty()); // No panic

    let vault = create_test_vault_with_all_module_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let loadout = builder
        .build_with_diff("k0maru-memory", &changes)
        .unwrap()
        .expect("Loadout must be generated");

    assert!(loadout.estimated_tokens < 300);
}

#[test]
fn test_mcp_loadout_nonexistent_branch_fallback() {
    let git_dir = TempDir::new().unwrap();
    let root = git_dir.path();

    let run = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .expect("Failed to run git command");
        assert!(status.success());
    };

    run(&["init"]);
    run(&["config", "user.name", "Challenger 2"]);
    run(&["config", "user.email", "challenger2@example.com"]);
    fs::write(root.join("base.txt"), "base").unwrap();
    run(&["add", "base.txt"]);
    run(&["commit", "-m", "init"]);

    // Add unstaged file
    fs::write(root.join("unstaged.txt"), "unstaged modification").unwrap();

    // Specify completely bogus branch
    let changes = GitInspector::inspect_diff(root, Some("nonexistent_branch_xyz")).unwrap();

    // Must not fail, must still capture the unstaged file from working tree
    let paths: Vec<String> = changes
        .iter()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();
    assert!(
        paths.iter().any(|p| p.contains("unstaged.txt")),
        "Expected working tree unstaged.txt despite branch failure: {:?}",
        paths
    );
}

#[test]
fn test_mcp_loadout_non_git_repo_graceful() {
    let non_git = TempDir::new().unwrap();
    let root = non_git.path();

    assert!(!GitInspector::is_git_repo(root));
    let changes = GitInspector::inspect_diff(root, Some("main")).unwrap();
    assert!(changes.is_empty());

    let vault = create_test_vault_with_all_module_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let loadout = builder
        .build_with_diff("k0maru-memory", &changes)
        .unwrap()
        .expect("Loadout must be generated");

    assert!(loadout.markdown.contains("Clean working tree"));
    assert!(loadout.estimated_tokens < 300);
}

#[test]
fn test_mcp_json_rpc_schema_and_call_permutations() {
    let vault = create_test_vault_with_all_module_adrs();
    let mut server = McpServer::new(vault.path());

    // 1. Verify tools/list schema exposes diff and branch
    let tools_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    let tools_resp = server.handle_line(&tools_req.to_string()).unwrap();
    let resp_val: serde_json::Value = serde_json::from_str(&tools_resp).unwrap();
    let tools = resp_val["result"]["tools"].as_array().unwrap();
    let tool = tools
        .iter()
        .find(|t| t["name"] == "get_project_loadout")
        .expect("get_project_loadout must be registered");

    let props = &tool["inputSchema"]["properties"];
    assert_eq!(props["project_name"]["type"], "string");
    assert_eq!(props["diff"]["type"], "boolean");
    assert_eq!(props["branch"]["type"], "string");

    // 2. Call with project_name only
    let call_base = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "get_project_loadout",
            "arguments": {
                "project_name": "k0maru-memory"
            }
        }
    });
    let resp_base = server.handle_line(&call_base.to_string()).unwrap();
    let val_base: serde_json::Value = serde_json::from_str(&resp_base).unwrap();
    assert!(val_base["error"].is_null());
    let md_base = val_base["result"]["content"][0]["text"].as_str().unwrap();
    assert!(estimate_tokens(md_base) < 300);

    // 3. Call with diff: true and branch: "main"
    let call_diff = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "get_project_loadout",
            "arguments": {
                "project_name": "k0maru-memory",
                "diff": true,
                "branch": "main"
            }
        }
    });
    let resp_diff = server.handle_line(&call_diff.to_string()).unwrap();
    let val_diff: serde_json::Value = serde_json::from_str(&resp_diff).unwrap();
    assert!(val_diff["error"].is_null());
    let md_diff = val_diff["result"]["content"][0]["text"].as_str().unwrap();
    assert!(md_diff.contains("[Diff-Aware]"));
    assert!(estimate_tokens(md_diff) < 300);

    // 4. Call with missing project_name parameter (must return error)
    let call_missing = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "get_project_loadout",
            "arguments": {
                "diff": true
            }
        }
    });
    let resp_missing = server.handle_line(&call_missing.to_string()).unwrap();
    let val_missing: serde_json::Value = serde_json::from_str(&resp_missing).unwrap();
    assert_eq!(
        val_missing["result"]["isError"], true,
        "Missing project_name must set isError: true: {:?}",
        val_missing
    );
    let err_msg = val_missing["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(err_msg.contains("project_name"));

    // 5. Call with non-existent project name
    let call_not_found = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "get_project_loadout",
            "arguments": {
                "project_name": "nonexistent_project_9999",
                "diff": true
            }
        }
    });
    let resp_not_found = server.handle_line(&call_not_found.to_string()).unwrap();
    let val_not_found: serde_json::Value = serde_json::from_str(&resp_not_found).unwrap();
    let text_not_found = val_not_found["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        text_not_found.contains("not found") || text_not_found.contains("未找到"),
        "Expected not found message, got: {}",
        text_not_found
    );
}

// ---------------------------------------------------------------------------
// GROUP 2: Module keyword extraction & ADR activation (storage, vector, mcp, parser)
// ---------------------------------------------------------------------------

#[test]
fn test_keyword_extraction_accuracy_all_modules() {
    // 1. Storage
    let storage_file = ModifiedFile::new(
        "crates/k0maru-core/src/storage/sqlite.rs",
        FileChangeStatus::Modified,
    );
    let kw_storage = storage_file.extract_keywords();
    assert!(
        kw_storage.contains(&"storage".to_string()),
        "Keywords must contain 'storage': {:?}",
        kw_storage
    );
    assert!(
        kw_storage.contains(&"sqlite".to_string()),
        "Keywords must contain 'sqlite': {:?}",
        kw_storage
    );

    // 2. Vector
    let vector_file = ModifiedFile::new(
        "crates/k0maru-core/src/vector/embedding.rs",
        FileChangeStatus::Modified,
    );
    let kw_vector = vector_file.extract_keywords();
    assert!(
        kw_vector.contains(&"vector".to_string()),
        "Keywords must contain 'vector': {:?}",
        kw_vector
    );
    assert!(
        kw_vector.contains(&"embedding".to_string()),
        "Keywords must contain 'embedding': {:?}",
        kw_vector
    );

    // 3. MCP
    let mcp_file = ModifiedFile::new("src/mcp/server.rs", FileChangeStatus::Modified);
    let kw_mcp = mcp_file.extract_keywords();
    assert!(
        kw_mcp.contains(&"mcp".to_string()),
        "Keywords must contain 'mcp': {:?}",
        kw_mcp
    );
    assert!(
        kw_mcp.contains(&"server".to_string()),
        "Keywords must contain 'server': {:?}",
        kw_mcp
    );

    // 4. Parser
    let parser_file = ModifiedFile::new(
        "crates/k0maru-core/src/parser/markdown.rs",
        FileChangeStatus::Modified,
    );
    let kw_parser = parser_file.extract_keywords();
    assert!(
        kw_parser.contains(&"parser".to_string()),
        "Keywords must contain 'parser': {:?}",
        kw_parser
    );
    assert!(
        kw_parser.contains(&"markdown".to_string()),
        "Keywords must contain 'markdown': {:?}",
        kw_parser
    );
}

#[test]
fn test_adr_activation_accuracy_for_storage() {
    let vault = create_test_vault_with_all_module_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let modified = vec![ModifiedFile::new(
        "crates/k0maru-core/src/storage/sqlite.rs",
        FileChangeStatus::Modified,
    )];

    let loadout = builder
        .build_with_diff("k0maru-memory", &modified)
        .unwrap()
        .expect("Loadout found");

    assert!(!loadout.l3_cards.is_empty());
    assert!(
        loadout.l3_cards[0].name.contains("SQLite") || loadout.l3_cards[0].name.contains("storage"),
        "Top activated card must be Storage ADR, got: {:?}",
        loadout.l3_cards[0].name
    );
    assert!(loadout.estimated_tokens < 300);
}

#[test]
fn test_adr_activation_accuracy_for_vector() {
    let vault = create_test_vault_with_all_module_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let modified = vec![ModifiedFile::new(
        "crates/k0maru-core/src/vector/embedding.rs",
        FileChangeStatus::Modified,
    )];

    let loadout = builder
        .build_with_diff("k0maru-memory", &modified)
        .unwrap()
        .expect("Loadout found");

    assert!(!loadout.l3_cards.is_empty());
    assert!(
        loadout.l3_cards[0].name.contains("向量") || loadout.l3_cards[0].name.contains("vector"),
        "Top activated card must be Vector ADR, got: {:?}",
        loadout.l3_cards[0].name
    );
    assert!(loadout.estimated_tokens < 300);
}

#[test]
fn test_adr_activation_accuracy_for_mcp() {
    let vault = create_test_vault_with_all_module_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let modified = vec![ModifiedFile::new(
        "src/mcp/server.rs",
        FileChangeStatus::Modified,
    )];

    let loadout = builder
        .build_with_diff("k0maru-memory", &modified)
        .unwrap()
        .expect("Loadout found");

    assert!(!loadout.l3_cards.is_empty());
    assert!(
        loadout.l3_cards[0].name.contains("FastMCP") || loadout.l3_cards[0].name.contains("mcp"),
        "Top activated card must be FastMCP ADR, got: {:?}",
        loadout.l3_cards[0].name
    );
    assert!(loadout.estimated_tokens < 300);
}

#[test]
fn test_adr_activation_accuracy_for_parser() {
    let vault = create_test_vault_with_all_module_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let modified = vec![ModifiedFile::new(
        "crates/k0maru-core/src/parser/markdown.rs",
        FileChangeStatus::Modified,
    )];

    let loadout = builder
        .build_with_diff("k0maru-memory", &modified)
        .unwrap()
        .expect("Loadout found");

    assert!(!loadout.l3_cards.is_empty());
    assert!(
        loadout.l3_cards[0].name.contains("Markdown解析")
            || loadout.l3_cards[0].name.contains("parser"),
        "Top activated card must be Parser ADR, got: {:?}",
        loadout.l3_cards[0].name
    );
    assert!(loadout.estimated_tokens < 300);
}

#[test]
fn test_joint_multi_module_activation_and_greedy_budget() {
    let vault = create_test_vault_with_all_module_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    // Modify files across all 4 modules simultaneously
    let modified = vec![
        ModifiedFile::new(
            "crates/k0maru-core/src/storage/sqlite.rs",
            FileChangeStatus::Modified,
        ),
        ModifiedFile::new(
            "crates/k0maru-core/src/vector/embedding.rs",
            FileChangeStatus::Modified,
        ),
        ModifiedFile::new("src/mcp/server.rs", FileChangeStatus::Modified),
        ModifiedFile::new(
            "crates/k0maru-core/src/parser/markdown.rs",
            FileChangeStatus::Modified,
        ),
    ];

    let loadout = builder
        .build_with_diff("k0maru-memory", &modified)
        .unwrap()
        .expect("Loadout found");

    // Must pack up to 4 cards without exceeding budget
    assert!(
        loadout.l3_cards.len() >= 3,
        "Should pack multiple relevant cards, got {}",
        loadout.l3_cards.len()
    );

    // Must stay strictly below 300 tokens
    assert!(
        loadout.estimated_tokens < 300,
        "Token budget exceeded: {}",
        loadout.estimated_tokens
    );
    assert!(
        estimate_tokens(&loadout.markdown) < 300,
        "Markdown token budget exceeded: {}",
        estimate_tokens(&loadout.markdown)
    );
}

#[test]
fn test_stress_500_modified_files_strict_budget() {
    let vault = create_test_vault_with_all_module_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    // 500 files modified across 100 subsystems
    let mut modified = Vec::new();
    for i in 0..500 {
        modified.push(ModifiedFile::new(
            format!("crates/subsystem_{i}/deeply/nested/component_{i}.rs"),
            FileChangeStatus::Modified,
        ));
    }

    let loadout = builder
        .build_with_diff("k0maru-memory", &modified)
        .unwrap()
        .expect("Loadout found");

    assert!(
        loadout.estimated_tokens < 300,
        "500-file diff exceeded 300 tokens: {}",
        loadout.estimated_tokens
    );
    assert!(
        estimate_tokens(&loadout.markdown) < 300,
        "Markdown for 500-file diff exceeded 300 tokens: {}",
        estimate_tokens(&loadout.markdown)
    );
}

#[test]
fn test_mixed_change_statuses_and_renames() {
    let vault = create_test_vault_with_all_module_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    let modified = vec![
        ModifiedFile::new("src/storage/old_sqlite.rs", FileChangeStatus::Deleted),
        ModifiedFile::new("src/storage/new_sqlite.rs", FileChangeStatus::Added),
        ModifiedFile::new("src/mcp/handler.rs", FileChangeStatus::Renamed),
        ModifiedFile::new("src/vector/index.rs", FileChangeStatus::Untracked),
        ModifiedFile::new(
            "crates/k0maru-core/src/parser/ast.rs",
            FileChangeStatus::Modified,
        ),
    ];

    let loadout = builder
        .build_with_diff("k0maru-memory", &modified)
        .unwrap()
        .expect("Loadout found");

    assert!(loadout.estimated_tokens < 300);
    assert!(estimate_tokens(&loadout.markdown) < 300);
    // Should still activate relevant ADRs
    assert!(!loadout.l3_cards.is_empty());
}

#[test]
fn test_cjk_and_unicode_paths_and_tokens() {
    let vault = create_test_vault_with_all_module_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    let modified = vec![
        ModifiedFile::new(
            "src/存储模块/sqlite_本地缓存.rs",
            FileChangeStatus::Modified,
        ),
        ModifiedFile::new("src/向量计算/onnx_嵌入引擎.rs", FileChangeStatus::Modified),
    ];

    let loadout = builder
        .build_with_diff("k0maru-memory", &modified)
        .unwrap()
        .expect("Loadout found");

    let tokens = estimate_tokens(&loadout.markdown);
    assert!(tokens < 300, "CJK paths token budget violation: {}", tokens);
}
