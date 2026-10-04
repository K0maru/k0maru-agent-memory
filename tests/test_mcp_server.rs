mod common;

use std::io::Cursor;

use common::fixtures::{mock_karpathy_wiki, mock_obsidian_vault};
use k0maru::mcp::McpServer;
use serde_json::{json, Value};

#[test]
fn test_mcp_initialize() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    let req = json!({
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
        .handle_line(&req.to_string())
        .expect("Should respond to initialize");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON response");

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(resp["result"]["serverInfo"]["name"], "k0maru");
    assert_eq!(resp["result"]["serverInfo"]["version"], k0maru::VERSION);
    assert!(resp["result"]["capabilities"]["tools"].is_object());
}

#[test]
fn test_mcp_notifications_initialized() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    // notifications/initialized must be silently acknowledged (no JSON-RPC response returned)
    let notif1 = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    assert!(server.handle_line(&notif1.to_string()).is_none());

    // Also support bare "initialized" notification
    let notif2 = json!({
        "jsonrpc": "2.0",
        "method": "initialized"
    });
    assert!(server.handle_line(&notif2.to_string()).is_none());
}

#[test]
fn test_mcp_tools_list() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    let req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    let resp_str = server
        .handle_line(&req.to_string())
        .expect("Should respond to tools/list");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON response");

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 2);

    let tools = resp["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 5);

    let tool_names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().expect("tool name"))
        .collect();

    assert!(tool_names.contains(&"get_project_loadout"));
    assert!(tool_names.contains(&"recall_memory"));
    assert!(tool_names.contains(&"offload_context"));
    assert!(tool_names.contains(&"inspect_log_node"));
    assert!(tool_names.contains(&"flush_session"));

    // Verify schemas
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        let schema = &tool["inputSchema"];
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().expect("required array");
        match name {
            "get_project_loadout" => {
                assert!(required.iter().any(|r| r == "project_name"));
            }
            "recall_memory" => {
                assert!(required.iter().any(|r| r == "query"));
            }
            "offload_context" => {
                assert!(required.iter().any(|r| r == "raw_text"));
            }
            "inspect_log_node" => {
                assert!(required.iter().any(|r| r == "node_id"));
            }
            "flush_session" => {
                assert!(required.iter().any(|r| r == "title"));
                assert!(required.iter().any(|r| r == "content"));
            }
            _ => panic!("Unexpected tool: {}", name),
        }
    }
}

#[test]
fn test_mcp_call_get_project_loadout() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    let req = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "get_project_loadout",
            "arguments": {
                "project_name": "K0maru Agent Memory"
            }
        }
    });

    let resp_str = server
        .handle_line(&req.to_string())
        .expect("Should respond to tools/call");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON response");

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 3);
    assert_ne!(resp["result"]["isError"], true);

    let content = resp["result"]["content"].as_array().expect("content array");
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "text");

    let text = content[0]["text"].as_str().expect("text string");
    assert!(text.contains("k0maru-memory"));
    assert!(text.contains("Architecture_Decisions"));
}

#[test]
fn test_mcp_call_recall_memory_fts() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    let req = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "recall_memory",
            "arguments": {
                "query": "Architecture",
                "limit": 5
            }
        }
    });

    let resp_str = server
        .handle_line(&req.to_string())
        .expect("Should respond to recall_memory");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON response");

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 4);
    assert_ne!(resp["result"]["isError"], true);

    let content = resp["result"]["content"].as_array().expect("content array");
    assert_eq!(content.len(), 1);
    let text = content[0]["text"].as_str().expect("text string");

    // Must recall the note mentioning Architecture
    assert!(text.contains("Architecture Decisions"));
    assert!(text.contains("Truth in files"));
}

#[test]
fn test_mcp_call_offload_and_inspect_e2e() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    // Generate 60 lines of text (exceeding default threshold of 50)
    let raw_text = (1..=60)
        .map(|i| format!("Step {:02}: processing data chunk #{}", i, i))
        .collect::<Vec<_>>()
        .join("\n");

    let offload_req = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "offload_context",
            "arguments": {
                "raw_text": raw_text,
                "task_name": "mcp_e2e_test"
            }
        }
    });

    let resp_str = server
        .handle_line(&offload_req.to_string())
        .expect("Should respond to offload_context");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON response");

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 5);
    assert_ne!(resp["result"]["isError"], true);

    let content = resp["result"]["content"].as_array().expect("content array");
    let text = content[0]["text"].as_str().expect("text string");

    // Must return Mermaid graph and node ID
    assert!(text.contains("stateDiagram-v2"));
    assert!(text.contains("Offloaded"));
    assert!(text.contains("node_"));

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

    // Now test inspect_log_node with the extracted node ID
    let inspect_req = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "tools/call",
        "params": {
            "name": "inspect_log_node",
            "arguments": {
                "node_id": node_id
            }
        }
    });

    let inspect_resp_str = server
        .handle_line(&inspect_req.to_string())
        .expect("Should respond to inspect_log_node");
    let inspect_resp: Value = serde_json::from_str(&inspect_resp_str).expect("Valid JSON response");

    assert_eq!(inspect_resp["jsonrpc"], "2.0");
    assert_eq!(inspect_resp["id"], 6);
    assert_ne!(inspect_resp["result"]["isError"], true);

    let retrieved_content = inspect_resp["result"]["content"][0]["text"]
        .as_str()
        .expect("retrieved log");
    assert!(retrieved_content.contains("Step 01: processing data chunk #1"));
    assert!(retrieved_content.contains("Step 60: processing data chunk #60"));
}

#[test]
fn test_mcp_unknown_method_error() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    let req = json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "unknown_rpc_method"
    });

    let resp_str = server
        .handle_line(&req.to_string())
        .expect("Should return error");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON response");

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 99);
    assert_eq!(resp["error"]["code"], -32601);
    assert_eq!(resp["error"]["message"], "Method not found");
}

#[test]
fn test_mcp_invalid_json_parse_error() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    let invalid_json = "{ invalid_json: true ";
    let resp_str = server
        .handle_line(invalid_json)
        .expect("Should return parse error");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON response");

    assert_eq!(resp["jsonrpc"], "2.0");
    assert!(resp["id"].is_null());
    assert_eq!(resp["error"]["code"], -32700);
    assert_eq!(resp["error"]["message"], "Parse error");
}

#[test]
fn test_mcp_run_stdio_stream() {
    let vault = mock_obsidian_vault();
    let mut server = McpServer::new(vault.path());

    let input_lines = format!(
        "{}\n{}\n{}\n",
        json!({"jsonrpc": "2.0", "id": 10, "method": "initialize"}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 11, "method": "tools/list"})
    );

    let reader = Cursor::new(input_lines);
    let mut writer = Vec::new();

    server
        .run_stdio(reader, &mut writer)
        .expect("run_stdio succeeds");

    let output_str = String::from_utf8(writer).expect("Valid UTF-8 output");
    let output_lines: Vec<&str> = output_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    // 2 responses (initialize, tools/list) because notifications/initialized produces no response
    assert_eq!(output_lines.len(), 2);

    let resp1: Value = serde_json::from_str(output_lines[0]).expect("Valid JSON line 1");
    assert_eq!(resp1["id"], 10);
    assert_eq!(resp1["result"]["protocolVersion"], "2024-11-05");

    let resp2: Value = serde_json::from_str(output_lines[1]).expect("Valid JSON line 2");
    assert_eq!(resp2["id"], 11);
    assert!(resp2["result"]["tools"].is_array());
}

#[test]
fn test_mcp_with_karpathy_wiki() {
    let wiki = mock_karpathy_wiki();
    let mut server = McpServer::new(wiki.path());

    let req = json!({
        "jsonrpc": "2.0",
        "id": 20,
        "method": "tools/call",
        "params": {
            "name": "recall_memory",
            "arguments": {
                "query": "sqlite",
                "limit": 3
            }
        }
    });

    let resp_str = server
        .handle_line(&req.to_string())
        .expect("Should respond to recall_memory");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON response");
    assert_eq!(resp["id"], 20);
    assert_ne!(resp["result"]["isError"], true);
    let text = resp["result"]["content"][0]["text"].as_str().expect("text");
    assert!(text.contains("SQLite FTS5 Notes"));
}

#[test]
fn test_cli_mcp_stdio_e2e() {
    use assert_cmd::Command;

    let vault = mock_obsidian_vault();
    let vault_path = vault.path().to_str().unwrap();

    let input_req = format!(
        "{}\n{}\n",
        json!({"jsonrpc": "2.0", "id": 100, "method": "initialize"}),
        json!({"jsonrpc": "2.0", "id": 101, "method": "tools/call", "params": {"name": "recall_memory", "arguments": {"query": "Architecture"}}})
    );

    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    let assert = cmd
        .args(["mcp", "--vault", vault_path])
        .write_stdin(input_req)
        .assert()
        .success();

    let stdout_bytes = assert.get_output().stdout.clone();
    let stdout_str = String::from_utf8(stdout_bytes).expect("Valid UTF-8 stdout");
    let lines: Vec<&str> = stdout_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    assert_eq!(lines.len(), 2);

    let init_resp: Value = serde_json::from_str(lines[0]).expect("Line 1 JSON");
    assert_eq!(init_resp["id"], 100);
    assert_eq!(init_resp["result"]["serverInfo"]["name"], "k0maru");

    let recall_resp: Value = serde_json::from_str(lines[1]).expect("Line 2 JSON");
    assert_eq!(recall_resp["id"], 101);
    let text = recall_resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(text.contains("Architecture_Decisions"));
}
