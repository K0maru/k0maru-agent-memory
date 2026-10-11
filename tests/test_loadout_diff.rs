use std::fs;
use std::process::Command;
use tempfile::TempDir;

use k0maru::adapters::ObsidianAdapter;
use k0maru::git::{parse_diff_stream, FileChangeStatus, GitInspector, ModifiedFile};
use k0maru::loadout::{estimate_tokens, LoadoutBuilder};
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::SqliteStorage;

/// Creates a rich test vault containing specific ADR cards for different modules.
fn create_test_vault_with_adrs() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory for test vault");
    let root = temp_dir.path();

    let projects_dir = root.join("10_Projects");
    let cards_dir = root.join("20_Cards");
    let logs_dir = root.join("01_AI_Logs");

    fs::create_dir_all(&projects_dir).unwrap();
    fs::create_dir_all(&cards_dir).unwrap();
    fs::create_dir_all(&logs_dir).unwrap();

    // Storage ADR Card
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

## Invariant Details
Cache file must reside in .k0maru/cache.sqlite and never be committed.
"#;
    fs::write(cards_dir.join("单文件SQLite缓存契约.md"), storage_card).unwrap();

    // Vector ADR Card
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

    // CLI ADR Card
    let cli_card = r#"---
title: CLI规范与POSIX管道标准
hierarchy: L3Evergreen
tags:
  - cli
  - posix
  - stream
---
# CLI规范与POSIX管道标准

## 📌 核心概念
> All subcommands support standard stdin/stdout pipes, json streams, and standard exit codes.

## Details
Respect non-blocking TTY guards.
"#;
    fs::write(cards_dir.join("CLI规范与POSIX管道标准.md"), cli_card).unwrap();

    // General Architecture Card
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

    // Project Note
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
- [[20_Cards/CLI规范与POSIX管道标准]]
"#;
    fs::write(projects_dir.join("k0maru-memory.md"), project_note).unwrap();

    temp_dir
}

#[test]
fn test_git_inspector_live_git_repo() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Initialize git repository
    let run = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .expect("Failed to run git command");
        assert!(status.success(), "Git command failed: {:?}", args);
    };

    run(&["init"]);
    run(&["config", "user.name", "Test User"]);
    run(&["config", "user.email", "test@example.com"]);

    // Create and commit initial base file
    fs::write(root.join("initial.txt"), "hello world").unwrap();
    run(&["add", "initial.txt"]);
    run(&["commit", "-m", "initial commit"]);

    // Staged modification
    fs::create_dir_all(root.join("src/storage")).unwrap();
    fs::write(root.join("src/storage/sqlite.rs"), "pub struct Sqlite;").unwrap();
    run(&["add", "src/storage/sqlite.rs"]);

    // Unstaged modification
    fs::write(root.join("initial.txt"), "hello modified").unwrap();

    // Untracked file
    fs::create_dir_all(root.join("src/vector")).unwrap();
    fs::write(root.join("src/vector/engine.rs"), "pub struct Engine;").unwrap();

    let changes = GitInspector::inspect_diff(root, None).unwrap();
    assert!(!changes.is_empty());

    let paths: Vec<String> = changes
        .iter()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();

    assert!(
        paths.iter().any(|p| p.contains("sqlite.rs")),
        "Expected sqlite.rs in changes: {:?}",
        paths
    );
    assert!(
        paths.iter().any(|p| p.contains("initial.txt")),
        "Expected initial.txt in changes: {:?}",
        paths
    );
    assert!(
        paths.iter().any(|p| p.contains("engine.rs")),
        "Expected engine.rs in changes: {:?}",
        paths
    );
}

#[test]
fn test_git_inspector_non_git_repo_graceful() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    assert!(!GitInspector::is_git_repo(root));
    let changes = GitInspector::inspect_diff(root, None).unwrap();
    assert!(changes.is_empty());
}

#[test]
fn test_parse_diff_stream_comprehensive() {
    let diff_input = r#"
diff --git a/src/storage/sqlite.rs b/src/storage/sqlite.rs
index abc..def 100644
--- a/src/storage/sqlite.rs
+++ b/src/storage/sqlite.rs
@@ -10,2 +10,4 @@
+use rusqlite::Connection;
 M src/main.rs
?? docs/adr/001-storage.md
A  crates/k0maru-core/src/lib.rs
"#;

    let files = parse_diff_stream(diff_input.as_bytes()).unwrap();
    assert_eq!(files.len(), 4);

    let paths: Vec<String> = files
        .iter()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();

    assert!(paths.contains(&"src/storage/sqlite.rs".to_string()));
    assert!(paths.contains(&"src/main.rs".to_string()));
    assert!(paths.contains(&"docs/adr/001-storage.md".to_string()));
    assert!(paths.contains(&"crates/k0maru-core/src/lib.rs".to_string()));
}

#[test]
fn test_change_aware_loadout_activates_storage_adr() {
    let vault = create_test_vault_with_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    let modified = vec![ModifiedFile::new(
        "src/storage/sqlite.rs",
        FileChangeStatus::Modified,
    )];

    let loadout = builder
        .build_with_diff("k0maru-memory", &modified)
        .unwrap()
        .expect("Project loadout should be found");

    assert_eq!(loadout.project_name, "k0maru-memory");
    assert!(
        loadout.markdown.contains("[Diff-Aware]"),
        "Markdown must contain [Diff-Aware] tag"
    );
    assert!(
        loadout.markdown.contains("Active Changes"),
        "Markdown must contain Active Changes summary"
    );
    assert!(
        loadout.markdown.contains("storage"),
        "Markdown must reflect storage module keyword"
    );

    // The top activated card should be the Storage ADR card
    assert!(!loadout.l3_cards.is_empty());
    assert!(
        loadout.l3_cards[0].name.contains("SQLite") || loadout.l3_cards[0].name.contains("storage"),
        "Top activated card should be the Storage ADR card, found: {:?}",
        loadout.l3_cards[0].name
    );

    // Strict <300 token budget verification
    assert!(
        loadout.estimated_tokens < 300,
        "Token count must be strictly <300, got {}",
        loadout.estimated_tokens
    );
    assert!(
        estimate_tokens(&loadout.markdown) < 300,
        "Direct estimate of markdown must be strictly <300, got {}",
        estimate_tokens(&loadout.markdown)
    );
}

#[test]
fn test_change_aware_loadout_activates_vector_adr() {
    let vault = create_test_vault_with_adrs();
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
        .expect("Project loadout should be found");

    assert!(!loadout.l3_cards.is_empty());
    assert!(
        loadout.l3_cards[0].name.contains("向量") || loadout.l3_cards[0].name.contains("vector"),
        "Top activated card should be Vector ADR card, found: {:?}",
        loadout.l3_cards[0].name
    );

    assert!(loadout.estimated_tokens < 300);
}

#[test]
fn test_change_aware_loadout_fallback_on_unrelated_changes() {
    let vault = create_test_vault_with_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    let modified = vec![ModifiedFile::new(
        "misc/unrelated_scratch.txt",
        FileChangeStatus::Modified,
    )];

    let loadout = builder
        .build_with_diff("k0maru-memory", &modified)
        .unwrap()
        .expect("Project loadout should be found");

    // Falls back to project baseline cards
    assert!(!loadout.l3_cards.is_empty());
    assert!(loadout.estimated_tokens < 300);
}

#[test]
fn test_change_aware_loadout_clean_working_tree() {
    let vault = create_test_vault_with_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    let loadout = builder
        .build_with_diff("k0maru-memory", &[])
        .unwrap()
        .expect("Project loadout should be found");

    assert!(loadout.markdown.contains("Clean working tree"));
    assert!(loadout.estimated_tokens < 300);
}

#[test]
fn test_change_aware_loadout_massive_diff_strict_token_budget() {
    let vault = create_test_vault_with_adrs();
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    // 50 modified files
    let mut modified = Vec::new();
    for i in 0..50 {
        modified.push(ModifiedFile::new(
            format!("src/module_{i}/subsystem_{i}/file_{i}.rs"),
            FileChangeStatus::Modified,
        ));
    }

    let loadout = builder
        .build_with_diff("k0maru-memory", &modified)
        .unwrap()
        .expect("Project loadout should be found");

    assert!(
        loadout.estimated_tokens < 300,
        "Must remain <300 tokens even with 50 files, got {}",
        loadout.estimated_tokens
    );
    assert!(
        estimate_tokens(&loadout.markdown) < 300,
        "Markdown must be <300 tokens, got {}",
        estimate_tokens(&loadout.markdown)
    );
}

#[test]
fn test_cli_loadout_diff_flag() {
    let vault = create_test_vault_with_adrs();
    let mut cmd = assert_cmd::Command::cargo_bin("k0maru").unwrap();
    cmd.arg("loadout")
        .arg("k0maru-memory")
        .arg("--diff")
        .arg("--vault")
        .arg(vault.path());

    let output = cmd.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[Diff-Aware]"));
    assert!(stdout.contains("Active Changes"));
    assert!(estimate_tokens(&stdout) < 300);
}

#[test]
fn test_cli_loadout_diff_json_flag() {
    let vault = create_test_vault_with_adrs();
    let mut cmd = assert_cmd::Command::cargo_bin("k0maru").unwrap();
    cmd.arg("loadout")
        .arg("k0maru-memory")
        .arg("--diff")
        .arg("--json")
        .arg("--vault")
        .arg(vault.path());

    let output = cmd.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(val["project_name"], "k0maru-memory");
    let md = val["markdown"].as_str().unwrap();
    assert!(md.contains("[Diff-Aware]"));
    assert!(estimate_tokens(md) < 300);
}

#[test]
fn test_mcp_loadout_diff_support() {
    use k0maru::McpServer;
    use serde_json::json;

    let vault = create_test_vault_with_adrs();
    let mut server = McpServer::new(vault.path());

    // 1. Verify tools list exposes diff and branch parameters
    let tools_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    let tools_resp = server.handle_line(&tools_req.to_string()).unwrap();
    let resp_val: serde_json::Value = serde_json::from_str(&tools_resp).unwrap();
    let tools = resp_val["result"]["tools"].as_array().unwrap();
    let loadout_tool = tools
        .iter()
        .find(|t| t["name"] == "get_project_loadout")
        .expect("get_project_loadout tool must exist");

    assert!(loadout_tool["inputSchema"]["properties"]["diff"].is_object());
    assert!(loadout_tool["inputSchema"]["properties"]["branch"].is_object());

    // 2. Call tool with diff = true
    let call_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "get_project_loadout",
            "arguments": {
                "project_name": "k0maru-memory",
                "diff": true
            }
        }
    });
    let call_resp = server.handle_line(&call_req.to_string()).unwrap();
    let call_val: serde_json::Value = serde_json::from_str(&call_resp).unwrap();
    assert!(
        call_val["error"].is_null(),
        "Error: {:?}",
        call_val["error"]
    );
    let content = call_val["result"]["content"][0]["text"].as_str().unwrap();
    assert!(content.contains("[Diff-Aware]"));
    assert!(estimate_tokens(content) < 300);
}
