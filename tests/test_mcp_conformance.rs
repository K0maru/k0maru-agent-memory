use std::io::Write;
use std::process::{Command, Stdio};

use tempfile::tempdir;

#[test]
fn test_mcp_full_pipeline_conformance() {
    let temp_vault = tempdir().expect("tempdir");
    let vault = temp_vault.path();

    // Initialize mock database cache schema so instant search re-index works seamlessly
    let cache_dir = vault.join(".k0maru");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let cache_file = cache_dir.join("cache.sqlite");
    let _storage = k0maru::storage::SqliteStorage::open(&cache_file).unwrap();

    // Establish structured vault convention
    std::fs::create_dir_all(vault.join("decisions")).unwrap();
    std::fs::create_dir_all(vault.join("skills")).unwrap();

    let mut child = Command::new(assert_cmd::cargo::cargo_bin("k0maru"))
        .args(["mcp", "--vault", vault.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn k0maru mcp process");

    let mut stdin = child.stdin.take().expect("take stdin");

    // 1. initialize
    let req1 = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "hermes-agent",
                "version": "0.1.0"
            }
        }
    });

    // 2. notifications/initialized
    let req2 = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });

    // 3. tools/list
    let req3 = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    // 4. tools/call: flush_session
    let req4 = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "flush_session",
            "arguments": {
                "title": "Hermes Sandboxed Decision",
                "content": "Verified zero host pollution in sandbox test",
                "category": "decision",
                "summary": "Verified zero host pollution in sandbox test",
                "tags": ["hermes", "sandbox"]
            }
        }
    });

    // 5. tools/call: recall_memory
    let req5 = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "recall_memory",
            "arguments": {
                "query": "Hermes Sandboxed Decision"
            }
        }
    });

    // 6. tools/call: distill_session_skill
    let req6 = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "distill_session_skill",
            "arguments": {
                "title": "Resolve Borrow Checker Conflict",
                "raw_trace": "error[E0382]: borrow of moved value: `data`\n  --> src/main.rs:12:9\nfix: clone before move",
                "tags": ["rust", "hermes"]
            }
        }
    });

    let input_lines = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n",
        req1, req2, req3, req4, req5, req6
    );

    stdin
        .write_all(input_lines.as_bytes())
        .expect("write stdin");
    drop(stdin); // Send EOF to terminate stdio loop

    let output = child.wait_with_output().expect("wait on child process");
    assert!(
        output.status.success(),
        "process should exit cleanly with 0"
    );

    let stdout_str = String::from_utf8(output.stdout).expect("valid utf8 stdout");
    let lines: Vec<&str> = stdout_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    // We expect exactly 5 response lines (notifications produce no response line)
    assert_eq!(
        lines.len(),
        5,
        "stdout must contain exactly 5 JSON-RPC response lines, got: {}",
        stdout_str
    );

    // Verify all response lines parse as strict JSON-RPC 2.0 without corruption
    let mut responses = Vec::new();
    for line in &lines {
        let val: serde_json::Value =
            serde_json::from_str(line).expect("every stdout line must be strict JSON-RPC");
        assert_eq!(val["jsonrpc"], "2.0");
        responses.push(val);
    }

    // Response 1: initialize
    assert_eq!(responses[0]["id"], 1);
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "k0maru");

    // Response 2: tools/list (6 registered tools)
    assert_eq!(responses[1]["id"], 2);
    let tools = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools array");
    assert_eq!(tools.len(), 6);
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(tool_names.contains(&"get_project_loadout"));
    assert!(tool_names.contains(&"recall_memory"));
    assert!(tool_names.contains(&"offload_context"));
    assert!(tool_names.contains(&"inspect_log_node"));
    assert!(tool_names.contains(&"flush_session"));
    assert!(tool_names.contains(&"distill_session_skill"));

    // Response 3: tools/call flush_session
    assert_eq!(responses[2]["id"], 3);
    assert!(
        responses[2]["result"]["isError"].is_null() || responses[2]["result"]["isError"] == false
    );
    let flush_text = responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        flush_text
            .to_lowercase()
            .contains("hermes-sandboxed-decision"),
        "flush response must confirm note path, got: {}",
        flush_text
    );

    // Response 4: tools/call recall_memory
    assert_eq!(responses[3]["id"], 4);
    let recall_text = responses[3]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        recall_text.contains("Hermes Sandboxed Decision"),
        "recall must retrieve newly flushed note, got: {}",
        recall_text
    );

    // Response 5: tools/call distill_session_skill
    assert_eq!(responses[4]["id"], 5);
    let distill_text = responses[4]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        distill_text.contains("Resolve Borrow Checker Conflict"),
        "distill must confirm skill creation"
    );

    // Verify physical vault files created
    let decisions_dir = vault.join("decisions");
    assert!(decisions_dir.exists(), "decisions directory must exist");
    let skills_dir = vault.join("skills");
    assert!(skills_dir.exists(), "skills directory must exist");
}
