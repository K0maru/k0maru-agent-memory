mod common;

use common::fixtures::mock_obsidian_vault;
use k0maru::mcp::McpServer;
use serde_json::{json, Value};

#[test]
fn test_mcp_initialize_and_tools_list() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    // 1. initialize
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let resp_str = server
        .handle_line(&init_req.to_string())
        .expect("Should respond to initialize");
    let resp: Value = serde_json::from_str(&resp_str).unwrap();
    assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(resp["result"]["serverInfo"]["name"], "k0maru");

    let instructions = resp["result"]["instructions"]
        .as_str()
        .expect("initialize must advertise the autonomous agent protocol");
    assert!(instructions.contains("get_project_loadout(diff=true)"));
    assert!(instructions.contains("recall_memory"));
    assert!(instructions.contains("guard_check"));
    assert!(instructions.contains("## Non-Negotiable Invariants"));

    // 2. tools/list
    let tools_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let tools_str = server.handle_line(&tools_req.to_string()).unwrap();
    let tools_resp: Value = serde_json::from_str(&tools_str).unwrap();
    let tools = tools_resp["result"]["tools"].as_array().unwrap();

    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(tool_names.contains(&"get_project_loadout"));
    assert!(tool_names.contains(&"recall_memory"));
    assert!(tool_names.contains(&"offload_context"));
    assert!(tool_names.contains(&"inspect_log_node"));
    assert!(tool_names.contains(&"flush_session"));
    assert!(tool_names.contains(&"distill_session_skill"));
    assert!(tool_names.contains(&"gc_session_context"));
    assert!(tool_names.contains(&"guard_check"));
    assert!(tool_names.contains(&"timeline_memory"));
    assert!(tool_names.contains(&"inspect_note"));
    assert!(tool_names.contains(&"handoff_session"));
}

#[test]
fn test_mcp_protocol_version_negotiation() {
    let vault = mock_obsidian_vault();

    let negotiate = |server: &mut McpServer, requested: &str| -> String {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "initialize",
            "params": { "protocolVersion": requested }
        });
        let resp: Value =
            serde_json::from_str(&server.handle_line(&req.to_string()).unwrap()).unwrap();
        resp["result"]["protocolVersion"]
            .as_str()
            .unwrap()
            .to_string()
    };

    let mut server = McpServer::new(vault.path());
    assert_eq!(negotiate(&mut server, "2024-11-05"), "2024-11-05");
    assert_eq!(negotiate(&mut server, "2025-03-26"), "2025-03-26");
    assert_eq!(negotiate(&mut server, "2025-06-18"), "2025-06-18");
    assert_eq!(negotiate(&mut server, "1999-01-01"), "2024-11-05");
}

#[test]
fn test_mcp_guard_check_headerless_diff_fails_closed() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    let res = server.handle_tool_call(
        json!(8),
        &json!({
            "name": "guard_check",
            "arguments": {
                "diff": "@@ -1 +1 @@\n+fn orphaned() {}\n"
            }
        }),
    );

    assert_eq!(
        res["result"]["isError"], true,
        "a non-empty diff without file headers must fail closed: {res}"
    );
    let text = res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("no file headers"),
        "error must explain the malformed diff, got: {text}"
    );
}

#[test]
fn test_mcp_new_with_model_mock_never_loads_default_model() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new_with_model(vault.path(), Some("mock"));

    let res = server.handle_tool_call(
        json!(9),
        &json!({
            "name": "recall_memory",
            "arguments": { "query": "Architecture Decisions" }
        }),
    );
    let text = res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Architecture Decisions"));
}

#[test]
fn test_mcp_tool_recall_memory_hybrid() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    let res = server.handle_tool_call(
        json!(10),
        &json!({
            "name": "recall_memory",
            "arguments": {
                "query": "Architecture Decisions"
            }
        }),
    );

    let text = res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Architecture Decisions"));
}

#[test]
fn test_mcp_recall_memory_compact_wire_vs_full() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    // 1. Default (full omitted or full: false): compact single-line K-DSL wire
    let res_default = server.handle_tool_call(
        json!(20),
        &json!({
            "name": "recall_memory",
            "arguments": {
                "query": "Architecture Decisions"
            }
        }),
    );
    let text_default = res_default["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        text_default.starts_with("!μ:"),
        "Default should be compact K-DSL wire starting with !μ:"
    );
    assert!(text_default.contains("20_Cards/Architecture_Decisions.md"));
    assert!(text_default.contains("|score:"));
    assert!(text_default.contains("|links:"));
    assert!(text_default.contains(" § "));
    assert!(
        !text_default.contains("## "),
        "Compact wire format should not contain markdown headers"
    );
    assert!(
        text_default.len() < 1200,
        "Compact wire output must be under 1200 characters"
    );
    for line in text_default.lines() {
        assert!(
            line.starts_with("!μ:"),
            "Every line in compact wire format must start with !μ:"
        );
        assert!(!line.contains('\n'), "Each hit must be a single line");
    }

    // 2. Explicit full: false
    let res_compact = server.handle_tool_call(
        json!(21),
        &json!({
            "name": "recall_memory",
            "arguments": {
                "query": "Architecture Decisions",
                "full": false
            }
        }),
    );
    let text_compact = res_compact["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(text_compact.starts_with("!μ:"));
    assert_eq!(text_compact, text_default);

    // 3. Explicit full: true -> verbose markdown format
    let res_full = server.handle_tool_call(
        json!(22),
        &json!({
            "name": "recall_memory",
            "arguments": {
                "query": "Architecture Decisions",
                "full": true
            }
        }),
    );
    let text_full = res_full["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text_full.contains("## Architecture Decisions"));
    assert!(text_full.contains("Score: "));
    assert!(text_full.contains("Hierarchy: L3Evergreen"));
    assert_ne!(text_compact, text_full);
}

#[test]
fn test_mcp_tool_get_project_loadout() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    let res = server.handle_tool_call(
        json!(11),
        &json!({
            "name": "get_project_loadout",
            "arguments": {
                "project_name": "K0maru Agent Memory"
            }
        }),
    );

    let text = res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("k0maru-memory"));
}

#[test]
fn test_mcp_offload_and_inspect_tools() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    let big_log = (1..=100)
        .map(|i| format!("Compilation log line {}", i))
        .collect::<Vec<_>>()
        .join("\n");

    let offload_res = server.handle_tool_call(
        json!(12),
        &json!({
            "name": "offload_context",
            "arguments": {
                "raw_text": big_log
            }
        }),
    );

    let text = offload_res["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(text.contains("stateDiagram-v2"));
    assert!(text.contains("k0maru inspect node_"));

    // Extract node ID: pattern node_[0-9a-f]{8}
    let node_id = text
        .lines()
        .find_map(|line| {
            if let Some(pos) = line.find("node_") {
                let candidate = &line[pos..];
                let id_end = candidate
                    .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .unwrap_or(candidate.len());
                Some(candidate[..id_end].to_string())
            } else {
                None
            }
        })
        .expect("Must extract node_id from offload response");

    let inspect_res = server.handle_tool_call(
        json!(13),
        &json!({
            "name": "inspect_log_node",
            "arguments": {
                "node_id": node_id
            }
        }),
    );
    let inspect_text = inspect_res["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(inspect_text.contains("Compilation log line 1"));
}

#[test]
fn test_mcp_flush_and_distill_tools() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    // Flush session tool
    let flush_res = server.handle_tool_call(
        json!(14),
        &json!({
            "name": "flush_session",
            "arguments": {
                "title": "MCP Flush Note",
                "content": "Flushed via MCP tool.",
                "dry_run": true
            }
        }),
    );
    let flush_text = flush_res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(flush_text.contains("MCP Flush Note"));

    // Distill tool
    let distill_res = server.handle_tool_call(
        json!(15),
        &json!({
            "name": "distill_session_skill",
            "arguments": {
                "raw_trace": "error[E0308]: mismatched types\nfixed\n",
                "title": "Distilled MCP Skill",
                "dry_run": true
            }
        }),
    );
    let distill_text = distill_res["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(distill_text.contains("Distilled MCP Skill"));
}

#[test]
fn test_mcp_gc_session_context_tool() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    let trajectory = json!({
        "session_id": "test_mcp_gc",
        "turns": [
            {
                "turn_index": 0,
                "role": "system",
                "content": "You are an assistant."
            },
            {
                "turn_index": 1,
                "role": "user",
                "content": "Please review this code."
            },
            {
                "turn_index": 2,
                "role": "assistant",
                "content": "Done."
            }
        ]
    });

    let res = server.handle_tool_call(
        json!(16),
        &json!({
            "name": "gc_session_context",
            "arguments": {
                "trajectory_json": trajectory.to_string()
            }
        }),
    );
    let text = res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("session_id")
            || text.contains("compacted_turns")
            || text.contains("test_mcp_gc")
    );
}

#[test]
fn test_mcp_sidecar_mode_filters_context_tools() {
    let vault = mock_obsidian_vault();

    let tools_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    // Default mode exposes all 11 tools.
    let mut server = McpServer::new(vault.path());
    assert!(!server.is_sidecar());

    let default_str = server
        .handle_line(&tools_req.to_string())
        .expect("Should respond to tools/list");
    let default_resp: Value = serde_json::from_str(&default_str).unwrap();
    let default_tools = default_resp["result"]["tools"].as_array().unwrap();
    assert_eq!(default_tools.len(), 11);

    // Sidecar mode exposes only the 8 orthogonal tools.
    let mut server = McpServer::new(vault.path()).with_sidecar(true);
    assert!(server.is_sidecar());

    let sidecar_str = server
        .handle_line(&tools_req.to_string())
        .expect("Should respond to tools/list");
    let sidecar_resp: Value = serde_json::from_str(&sidecar_str).unwrap();
    let sidecar_tools = sidecar_resp["result"]["tools"].as_array().unwrap();
    let sidecar_names: Vec<&str> = sidecar_tools
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();

    assert_eq!(
        sidecar_names,
        vec![
            "get_project_loadout",
            "recall_memory",
            "timeline_memory",
            "inspect_note",
            "flush_session",
            "distill_session_skill",
            "guard_check",
            "handoff_session",
        ]
    );
    assert_eq!(sidecar_tools.len(), 8);
    assert!(!sidecar_names.contains(&"offload_context"));
    assert!(!sidecar_names.contains(&"inspect_log_node"));
    assert!(!sidecar_names.contains(&"gc_session_context"));
}

#[test]
fn test_mcp_guard_check_enforces_repo_local_k0maru_wiki_rules() {
    let vault = mock_obsidian_vault();
    let repo = tempfile::tempdir().unwrap();

    let repo_wiki = repo.path().join(".k0maru").join("wiki");
    std::fs::create_dir_all(repo_wiki.join("20_Cards")).unwrap();
    std::fs::write(
        repo_wiki.join("20_Cards").join("adr-repo-local.md"),
        "---\ntitle: Repo Local ADR\ninvariants:\n  - id: REPO-LOCAL-001\n    title: Repo local lock\n    file_pattern: \"*.rs\"\n    forbidden: \"local_forbidden_call\"\n    remediation: \"Use the repo-local API.\"\n---\n# Repo Local ADR\n",
    )
    .unwrap();

    // The MCP server is mounted on the global vault, so the repository's own
    // `.k0maru/wiki` rules must be merged in explicitly.
    let mut server = McpServer::new(vault.path());

    let res = server.handle_tool_call(
        json!(17),
        &json!({
            "name": "guard_check",
            "arguments": {
                "repo_path": repo.path().to_str().unwrap(),
                "diff": "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n+fn x() { local_forbidden_call(); }\n"
            }
        }),
    );

    assert_eq!(
        res["result"]["isError"], true,
        "repo-local .k0maru/wiki rules must be enforced from a global MCP vault: {res}"
    );
    let text = res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("REPO-LOCAL-001"),
        "the repo-local rule must appear in the report, got: {text}"
    );
}

#[test]
fn test_mcp_timeline_memory_tool() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    let timeline_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "timeline_memory",
            "arguments": {
                "window": 5
            }
        }
    });

    let resp_str = server
        .handle_line(&timeline_req.to_string())
        .expect("Should respond to timeline_memory");
    let resp: Value = serde_json::from_str(&resp_str).unwrap();
    assert_ne!(resp["result"]["isError"], true);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("| Date | Kind | Title | Path | Status |"));
}

#[test]
fn test_mcp_inspect_note_tool_and_traversal_rejection() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    // 1. Valid note inspection
    let inspect_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "inspect_note",
            "arguments": {
                "path": "10_Projects/k0maru-memory.md"
            }
        }
    });

    let resp_str = server
        .handle_line(&inspect_req.to_string())
        .expect("Should respond to inspect_note");
    let resp: Value = serde_json::from_str(&resp_str).unwrap();
    assert_ne!(resp["result"]["isError"], true);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("# K0maru Agent Memory") || text.contains("10_Projects/k0maru-memory.md")
    );
    assert!(text.contains("## Body"));

    // 2. Path traversal rejection: ../Cargo.toml
    let traversal_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "inspect_note",
            "arguments": {
                "path": "../Cargo.toml"
            }
        }
    });

    let resp_str = server
        .handle_line(&traversal_req.to_string())
        .expect("Should respond to inspect_note traversal attempt");
    let resp: Value = serde_json::from_str(&resp_str).unwrap();
    assert_eq!(resp["result"]["isError"], true);
    let err_msg = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        err_msg.contains("traversal")
            || err_msg.contains("not allowed")
            || err_msg.contains("Failed to inspect"),
        "Error message should mention rejection: {}",
        err_msg
    );
}

#[test]
fn test_mcp_section_recall_locator_and_inspect_note_range() {
    let vault = mock_obsidian_vault();
    std::fs::write(
        vault.path().join("20_Cards/Gateway_Ledger.md"),
        "\
---
title: Gateway Ledger
hierarchy: L3Evergreen
---
# Gateway Ledger

Intro section.

## Settlement Quorum
Quorum verification requires three replicas.

## Idempotency Window
Idempotency token TTL is twenty-four hours for distributed-settlement replay protection.

## Compaction
Run nightly vacuum.
",
    )
    .unwrap();

    let mut server = McpServer::new(vault.path());

    // Compact recall returns section locator in header slot while preserving !μ: and <1200 budget
    let res_compact = server.handle_tool_call(
        json!(30),
        &json!({
            "name": "recall_memory",
            "arguments": {
                "query": "Idempotency token replay protection",
                "mode": "bm25"
            }
        }),
    );
    let compact_text = res_compact["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(compact_text.len() < 1200);
    let first_line = compact_text.lines().next().unwrap();
    assert!(first_line.starts_with("!μ:L3|20_Cards/Gateway_Ledger.md:"));
    assert!(first_line.contains("#Idempotency Window|score:"));

    // Extract the locator from `!μ:L3|<locator>|score:...`
    let locator = first_line.split('|').nth(1).unwrap();

    // Full mode formats `## {title} ({locator})`
    let res_full = server.handle_tool_call(
        json!(31),
        &json!({
            "name": "recall_memory",
            "arguments": {
                "query": "Idempotency token replay protection",
                "mode": "bm25",
                "full": true
            }
        }),
    );
    let full_text = res_full["result"]["content"][0]["text"].as_str().unwrap();
    assert!(full_text.contains(&format!("## Gateway Ledger ({})", locator)));

    // inspect_note with locator returns only that section's lines
    let res_inspect = server.handle_tool_call(
        json!(32),
        &json!({
            "name": "inspect_note",
            "arguments": {
                "path": locator
            }
        }),
    );
    let inspect_text = res_inspect["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(inspect_text.contains("## Idempotency Window"));
    assert!(inspect_text.contains("twenty-four hours"));
    assert!(!inspect_text.contains("## Compaction"));

    // inspect_note with full_body: true returns the whole body
    let res_inspect_full = server.handle_tool_call(
        json!(33),
        &json!({
            "name": "inspect_note",
            "arguments": {
                "path": locator,
                "full_body": true
            }
        }),
    );
    let inspect_full_text = res_inspect_full["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(inspect_full_text.contains("## Settlement Quorum"));
    assert!(inspect_full_text.contains("## Compaction"));
}

#[test]
fn test_mcp_budgeted_section_backfill_caps_at_64() {
    let vault = mock_obsidian_vault();
    // Create a note with 72 sections so a single MCP call must cap section embedding at 64.
    let mut big_doc = String::from("# Big Gateway Spec\n\n");
    for i in 1..=72 {
        big_doc.push_str(&format!(
            "## Section {}\nDistributed settlement checkpoint {}.\n\n",
            i, i
        ));
    }
    std::fs::write(vault.path().join("20_Cards/BigSpec.md"), big_doc).unwrap();

    let embedder = std::sync::Arc::new(k0maru::MockEmbeddingEngine::new(384));
    let cache_path = vault.path().join(".k0maru").join("cache.sqlite");
    {
        let mut storage = k0maru::SqliteStorage::open_rw(&cache_path).unwrap();
        storage.ensure_vector_dimension(384).unwrap();
        let adapter = k0maru::ObsidianAdapter::new(vault.path());
        let mut scanner = k0maru::IncrementalScanner::new(&adapter, &mut storage);
        scanner.sync(false).unwrap();
        // Sync only doc vectors so section_vectors has 0 rows prior to MCP recall
        k0maru::VectorSyncEngine::sync(&mut storage, &*embedder).unwrap();
    }

    let mut server = McpServer::new(vault.path()).with_embedder(embedder);
    let res = server.handle_tool_call(
        json!(34),
        &json!({
            "name": "recall_memory",
            "arguments": {
                "query": "Distributed settlement checkpoint"
            }
        }),
    );
    assert_ne!(res["result"]["isError"], true);

    let storage = k0maru::SqliteStorage::open(&cache_path).unwrap();
    let sec_vec_count: i64 = storage
        .connection()
        .query_row("SELECT COUNT(*) FROM section_vectors", [], |r| r.get(0))
        .unwrap();
    assert!(
        sec_vec_count <= 64 && sec_vec_count > 0,
        "MCP sync_cache_with_storage must cap section embedding at 64 (got {})",
        sec_vec_count
    );
    assert_eq!(
        storage
            .meta_get("section_vectors_complete")
            .unwrap()
            .as_deref(),
        Some("0")
    );
}

#[test]
fn test_mcp_federated_recall_inspect_loadout_guard_and_readonly_proof() {
    let primary_vault = mock_obsidian_vault();
    let global_dir = tempfile::tempdir().unwrap();
    let global_vault = global_dir.path();

    std::fs::create_dir_all(global_vault.join("20_Cards")).unwrap();
    std::fs::write(
        global_vault.join("20_Cards/GlobalSettlement.md"),
        "\
---
title: Global Settlement Rules
hierarchy: L3Evergreen
invariants:
  - id: ADR-GLOBAL-MCP
    title: Require fenced lease
    file_pattern: \"*.rs\"
    forbidden: \"mcp_global_forbidden_fn\"
    remediation: \"Use fenced_lease_acquire instead.\"
---
# Global Settlement Rules

## Core Concept
> Every k0maru-memory settlement worker must hold a fenced lease.

## Quorum Check
Quorum requires three independent settlement replicas.

## Cleanup
Archive stale checkpoints.
",
    )
    .unwrap();
    std::fs::write(
        global_vault.join("20_Cards/GlobalReferrer.md"),
        "\
---
title: Global Referrer
hierarchy: L3Evergreen
---
# Global Referrer

References [[GlobalSettlement]].
",
    )
    .unwrap();

    // Sync global vault into its own cache.sqlite beforehand.
    let global_cache = global_vault.join(".k0maru").join("cache.sqlite");
    {
        let mut storage = k0maru::SqliteStorage::open_rw(&global_cache).unwrap();
        let adapter = k0maru::ObsidianAdapter::new(global_vault);
        let mut scanner = k0maru::IncrementalScanner::new(&adapter, &mut storage);
        scanner.sync(false).unwrap();
    }

    let before_meta = std::fs::metadata(&global_cache).unwrap();
    let before_wal = std::fs::metadata(global_vault.join(".k0maru/cache.sqlite-wal"))
        .map(|m| m.len())
        .unwrap_or(0);
    std::thread::sleep(std::time::Duration::from_millis(20));

    // Unconfigured server rejects @global: inspect_note.
    let mut unconfigured = McpServer::new(primary_vault.path());
    let unconf_res = unconfigured.handle_tool_call(
        json!(40),
        &json!({
            "name": "inspect_note",
            "arguments": { "path": "@global:20_Cards/GlobalSettlement.md" }
        }),
    );
    assert_eq!(unconf_res["result"]["isError"], true);

    // Configured federated server.
    let mut server =
        McpServer::new(primary_vault.path()).with_global_vault(Some(global_vault.to_path_buf()));

    // 1. recall_memory compact: @global: locator and global backlink count dispatched from ReadOnlyStorage.
    let recall_compact = server.handle_tool_call(
        json!(41),
        &json!({
            "name": "recall_memory",
            "arguments": {
                "query": "three independent settlement replicas",
                "mode": "bm25"
            }
        }),
    );
    assert_ne!(recall_compact["result"]["isError"], true);
    let compact_text = recall_compact["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let global_line = compact_text
        .lines()
        .find(|l| l.contains("@global:20_Cards/GlobalSettlement.md"))
        .expect("compact recall must include @global: hit");
    assert!(
        global_line.contains("|links:1/0"),
        "backlinks must be dispatched to global ReadOnlyStorage: {}",
        global_line
    );
    let locator = global_line.split('|').nth(1).unwrap();

    // 2. recall_memory full: dispatches backlinks & hierarchy from global ReadOnlyStorage.
    let recall_full = server.handle_tool_call(
        json!(42),
        &json!({
            "name": "recall_memory",
            "arguments": {
                "query": "three independent settlement replicas",
                "mode": "bm25",
                "full": true
            }
        }),
    );
    let full_text = recall_full["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(full_text.contains("@global:20_Cards/GlobalSettlement.md"));
    assert!(full_text.contains("[[20_Cards/GlobalReferrer.md]]"));
    assert!(full_text.contains("Hierarchy: L3Evergreen"));

    // 3. inspect_note on @global: locator slices exact section lines and rejects traversal.
    let inspect_res = server.handle_tool_call(
        json!(43),
        &json!({
            "name": "inspect_note",
            "arguments": { "path": locator }
        }),
    );
    assert_ne!(inspect_res["result"]["isError"], true);
    let inspect_text = inspect_res["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(inspect_text.contains("## Quorum Check"));
    assert!(!inspect_text.contains("## Cleanup"));
    assert!(inspect_text.contains("20_Cards/GlobalReferrer.md"));

    let traversal_res = server.handle_tool_call(
        json!(44),
        &json!({
            "name": "inspect_note",
            "arguments": { "path": "@global:../Cargo.toml" }
        }),
    );
    assert_eq!(traversal_res["result"]["isError"], true);

    // 4. get_project_loadout appends Global invariants within 300 tokens.
    let loadout_res = server.handle_tool_call(
        json!(45),
        &json!({
            "name": "get_project_loadout",
            "arguments": { "project_name": "k0maru-memory" }
        }),
    );
    assert_ne!(loadout_res["result"]["isError"], true);
    let loadout_text = loadout_res["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(loadout_text.contains("## Global invariants"));
    assert!(loadout_text.contains("@global:20_Cards/GlobalSettlement.md"));
    assert!(k0maru::loadout::estimate_tokens(loadout_text) <= 300);

    // 5. guard_check tags global-origin rule IDs with [global] in report text.
    let guard_res = server.handle_tool_call(
        json!(46),
        &json!({
            "name": "guard_check",
            "arguments": {
                "diff": "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n+fn f() { mcp_global_forbidden_fn(); }\n"
            }
        }),
    );
    assert_eq!(guard_res["result"]["isError"], true);
    let guard_text = guard_res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        guard_text.contains("[DENY] ADR-GLOBAL-MCP [global]:"),
        "expected [global] tag in guard_check report text:\n{}",
        guard_text
    );

    // 6. flush_session writes ONLY to primary vault.
    let flush_res = server.handle_tool_call(
        json!(47),
        &json!({
            "name": "flush_session",
            "arguments": {
                "title": "Primary Only MCP Flush",
                "content": "Never writes to global."
            }
        }),
    );
    assert_ne!(flush_res["result"]["isError"], true);
    assert!(!global_vault.join("01_AI_Logs").exists());

    // 7. Global cache.sqlite size, mtime, -wal size, and doc_access remain completely untouched.
    let after_meta = std::fs::metadata(&global_cache).unwrap();
    let after_wal = std::fs::metadata(global_vault.join(".k0maru/cache.sqlite-wal"))
        .map(|m| m.len())
        .unwrap_or(0);
    assert_eq!(before_meta.len(), after_meta.len());
    assert_eq!(before_meta.modified().ok(), after_meta.modified().ok());
    assert_eq!(before_wal, after_wal);

    let ro = k0maru::ReadOnlyStorage::open(&global_cache).unwrap();
    let access_rows: i64 = ro
        .storage()
        .connection()
        .query_row("SELECT COUNT(*) FROM doc_access", [], |r| r.get(0))
        .unwrap();
    assert_eq!(access_rows, 0);
}

#[test]
fn test_mcp_handoff_session_tool_lifecycle() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    // 1. create
    let create_res = server.handle_tool_call(
        json!(50),
        &json!({
            "name": "handoff_session",
            "arguments": {
                "action": "create",
                "from_agent": "agent-1",
                "to_agent": "agent-2",
                "project": "payment-gateway",
                "title": "Finish ledger compaction",
                "goal": "Compact WAL segments",
                "state": "in_progress",
                "next_steps": ["Run vacuum", "Verify checksums"],
                "json": true
            }
        }),
    );
    assert_ne!(create_res["result"]["isError"], true, "{create_res}");
    let created: Value =
        serde_json::from_str(create_res["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    let handoff_id = created["id"].as_str().unwrap().to_string();

    // 2. list
    let list_res = server.handle_tool_call(
        json!(51),
        &json!({
            "name": "handoff_session",
            "arguments": {
                "action": "list",
                "project": "payment-gateway"
            }
        }),
    );
    assert_ne!(list_res["result"]["isError"], true);
    assert!(list_res["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains(&handoff_id));

    // 3. show
    let show_res = server.handle_tool_call(
        json!(52),
        &json!({
            "name": "handoff_session",
            "arguments": {
                "action": "show",
                "id": handoff_id
            }
        }),
    );
    assert_ne!(show_res["result"]["isError"], true);
    assert!(show_res["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Finish ledger compaction"));

    // 4. claim succeeds once, fails on second attempt
    let claim_ok = server.handle_tool_call(
        json!(53),
        &json!({
            "name": "handoff_session",
            "arguments": {
                "action": "claim",
                "agent": "agent-2",
                "id": handoff_id
            }
        }),
    );
    assert_ne!(claim_ok["result"]["isError"], true);
    assert!(claim_ok["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("claimed by agent-2"));

    let claim_dup = server.handle_tool_call(
        json!(54),
        &json!({
            "name": "handoff_session",
            "arguments": {
                "action": "claim",
                "agent": "agent-2",
                "id": handoff_id
            }
        }),
    );
    assert_eq!(claim_dup["result"]["isError"], true);
}
