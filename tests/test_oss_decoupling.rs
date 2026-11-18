use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;

use assert_cmd::Command;
use serde_json::{json, Value};
use tempfile::tempdir;

use k0maru::adapters::{detect_layout, open_vault_adapter, VaultLayout};
use k0maru::bootstrap::{run_bootstrap, BootstrapOptions};
use k0maru::convention::{FlushEngine, FlushRequest};
use k0maru::core::models::HierarchyLevel;
use k0maru::core::traits::VaultAdapter;
use k0maru::ecosystem::McpClient;
use k0maru::install::InstallTarget;
use k0maru::loadout::{estimate_tokens, LoadoutBuilder};
use k0maru::mcp::{McpProfile, McpServer};
use k0maru::offload::OffloadEngine;
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::SqliteStorage;

fn init_git_repo(path: &Path) {
    let _ = StdCommand::new("git")
        .args(["init"])
        .current_dir(path)
        .output();
    let _ = StdCommand::new("git")
        .args(["config", "user.name", "K0maru Tester"])
        .current_dir(path)
        .output();
    let _ = StdCommand::new("git")
        .args([
            "config",
            "user.email",
            "93422639+K0maru@users.noreply.github.com",
        ])
        .current_dir(path)
        .output();
}

#[test]
fn test_p0_sniffer_ignores_prose_and_flush_never_lands_at_root() {
    let dir = tempdir().unwrap();
    let vault = dir.path();

    // AGENTS.md with prose bullets that previously poisoned ConventionSniffer
    fs::write(
        vault.join("AGENTS.md"),
        r#"# Agent Rules
- Logs: emitted to stderr via tracing, never to files
- Decision: we use Postgres for persistence
- Concept: stderr
"#,
    )
    .unwrap();

    let engine = FlushEngine::new(vault).unwrap();

    let log_res = engine
        .flush(FlushRequest {
            title: "Retry Storm Fix".to_string(),
            summary: None,
            content: "Increased backoff jitter.".to_string(),
            category: "log".to_string(),
            tags: vec![],
            related_notes: vec![],
            dry_run: false,
            supersedes: vec![],
        })
        .unwrap();
    assert!(
        log_res.relative_path.starts_with("logs/"),
        "expected log note in logs/, got: {:?}",
        log_res.relative_path
    );
    assert!(log_res.file_path.is_file());

    let dec_res = engine
        .flush(FlushRequest {
            title: "Adopt OTel".to_string(),
            summary: None,
            content: "Use OpenTelemetry SDK.".to_string(),
            category: "decision".to_string(),
            tags: vec![],
            related_notes: vec![],
            dry_run: false,
            supersedes: vec![],
        })
        .unwrap();
    assert!(
        dec_res.relative_path.starts_with("decisions/"),
        "expected decision note in decisions/, got: {:?}",
        dec_res.relative_path
    );
    assert!(dec_res.file_path.is_file());

    // Unknown/traversal categories must be rejected with allowed categories list
    let err = engine
        .flush(FlushRequest {
            title: "Bad".to_string(),
            summary: None,
            content: "Should fail".to_string(),
            category: "../../escape".to_string(),
            tags: vec![],
            related_notes: vec![],
            dry_run: false,
            supersedes: vec![],
        })
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("decision") && err.contains("log") && err.contains("skill"),
        "error should list valid categories: {err}"
    );

    // Explicit ```k0maru block overrides ARE honored
    let dir2 = tempdir().unwrap();
    let v2 = dir2.path();
    fs::write(
        v2.join("AGENTS.md"),
        "```k0maru\nlogs: custom_logs\ndecisions: docs/adr\n```\n",
    )
    .unwrap();
    let eng2 = FlushEngine::new(v2).unwrap();
    let r2 = eng2
        .flush(FlushRequest {
            title: "Custom Routed".to_string(),
            summary: None,
            content: "Body".to_string(),
            category: "log".to_string(),
            tags: vec![],
            related_notes: vec![],
            dry_run: false,
            supersedes: vec![],
        })
        .unwrap();
    assert!(
        r2.relative_path.starts_with("custom_logs/"),
        "explicit ```k0maru fence must route to custom_logs/, got: {:?}",
        r2.relative_path
    );
}

#[test]
fn test_p0_flush_surfaces_sync_warning_and_mcp_enforces_live_db_health() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    fs::create_dir_all(vault.join("decisions")).unwrap();
    fs::create_dir_all(vault.join("logs")).unwrap();
    fs::write(
        vault.join("decisions/use-grpc.md"),
        "---\ntitle: Use gRPC\n---\n\n# Use gRPC\n\n## Transport\n\nUse gRPC for internal RPC.\n",
    )
    .unwrap();

    // Sync initial v3 cache
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["sync", "--vault", vault.to_str().unwrap()])
        .assert()
        .success();

    let cache_path = vault.join(".k0maru/cache.sqlite");
    assert!(cache_path.is_file());

    // 1. Live MCP session sees sections initially
    let mut server = McpServer::new(vault);
    let r1 = server
        .handle_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "recall_memory",
                    "arguments": { "query": "internal RPC" }
                }
            })
            .to_string(),
        )
        .unwrap();
    let v1: Value = serde_json::from_str(&r1).unwrap();
    assert_ne!(v1["result"]["isError"], true);
    let t1 = v1["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        t1.contains("decisions/use-grpc.md:") && t1.contains("#Transport"),
        "expected section locator before compact, got: {t1}"
    );

    // 2. Replace cache.sqlite on disk (simulate sync --force / atomic DB swap) -> McpServer transparently reopens
    let tmp_swap = vault.join(".k0maru/cache.sqlite.swap");
    {
        let mut new_storage = SqliteStorage::open_rw(&tmp_swap).unwrap();
        let adapter = open_vault_adapter(vault);
        let mut scanner = IncrementalScanner::new(&adapter, &mut new_storage);
        scanner.sync(true).unwrap();
    }
    fs::rename(&tmp_swap, &cache_path).unwrap();

    let r2 = server
        .handle_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "recall_memory",
                    "arguments": { "query": "internal RPC" }
                }
            })
            .to_string(),
        )
        .unwrap();
    let v2: Value = serde_json::from_str(&r2).unwrap();
    assert_ne!(v2["result"]["isError"], true);
    let t2 = v2["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        t2.contains("decisions/use-grpc.md:") && t2.contains("#Transport"),
        "expected section locator after disk replacement, got: {t2}"
    );

    // 3. Also verify doctor --compact keeps section locators working mid-session
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["doctor", "--compact", "--vault", vault.to_str().unwrap()])
        .assert()
        .success();
    let r2b = server
        .handle_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 20,
                "method": "tools/call",
                "params": {
                    "name": "recall_memory",
                    "arguments": { "query": "internal RPC" }
                }
            })
            .to_string(),
        )
        .unwrap();
    let v2b: Value = serde_json::from_str(&r2b).unwrap();
    assert_ne!(v2b["result"]["isError"], true);
    assert!(v2b["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("#Transport"));

    // 4. Mid-session PRAGMA user_version = 4 -> MCP must fail with isError:true (no silent in-memory fallback!)
    {
        let ext = rusqlite::Connection::open(&cache_path).unwrap();
        ext.pragma_update(None, "user_version", 4).unwrap();
    }
    let r3 = server
        .handle_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "recall_memory",
                    "arguments": { "query": "internal RPC" }
                }
            })
            .to_string(),
        )
        .unwrap();
    let v3: Value = serde_json::from_str(&r3).unwrap();
    assert_eq!(v3["result"]["isError"], true);
    let err3 = v3["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        err3.contains("v4 > supported v3") || err3.contains("schema mismatch"),
        "expected schema mismatch error, got: {err3}"
    );

    // 5. Fresh McpServer on v4 DB must ALSO return isError:true immediately
    let mut fresh_server = McpServer::new(vault);
    let r4 = fresh_server
        .handle_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": {
                    "name": "get_project_loadout",
                    "arguments": {}
                }
            })
            .to_string(),
        )
        .unwrap();
    let v4: Value = serde_json::from_str(&r4).unwrap();
    assert_eq!(v4["result"]["isError"], true);

    // 6. FlushEngine and MCP flush_session on v4 DB surface [warning] instead of swallowing error
    let engine = FlushEngine::new(vault).unwrap();
    let flush_res = engine
        .flush(FlushRequest {
            title: "Note During Schema Upgrade".to_string(),
            summary: None,
            content: "Written safely to disk.".to_string(),
            category: "log".to_string(),
            tags: vec![],
            related_notes: vec![],
            dry_run: false,
            supersedes: vec![],
        })
        .unwrap();
    assert!(flush_res.file_path.is_file());
    assert!(
        flush_res
            .sync_warning
            .as_deref()
            .is_some_and(|w| w.contains("cache sync failed")),
        "expected sync_warning on FlushResult, got: {:?}",
        flush_res.sync_warning
    );

    let r5 = fresh_server
        .handle_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "tools/call",
                "params": {
                    "name": "flush_session",
                    "arguments": {
                        "title": "MCP Flush During Upgrade",
                        "content": "Still written to disk.",
                        "category": "log"
                    }
                }
            })
            .to_string(),
        )
        .unwrap();
    let v5: Value = serde_json::from_str(&r5).unwrap();
    let t5 = v5["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        t5.contains("[warning] note written to disk, but cache sync failed"),
        "expected [warning] in MCP flush_session output, got: {t5}"
    );
}

#[test]
fn test_zero_personal_coupling_loadout_refs_clients_and_ui() {
    // 1. LoadoutBuilder never emits vault-specific 00_Daily / 20_Cards rules
    let dir = tempdir().unwrap();
    let vault = dir.path();
    fs::create_dir_all(vault.join("00_Daily")).unwrap();
    fs::create_dir_all(vault.join("projects")).unwrap();
    fs::write(
        vault.join("projects/billing.md"),
        "---\ntitle: Billing\nstatus: active\n---\n\n# Billing\n\n> Ledger service.\n",
    )
    .unwrap();

    let adapter = open_vault_adapter(vault);
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let loadout = builder.build("billing").unwrap().unwrap();
    assert!(!loadout.markdown.contains("00_Daily"));
    assert!(!loadout.markdown.contains("20_Cards"));
    assert!(!loadout.markdown.contains("WikiLinks First"));
    assert!(loadout.markdown.contains("Agent Autonomous Protocol"));

    // 2. Default refs_dir uses .k0maru/refs, never .scratch/refs
    assert_eq!(OffloadEngine::default().refs_dir, Path::new(".k0maru/refs"));
    let mcp = McpServer::new(vault);
    assert_eq!(mcp.refs_dir(), vault.join(".k0maru/refs"));

    // 3. Gemini CLI vs Antigravity split
    let home = tempdir().unwrap();
    assert_eq!(
        McpClient::Gemini.config_path(Some(home.path())).unwrap(),
        home.path().join(".gemini/settings.json")
    );
    assert_eq!(
        McpClient::Antigravity
            .config_path(Some(home.path()))
            .unwrap(),
        home.path().join(".gemini/antigravity-cli/mcp_config.json")
    );
    assert_eq!(
        "gemini".parse::<InstallTarget>().unwrap(),
        InstallTarget::Gemini
    );
    assert_eq!(
        "antigravity".parse::<InstallTarget>().unwrap(),
        InstallTarget::Antigravity
    );

    // 4. app.js contains zero fabricated client fallback
    let app_js = include_str!("../src/ui/assets/app.js");
    assert!(!app_js.contains("Skill export sync active"));
    assert!(!app_js.contains("Integrated session & hooks active"));

    // 5. unified detect_layout + wiki/logs hierarchy
    assert_eq!(detect_layout(vault), VaultLayout::Structured);
    let doc = adapter
        .read_document(Path::new("projects/billing.md"))
        .unwrap();
    assert_eq!(doc.hierarchy, HierarchyLevel::L3Evergreen);
}

#[test]
fn test_engineering_repo_init_indexes_docs_adr_and_enforces_guard() {
    let dir = tempdir().unwrap();
    let repo = dir.path();
    init_git_repo(repo);

    fs::create_dir_all(repo.join("docs/adr")).unwrap();
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(
        repo.join("README.md"),
        "# Acme Service\n\nPayment service.\n",
    )
    .unwrap();
    fs::write(
        repo.join("docs/adr/0001-no-raw-sql.md"),
        r#"---
id: ADR-SQL-001
title: "No Raw SQL"
status: active
---

# ADR-SQL-001: No Raw SQL

## Non-Negotiable Invariants
- [ADR-SQL-001] (deny) [src/**/*.rs] forbidden: "execute_raw" | remediation: "Use parameterized query builder."
"#,
    )
    .unwrap();
    fs::write(repo.join("src/lib.rs"), "pub fn query() {}\n").unwrap();

    // Commit baseline
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "chore: baseline"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Run bootstrap (k0maru init)
    let report = run_bootstrap(BootstrapOptions {
        repo_path: repo.to_path_buf(),
        vault_path: None,
        project_name: None,
        install_clients: false,
        home_override: None,
    })
    .unwrap();

    assert!(
        report
            .vault_path
            .join("decisions/0001-no-raw-sql.md")
            .is_file(),
        "k0maru init should mirror/index docs/adr/0001-no-raw-sql.md"
    );

    // `search` finds docs/adr content out of the box
    let search_out = Command::cargo_bin("k0maru")
        .unwrap()
        .current_dir(repo)
        .args(["search", "parameterized query builder"])
        .output()
        .unwrap();
    assert!(search_out.status.success());
    let stdout = String::from_utf8_lossy(&search_out.stdout);
    assert!(
        stdout.contains("0001-no-raw-sql"),
        "search should find mirrored docs/adr note: {stdout}"
    );

    // Introduce violation in working tree and run `guard check --diff` (even against an adr in docs/adr)
    fs::write(
        repo.join("src/lib.rs"),
        "pub fn query() { db.execute_raw(\"SELECT 1\"); }\n",
    )
    .unwrap();

    let guard_assert = Command::cargo_bin("k0maru")
        .unwrap()
        .current_dir(repo)
        .args(["guard", "check", "--diff"])
        .assert()
        .code(1);
    let guard_out = String::from_utf8_lossy(&guard_assert.get_output().stdout);
    assert!(
        guard_out.contains("ADR-SQL-001"),
        "guard check --diff must enforce docs/adr rules out of the box: {guard_out}"
    );
}

#[test]
fn test_commonmark_docs_corpus_garden_zero_false_orphans_and_overview_loadout() {
    let dir = tempdir().unwrap();
    let docs = dir.path();
    fs::create_dir_all(docs.join("routing")).unwrap();
    fs::create_dir_all(docs.join("auth")).unwrap();
    fs::write(
        docs.join("index.md"),
        "# Docs\n\n- [Routing](./routing/guide.md#paths)\n- [Auth](./auth/jwt.md)\n",
    )
    .unwrap();
    fs::write(
        docs.join("routing/guide.md"),
        "# Routing Guide\n\nSee [JWT Auth](../auth/jwt.md) and [Home](../index.md).\n",
    )
    .unwrap();
    fs::write(
        docs.join("auth/jwt.md"),
        "# JWT Auth\n\nBack to [Routing](../routing/guide.md).\n",
    )
    .unwrap();

    // 1. garden reports 0 orphans because CommonMark links populate link_edges
    let garden_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args(["garden", "--vault", docs.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(garden_out.status.success());
    let garden_json: Value = serde_json::from_slice(&garden_out.stdout).unwrap();
    assert_eq!(
        garden_json["orphans"].as_array().unwrap().len(),
        0,
        "CommonMark-linked docs corpus must have 0 orphans: {}",
        garden_json
    );

    // 2. loadout --list does NOT mislabel docs pages as projects
    let list_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "loadout",
            "--list",
            "--vault",
            docs.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(list_out.status.success());
    let projects: Value = serde_json::from_slice(&list_out.stdout).unwrap();
    assert_eq!(projects.as_array().unwrap().len(), 0);

    // 3. get_project_loadout({}) returns Vault Overview <= 300 tokens
    let mut server = McpServer::new(docs);
    let resp_str = server
        .handle_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "get_project_loadout",
                    "arguments": {}
                }
            })
            .to_string(),
        )
        .unwrap();
    let resp: Value = serde_json::from_str(&resp_str).unwrap();
    assert_ne!(resp["result"]["isError"], true);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Vault Overview Loadout"), "got: {text}");
    assert!(estimate_tokens(text) <= 300);
}

#[test]
fn test_consumer_model_minimal_profile_inspect_handoff_and_guard_mcp() {
    let dir = tempdir().unwrap();
    let vault = dir.path().join("wiki");
    fs::create_dir_all(vault.join("decisions")).unwrap();
    fs::create_dir_all(vault.join("logs")).unwrap();
    fs::write(
        vault.join("decisions/use-grpc.md"),
        "---\ntitle: Use gRPC\n---\n\n# Use gRPC\n\n## Details\nLine 1\nLine 2\n",
    )
    .unwrap();
    fs::write(
        vault.join("logs/2026-01-02-session.md"),
        "---\ntitle: Session Log\n---\n\n# Session Log\n\nRetried backoff.\n",
    )
    .unwrap();

    // 1. Minimal profile tools/list <= 2.2 KB and 5 tools, no ALL-CAPS ALWAYS
    let mut min_server = McpServer::new(&vault).with_profile(McpProfile::Minimal);
    let list_str = min_server
        .handle_line(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}).to_string())
        .unwrap();
    assert!(
        list_str.len() <= 2200,
        "minimal tools/list must be <= 2200 bytes, got {} bytes",
        list_str.len()
    );
    assert!(!list_str.contains("ALWAYS"));
    let list_val: Value = serde_json::from_str(&list_str).unwrap();
    let names: Vec<&str> = list_val["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec![
            "get_project_loadout",
            "recall_memory",
            "inspect_note",
            "flush_session",
            "guard_check"
        ]
    );

    // 2. Full profile checks: no oneOf on supersedes, no colab/A100, no ALL-CAPS ALWAYS
    let mut full_server = McpServer::new(&vault);
    let full_list = full_server
        .handle_line(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}).to_string())
        .unwrap();
    assert!(!full_list.contains("colab"));
    assert!(!full_list.contains("A100 benchmark"));
    assert!(!full_list.contains("ALWAYS"));
    let full_val: Value = serde_json::from_str(&full_list).unwrap();
    let flush_schema = full_val["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "flush_session")
        .unwrap();
    assert!(flush_schema["inputSchema"]["properties"]["supersedes"]
        .get("oneOf")
        .is_none());
    assert_eq!(
        flush_schema["inputSchema"]["properties"]["supersedes"]["type"],
        "array"
    );

    // 3. inspect_note forgiving locators: stem, missing .md, vault-dir prefix, copied K-DSL
    for locator in [
        "use-grpc",
        "logs/2026-01-02-session",
        "wiki/logs/2026-01-02-session.md",
        "!μ:L3|decisions/use-grpc.md:1-5#Details|score:0.0164|rank:#1|links:0/0 § Line 1",
    ] {
        let r = full_server
            .handle_line(
                &json!({
                    "jsonrpc": "2.0",
                    "id": 3,
                    "method": "tools/call",
                    "params": {
                        "name": "inspect_note",
                        "arguments": { "path": locator }
                    }
                })
                .to_string(),
            )
            .unwrap();
        let v: Value = serde_json::from_str(&r).unwrap();
        assert_ne!(
            v["result"]["isError"], true,
            "inspect_note failed for '{}': {}",
            locator, v
        );
    }

    // 4. recall_memory direct path query returns it as #1 with |rank:#1|
    let recall_str = full_server
        .handle_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": {
                    "name": "recall_memory",
                    "arguments": { "query": "decisions/use-grpc.md" }
                }
            })
            .to_string(),
        )
        .unwrap();
    let recall_val: Value = serde_json::from_str(&recall_str).unwrap();
    let recall_text = recall_val["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        recall_text.contains("decisions/use-grpc.md") && recall_text.contains("|rank:#1|"),
        "got: {recall_text}"
    );

    // 5. get_project_loadout miss includes Available projects/topics:
    let miss_str = full_server
        .handle_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "tools/call",
                "params": {
                    "name": "get_project_loadout",
                    "arguments": { "project_name": "nonexistent-proj" }
                }
            })
            .to_string(),
        )
        .unwrap();
    let miss_val: Value = serde_json::from_str(&miss_str).unwrap();
    assert_eq!(miss_val["result"]["isError"], true);
    assert!(miss_val["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Available projects/topics:"));

    // 6. handoff_session accepts parameter aliases from/to/next/question/ref
    let handoff_str = full_server
        .handle_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "tools/call",
                "params": {
                    "name": "handoff_session",
                    "arguments": {
                        "action": "create",
                        "from": "qwen-7b",
                        "to": "reviewer",
                        "title": "Alias Handoff",
                        "next": ["Run tests"],
                        "question": ["Check timeout?"],
                        "ref": ["decisions/use-grpc.md"]
                    }
                }
            })
            .to_string(),
        )
        .unwrap();
    let handoff_val: Value = serde_json::from_str(&handoff_str).unwrap();
    assert_ne!(
        handoff_val["result"]["isError"], true,
        "handoff aliases failed: {}",
        handoff_val
    );
    let handoff_text = handoff_val["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(handoff_text.contains("qwen-7b"));
    assert!(handoff_text.contains("Run tests"));
}

#[test]
fn test_consumer_model_guard_diff_disambiguation_and_param_aliases() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("payment-gateway");
    fs::create_dir_all(&repo).unwrap();
    init_git_repo(&repo);

    let vault = repo.join(".k0maru").join("wiki");
    fs::create_dir_all(vault.join("decisions")).unwrap();
    fs::create_dir_all(vault.join("logs")).unwrap();
    fs::create_dir_all(repo.join("src/settlement")).unwrap();

    fs::write(
        vault.join("decisions/adr-001-lock.md"),
        "---\ntitle: ADR-001 Settlement Lock\ninvariants:\n  - id: ADR-SETTLE-001\n    title: Mandatory Distributed Lock\n    file_pattern: \"src/settlement/*.rs\"\n    forbidden: \"exec_without_lock\"\n    remediation: \"Acquire distributed lock first.\"\n---\n\n# ADR-001 Settlement Lock\n\n## Non-Negotiable Invariants\n- [ADR-SETTLE-001] (deny) [src/settlement/*.rs] forbidden: \"exec_without_lock\" | remediation: \"Acquire distributed lock first.\"\n",
    )
    .unwrap();
    fs::write(repo.join("src/settlement/mod.rs"), "pub fn settle() {}\n").unwrap();
    assert!(std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo)
        .status()
        .unwrap()
        .success());
    assert!(std::process::Command::new("git")
        .args(["commit", "-m", "init payment-gateway"])
        .current_dir(&repo)
        .status()
        .unwrap()
        .success());

    // 1. Minimal, sidecar, and full schemas expose expected guard_check fields.
    let mut min_server = McpServer::new(&vault).with_profile(McpProfile::Minimal);
    let min_list_str = min_server
        .handle_line(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}).to_string())
        .unwrap();
    assert!(
        min_list_str.len() <= 2200,
        "minimal tools/list must be <= 2200 bytes, got {} bytes",
        min_list_str.len()
    );
    let min_list_val: Value = serde_json::from_str(&min_list_str).unwrap();
    let min_guard_props = &min_list_val["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "guard_check")
        .unwrap()["inputSchema"]["properties"];
    assert_eq!(min_guard_props["repo_path"]["type"], "string");
    assert_eq!(min_guard_props["diff"]["type"], "boolean");
    assert_eq!(min_guard_props["patch"]["type"], "string");
    assert!(min_guard_props.get("files").is_some());
    assert!(min_guard_props.get("branch").is_some());

    let mut sidecar_server = McpServer::new(&vault).with_sidecar(true);
    let sidecar_list: Value = serde_json::from_str(
        &sidecar_server
            .handle_line(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}).to_string())
            .unwrap(),
    )
    .unwrap();
    let sidecar_guard_props = &sidecar_list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "guard_check")
        .unwrap()["inputSchema"]["properties"];
    assert_eq!(sidecar_guard_props["patch"]["type"], "string");

    let mut full_server = McpServer::new(&vault);
    let full_list: Value = serde_json::from_str(
        &full_server
            .handle_line(&json!({"jsonrpc": "2.0", "id": 3, "method": "tools/list"}).to_string())
            .unwrap(),
    )
    .unwrap();
    let full_guard_props = &full_list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "guard_check")
        .unwrap()["inputSchema"]["properties"];
    assert_eq!(full_guard_props["patch"]["type"], "string");

    // 2. Clean workspace git diff passes under boolean/string mode tokens.
    let call_guard = |srv: &mut McpServer, id: i64, args: Value| -> Value {
        let resp = srv
            .handle_line(
                &json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": "tools/call",
                    "params": {
                        "name": "guard_check",
                        "arguments": args
                    }
                })
                .to_string(),
            )
            .unwrap();
        serde_json::from_str(&resp).unwrap()
    };

    for (idx, args) in [
        json!({"diff": true}),
        json!({"diff": true, "repo_path": repo}),
        json!({"diff": "true", "repo_path": repo}),
        json!({"diff": "auto", "repo_path": repo}),
        json!({"repo_path": repo}),
        json!({}),
    ]
    .into_iter()
    .enumerate()
    {
        let res = call_guard(&mut min_server, 10 + idx as i64, args.clone());
        assert_ne!(
            res["result"]["isError"], true,
            "guard_check({args}) must succeed on clean git diff: {res}"
        );
    }

    // 3. Introduce an uncommitted violation and verify diff="true" detects it via git diff.
    fs::write(
        repo.join("src/settlement/mod.rs"),
        "pub fn settle() { exec_without_lock(); }\n",
    )
    .unwrap();
    let dirty_res = call_guard(
        &mut min_server,
        20,
        json!({"diff": "true", "repo_path": repo}),
    );
    assert_eq!(dirty_res["result"]["isError"], true);
    let dirty_text = dirty_res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        dirty_text.contains("ADR-SETTLE-001"),
        "expected ADR-SETTLE-001 violation, got: {dirty_text}"
    );
    assert!(
        !dirty_text.contains("Invalid unified diff"),
        "boolean string token must never be parsed as raw diff: {dirty_text}"
    );

    // 4. Malformed non-boolean diff or patch must still fail closed.
    for bad_args in [
        json!({"diff": "not a valid diff"}),
        json!({"patch": "not a valid diff"}),
    ] {
        let bad_res = call_guard(&mut full_server, 21, bad_args.clone());
        assert_eq!(
            bad_res["result"]["isError"], true,
            "malformed diff must fail closed for {bad_args}: {bad_res}"
        );
        let bad_text = bad_res["result"]["content"][0]["text"].as_str().unwrap();
        assert!(
            bad_text.contains("Invalid unified diff"),
            "expected 'Invalid unified diff', got: {bad_text}"
        );
    }

    // 5. Explicit valid patch works even when diff=true is also supplied.
    let valid_patch = "diff --git a/src/settlement/mod.rs b/src/settlement/mod.rs\n--- a/src/settlement/mod.rs\n+++ b/src/settlement/mod.rs\n@@ -1 +1 @@\n+pub fn settle() { exec_without_lock(); }\n";
    let patch_res = call_guard(
        &mut min_server,
        22,
        json!({"diff": true, "patch": valid_patch, "repo_path": repo}),
    );
    assert_eq!(patch_res["result"]["isError"], true);
    assert!(patch_res["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("ADR-SETTLE-001"));

    // 6. flush_session with 'body' alias.
    let flush_res: Value = serde_json::from_str(
        &min_server
            .handle_line(
                &json!({
                    "jsonrpc": "2.0",
                    "id": 30,
                    "method": "tools/call",
                    "params": {
                        "name": "flush_session",
                        "arguments": {
                            "title": "Idempotent Settlement Retry",
                            "body": "Always acquire a distributed lock before ledger mutation."
                        }
                    }
                })
                .to_string(),
            )
            .unwrap(),
    )
    .unwrap();
    assert_ne!(
        flush_res["result"]["isError"], true,
        "flush_session with 'body' alias failed: {flush_res}"
    );

    // 7. inspect_note with 'locator' alias.
    let inspect_res: Value = serde_json::from_str(
        &min_server
            .handle_line(
                &json!({
                    "jsonrpc": "2.0",
                    "id": 31,
                    "method": "tools/call",
                    "params": {
                        "name": "inspect_note",
                        "arguments": {
                            "locator": "decisions/adr-001-lock.md"
                        }
                    }
                })
                .to_string(),
            )
            .unwrap(),
    )
    .unwrap();
    assert_ne!(
        inspect_res["result"]["isError"], true,
        "inspect_note with 'locator' alias failed: {inspect_res}"
    );
    assert!(inspect_res["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("ADR-001 Settlement Lock"));

    // 8. recall_memory with 'q' alias and string 'limit': '3'.
    let recall_res: Value = serde_json::from_str(
        &min_server
            .handle_line(
                &json!({
                    "jsonrpc": "2.0",
                    "id": 32,
                    "method": "tools/call",
                    "params": {
                        "name": "recall_memory",
                        "arguments": {
                            "q": "Distributed Lock",
                            "limit": "3"
                        }
                    }
                })
                .to_string(),
            )
            .unwrap(),
    )
    .unwrap();
    assert_ne!(
        recall_res["result"]["isError"], true,
        "recall_memory with 'q' and string 'limit' failed: {recall_res}"
    );
    assert!(recall_res["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("adr-001-lock.md"));
}
