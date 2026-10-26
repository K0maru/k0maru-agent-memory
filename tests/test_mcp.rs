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

    // Default mode exposes all 8 tools.
    let mut server = McpServer::new(vault.path());
    assert!(!server.is_sidecar());

    let default_str = server
        .handle_line(&tools_req.to_string())
        .expect("Should respond to tools/list");
    let default_resp: Value = serde_json::from_str(&default_str).unwrap();
    let default_tools = default_resp["result"]["tools"].as_array().unwrap();
    assert_eq!(default_tools.len(), 8);

    // Sidecar mode exposes only the 5 core orthogonal tools.
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
            "flush_session",
            "distill_session_skill",
            "guard_check",
        ]
    );
    assert_eq!(sidecar_tools.len(), 5);
    assert!(!sidecar_names.contains(&"offload_context"));
    assert!(!sidecar_names.contains(&"inspect_log_node"));
    assert!(!sidecar_names.contains(&"gc_session_context"));
}
