use std::fs;

use k0maru::mcp::McpServer;
use serde_json::{json, Value};
use tempfile::tempdir;

#[test]
fn test_mcp_tools_list_contains_flush_session() {
    let temp = tempdir().unwrap();
    let mut server = McpServer::new(temp.path());

    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let resp_str = server
        .handle_line(&req.to_string())
        .expect("Should respond to tools/list");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON response");

    let tools = resp["result"]["tools"].as_array().expect("Tools array");
    let flush_tool = tools
        .iter()
        .find(|t| t["name"] == "flush_session")
        .expect("flush_session tool must be registered");

    assert_eq!(
        flush_tool["inputSchema"]["required"],
        json!(["title", "content"])
    );
    assert!(flush_tool["inputSchema"]["properties"]["title"].is_object());
    assert!(flush_tool["inputSchema"]["properties"]["content"].is_object());
    assert!(flush_tool["inputSchema"]["properties"]["summary"].is_object());
    assert!(flush_tool["inputSchema"]["properties"]["category"].is_object());
    assert!(flush_tool["inputSchema"]["properties"]["tags"].is_object());
    assert!(flush_tool["inputSchema"]["properties"]["related_notes"].is_object());
}

#[test]
fn test_mcp_call_flush_session_and_recall_memory_e2e() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();

    let mut server = McpServer::new(vault_root);

    // 1. Call flush_session tool via JSON-RPC
    let flush_call = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "flush_session",
            "arguments": {
                "title": "Subagent Protocol Architecture",
                "summary": "FastMCP integration testing protocol",
                "content": "Detailed specification of the FastMCP flush protocol and cache sync.",
                "category": "decision",
                "tags": ["mcp", "agent", "subagent"],
                "related_notes": ["Master Worker Model", "Adaptive Convention Engine"]
            }
        }
    });

    let resp_str = server
        .handle_line(&flush_call.to_string())
        .expect("Response expected");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON");

    assert_eq!(resp["id"], 2);
    assert!(resp["result"]["isError"].is_null() || resp["result"]["isError"] == false);

    let text_content = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("Text content");
    assert!(
        text_content.contains("✓ Crystallized note into vault"),
        "Response should confirm crystallization: {}",
        text_content
    );
    assert!(
        text_content.contains("[[Master Worker Model]]"),
        "Response should include WikiLinks: {}",
        text_content
    );
    assert!(
        text_content.contains("[[Adaptive Convention Engine]]"),
        "Response should include WikiLinks: {}",
        text_content
    );

    // Verify file actually exists on disk
    let files: Vec<_> = fs::read_dir(vault_root)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        !files.is_empty(),
        "At least one note file must exist on disk"
    );

    // 2. Call recall_memory to verify immediate searchability
    let recall_call = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "recall_memory",
            "arguments": {
                "query": "Subagent Protocol Architecture"
            }
        }
    });

    let recall_resp_str = server
        .handle_line(&recall_call.to_string())
        .expect("Response expected");
    let recall_resp: Value = serde_json::from_str(&recall_resp_str).expect("Valid JSON");

    let recall_text = recall_resp["result"]["content"][0]["text"]
        .as_str()
        .expect("Recall text");
    assert!(
        recall_text.contains("Subagent Protocol Architecture"),
        "Recalled memory should match note title: {}",
        recall_text
    );

    // 3. Call flush_session with identical arguments: deduplication check
    let flush_dup_call = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "flush_session",
            "arguments": {
                "title": "Subagent Protocol Architecture",
                "summary": "FastMCP integration testing protocol",
                "content": "Detailed specification of the FastMCP flush protocol and cache sync.",
                "category": "decision",
                "tags": ["mcp", "agent", "subagent"],
                "related_notes": ["Master Worker Model", "Adaptive Convention Engine"]
            }
        }
    });

    let dup_resp_str = server
        .handle_line(&flush_dup_call.to_string())
        .expect("Response expected");
    let dup_resp: Value = serde_json::from_str(&dup_resp_str).expect("Valid JSON");
    let dup_text = dup_resp["result"]["content"][0]["text"]
        .as_str()
        .expect("Dup text");
    assert!(
        dup_text.contains("identical content"),
        "Should indicate note is already up to date: {}",
        dup_text
    );
}

#[test]
fn test_mcp_flush_session_missing_required_args() {
    let temp = tempdir().unwrap();
    let mut server = McpServer::new(temp.path());

    // Missing 'content' parameter
    let invalid_call = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "flush_session",
            "arguments": {
                "title": "Incomplete Note"
            }
        }
    });

    let resp_str = server
        .handle_line(&invalid_call.to_string())
        .expect("Response expected");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON");

    assert_eq!(resp["result"]["isError"], true);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Missing required parameter 'content'"));
}
