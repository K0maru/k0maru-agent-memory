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
fn test_guard_install_pre_commit_hook() {
    let dir = tempdir().unwrap();
    init_git_repo(dir.path());

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.current_dir(dir.path())
        .args(["guard", "install", "--path", dir.path().to_str().unwrap()])
        .assert()
        .success();

    let hook_path = dir.path().join(".git").join("hooks").join("pre-commit");
    assert!(hook_path.exists(), "pre-commit hook file must be created");

    let content = fs::read_to_string(&hook_path).unwrap();
    assert!(content.contains("k0maru guard check --diff"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::metadata(&hook_path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o111, 0o111, "hook must be executable");
    }
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
    assert!(content_text.contains("ADR-SETTLE-001"));
    assert!(content_text.contains("execWithoutLock"));
    assert!(content_text.contains("\"is_passed\": false"));
}

#[test]
fn test_guard_cli_check_clean_and_deny_violations() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let k0maru_dir = root.join(".k0maru");
    fs::create_dir_all(&k0maru_dir).unwrap();
    fs::write(
        k0maru_dir.join("guard.toml"),
        r#"
[[rules]]
id = "ADR-001"
title = "No Raw SQL"
file_pattern = "*.rs"
severity = "deny"
forbidden = "execute_raw"
remediation = "Use query builder."
"#,
    )
    .unwrap();

    // Clean check passes (exit 0)
    let mut cmd_clean = Command::cargo_bin("k0maru").unwrap();
    cmd_clean
        .current_dir(root)
        .args(["guard", "check", "--vault", root.to_str().unwrap()])
        .assert()
        .success();

    // Stdin diff with violation fails (exit 1)
    let bad_diff = r#"
diff --git a/src/db.rs b/src/db.rs
--- a/src/db.rs
+++ b/src/db.rs
@@ -1,1 +1,2 @@
+    db.execute_raw("DROP TABLE users");
"#;

    let mut cmd_fail = Command::cargo_bin("k0maru").unwrap();
    cmd_fail
        .current_dir(root)
        .args([
            "guard",
            "check",
            "--vault",
            root.to_str().unwrap(),
            "--diff",
        ])
        .write_stdin(bad_diff)
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_guard_cli_and_mcp_kdsl_check() {
    let vault_dir = tempdir().unwrap();
    let root = vault_dir.path();
    let k0maru_dir = root.join(".k0maru");
    fs::create_dir_all(&k0maru_dir).unwrap();

    fs::write(
        k0maru_dir.join("guard.toml"),
        r#"
[[invariants]]
id = "SEC-004"
title = "Enforce RS256"
paths = ["src/middleware/*.ts"]
forbidden = "HS256"
severity = "deny"
remediation = "Use RS256 asymmetric signatures"
"#,
    )
    .unwrap();

    // 1. CLI with clean K-DSL packet
    let clean_wire = "!δ:0.1|AUTH-SRV|src/middleware/auth.ts#v4182 §AST:F[verifyToken] @OP:SET_PROP (Call[jwt.verify].Arg[2].Prop[algorithms]) -> ([\"RS256\"]) !Σ:VERIFY(LIVING_ADR#SEC-004)";
    let mut cmd_clean = Command::cargo_bin("k0maru").unwrap();
    cmd_clean
        .current_dir(root)
        .args([
            "guard",
            "check",
            "--vault",
            root.to_str().unwrap(),
            "--kdsl",
            clean_wire,
        ])
        .assert()
        .success();

    // 2. CLI with violating K-DSL packet
    let bad_wire = "!δ:0.1|AUTH-SRV|src/middleware/auth.ts#v4182 §AST:F[verifyToken] @OP:SET_PROP (Call[jwt.verify].Arg[2].Prop[algorithms]) -> ([\"HS256\"]) !Σ:VERIFY(LIVING_ADR#SEC-004)";
    let mut cmd_bad = Command::cargo_bin("k0maru").unwrap();
    cmd_bad
        .current_dir(root)
        .args([
            "guard",
            "check",
            "--vault",
            root.to_str().unwrap(),
            "--kdsl",
            bad_wire,
        ])
        .assert()
        .failure()
        .code(1);

    // 3. FastMCP tool with K-DSL
    let mut server = McpServer::new(root);

    let res_clean = server.handle_tool_call(
        json!(1),
        &json!({
            "name": "guard_check",
            "arguments": {
                "kdsl": clean_wire
            }
        }),
    );
    let clean_text = res_clean["result"]["content"][0]["text"].as_str().unwrap();
    assert!(clean_text.contains("All architectural invariants passed"));

    let res_bad = server.handle_tool_call(
        json!(2),
        &json!({
            "name": "guard_check",
            "arguments": {
                "kdsl": bad_wire
            }
        }),
    );
    let bad_text = res_bad["result"]["content"][0]["text"].as_str().unwrap();
    assert!(bad_text.contains("VIOLATIONS DETECTED") || bad_text.contains("SEC-004"));
}
