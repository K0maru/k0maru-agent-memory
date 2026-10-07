use std::fs;

use k0maru::mcp::McpServer;
use serde_json::{json, Value};
use tempfile::tempdir;

#[test]
fn test_mcp_tools_list_contains_distill_session_skill() {
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
    let distill_tool = tools
        .iter()
        .find(|t| t["name"] == "distill_session_skill")
        .expect("distill_session_skill tool must be registered");

    assert!(distill_tool["inputSchema"]["properties"]["raw_trace"].is_object());
    assert!(distill_tool["inputSchema"]["properties"]["node_id"].is_object());
    assert!(distill_tool["inputSchema"]["properties"]["title"].is_object());
    assert!(distill_tool["inputSchema"]["properties"]["context_hint"].is_object());
    assert!(distill_tool["inputSchema"]["properties"]["category"].is_object());
    assert!(distill_tool["inputSchema"]["properties"]["tags"].is_object());
    assert!(distill_tool["inputSchema"]["properties"]["dry_run"].is_object());
}

#[test]
fn test_mcp_call_distill_session_skill_and_recall_e2e() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();
    fs::create_dir_all(vault_root.join("skills")).unwrap();

    let mut server = McpServer::new(vault_root);

    let trace = r#"
$ cargo test --test test_distill_engine
error[E0432]: unresolved import `foo`
$ cargo add foo
$ cargo test
test result: ok. 200 passed
"#;

    // 1. Call distill_session_skill tool via JSON-RPC
    let distill_call = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
            "name": "distill_session_skill",
            "arguments": {
                "raw_trace": trace,
                "title": "Resolve Missing Foo In Test Suite",
                "context_hint": "Missing dependency during test run",
                "tags": ["topic/rust", "topic/testing"],
                "related_notes": ["Hermes Dynamic Skill"],
                "dry_run": false
            }
        }
    });

    let resp_str = server
        .handle_line(&distill_call.to_string())
        .expect("Response expected");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON");

    assert_eq!(resp["id"], 10);
    assert!(resp["result"]["isError"].is_null() || resp["result"]["isError"] == false);

    let text_content = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("Text content");
    assert!(
        text_content.contains("✓ Crystallized skill note into vault"),
        "Response should confirm skill crystallization: {}",
        text_content
    );
    assert!(text_content.contains("skills/"));

    // 2. Immediately call recall_memory to verify instant searchability
    let recall_call = json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "tools/call",
        "params": {
            "name": "recall_memory",
            "arguments": {
                "query": "Resolve Missing Foo In Test Suite",
                "limit": 5
            }
        }
    });

    let recall_resp_str = server
        .handle_line(&recall_call.to_string())
        .expect("Recall response expected");
    let recall_resp: Value = serde_json::from_str(&recall_resp_str).expect("Valid JSON");

    let recall_text = recall_resp["result"]["content"][0]["text"]
        .as_str()
        .expect("Recall text content");
    assert!(
        recall_text.contains("Resolve Missing Foo In Test Suite"),
        "recall_memory should immediately return the distilled skill: {}",
        recall_text
    );
}
