use assert_cmd::Command;
use k0maru::mcp::McpServer;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;
use tempfile::tempdir;

fn init_git_repo(path: &Path) {
    let out = StdCommand::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("git init failed");
    assert!(out.status.success());

    let _ = StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output();
    let _ = StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output();
}

#[test]
fn test_mcp_guard_check_flags_violation() {
    let vault_dir = tempdir().unwrap();
    let k0maru_dir = vault_dir.path().join(".k0maru");
    fs::create_dir_all(&k0maru_dir).unwrap();

    fs::write(
        k0maru_dir.join("guard.toml"),
        r#"
[[rules]]
id = "ADR-SETTLE-001"
title = "Mandatory Distributed Lock"
file_pattern = "*.go"
severity = "deny"
forbidden = "execWithoutLock"
remediation = "Acquire Redis lock before modifying ledger."
"#,
    )
    .unwrap();

    let mut server = McpServer::new(vault_dir.path());

    let dirty_diff = r#"
diff --git a/settle.go b/settle.go
new file mode 100644
--- /dev/null
+++ b/settle.go
@@ -0,0 +1,5 @@
+package settle
+func SettleLedger() {
+    execWithoutLock()
+}
"#;

    let res = server.handle_tool_call(
        json!(1),
        &json!({
            "name": "guard_check",
            "arguments": {
                "diff": dirty_diff
            }
        }),
    );

    let content_text = res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        content_text.contains("ADR-SETTLE-001"),
        "Must flag ADR-SETTLE-001 violation"
    );
    assert!(
        content_text.contains("execWithoutLock"),
        "Must identify offending content"
    );
    assert!(
        content_text.contains("Acquire Redis lock before modifying ledger"),
        "Must include remediation instructions"
    );
    assert!(
        content_text.contains("\"is_passed\": false")
            || content_text.contains("\"is_passed\":false"),
        "Report JSON must indicate is_passed = false"
    );
}

#[test]
fn test_mcp_guard_check_passes_clean_diff() {
    let vault_dir = tempdir().unwrap();
    let k0maru_dir = vault_dir.path().join(".k0maru");
    fs::create_dir_all(&k0maru_dir).unwrap();

    fs::write(
        k0maru_dir.join("guard.toml"),
        r#"
[[rules]]
id = "ADR-SETTLE-001"
title = "Mandatory Distributed Lock"
file_pattern = "*.go"
severity = "deny"
forbidden = "execWithoutLock"
must_contain = "redis.Lock"
remediation = "Acquire Redis lock before modifying ledger."
"#,
    )
    .unwrap();

    let mut server = McpServer::new(vault_dir.path());

    let clean_diff = r#"
diff --git a/settle.go b/settle.go
new file mode 100644
--- /dev/null
+++ b/settle.go
@@ -0,0 +1,5 @@
+package settle
+func SettleLedger() {
+    redis.Lock()
+}
"#;

    let res = server.handle_tool_call(
        json!(2),
        &json!({
            "name": "guard_check",
            "arguments": {
                "diff": clean_diff
            }
        }),
    );

    let content_text = res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        content_text.contains("All architectural invariants passed!"),
        "Clean diff must pass"
    );
    assert!(
        content_text.contains("\"is_passed\": true") || content_text.contains("\"is_passed\":true"),
        "Report JSON must indicate is_passed = true"
    );
}

#[test]
fn test_cli_pre_commit_hook_blocks_git_commit_on_violation() {
    let dir = tempdir().unwrap();
    init_git_repo(dir.path());

    // 1. Install hook
    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.current_dir(dir.path())
        .args(["guard", "install", "--path", dir.path().to_str().unwrap()])
        .assert()
        .success();

    // 2. Add guard rule
    let k0maru_dir = dir.path().join(".k0maru");
    fs::create_dir_all(&k0maru_dir).unwrap();
    fs::write(
        k0maru_dir.join("guard.toml"),
        r#"
[[rules]]
id = "ADR-BLOCK-001"
title = "No unwrap in commit"
file_pattern = "*.rs"
severity = "deny"
forbidden = "\\.unwrap\\(\\)"
remediation = "Handle errors with ?"
"#,
    )
    .unwrap();

    // Initial safe commit (can skip hook or commit rule file)
    let _ = StdCommand::new("git")
        .args(["add", ".k0maru/guard.toml"])
        .current_dir(dir.path())
        .output();
    let _ = StdCommand::new("git")
        .args(["commit", "-m", "init rule", "--no-verify"])
        .current_dir(dir.path())
        .output();

    // Create a violation file
    fs::write(
        dir.path().join("service.rs"),
        "fn calculate() {\n    let val = parse().unwrap();\n}\n",
    )
    .unwrap();

    let _ = StdCommand::new("git")
        .args(["add", "service.rs"])
        .current_dir(dir.path())
        .output();

    // Ensure k0maru binary is on PATH for hook execution
    let bin_path = assert_cmd::cargo::cargo_bin("k0maru");
    let target_bin_dir = bin_path.parent().unwrap();
    let current_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", target_bin_dir.display(), current_path);

    // Run git commit, hook should block it
    let commit_res = StdCommand::new("git")
        .args(["commit", "-m", "violating commit"])
        .current_dir(dir.path())
        .env("PATH", &new_path)
        .output()
        .expect("git commit invocation failed");

    assert!(
        !commit_res.status.success(),
        "Git commit must be blocked by k0maru guard pre-commit hook"
    );

    // Now fix violation
    fs::write(
        dir.path().join("service.rs"),
        "fn calculate() -> Result<(), ()> {\n    let val = parse()?;\n    Ok(())\n}\n",
    )
    .unwrap();

    let _ = StdCommand::new("git")
        .args(["add", "service.rs"])
        .current_dir(dir.path())
        .output();

    let clean_commit_res = StdCommand::new("git")
        .args(["commit", "-m", "compliant commit"])
        .current_dir(dir.path())
        .env("PATH", &new_path)
        .output()
        .expect("git commit invocation failed");

    assert!(
        clean_commit_res.status.success(),
        "Git commit must succeed once violation is fixed"
    );
}
