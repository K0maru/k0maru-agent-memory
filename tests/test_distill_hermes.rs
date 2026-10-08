use std::str::FromStr;
use tempfile::tempdir;

use k0maru::distill::{DistillEngine, DistillOptions, DistillTarget, DistilledSkill};

#[test]
fn test_distill_target_from_str() {
    assert_eq!(
        DistillTarget::from_str("default").unwrap(),
        DistillTarget::Default
    );
    assert_eq!(
        DistillTarget::from_str("HERMES").unwrap(),
        DistillTarget::Hermes
    );
    assert_eq!(
        DistillTarget::from_str("hermes").unwrap(),
        DistillTarget::Hermes
    );
    assert!(DistillTarget::from_str("invalid_target").is_err());
}

#[test]
fn test_distilled_skill_to_hermes_markdown() {
    let skill = DistilledSkill {
        title: "Fix SQLite Vec Linking Error".to_string(),
        trigger_context: "Building on macOS Apple Silicon".to_string(),
        root_cause: "Missing C compiler flags for sqlite-vec dynamic extension".to_string(),
        remediation: "export SQLITE_VEC_STATIC=1\ncargo build".to_string(),
        prevention_rules: vec![
            "Always verify static linking on aarch64".to_string(),
            "Check DYLD_LIBRARY_PATH".to_string(),
        ],
        tags: vec!["#type/skill".to_string(), "#topic/rust".to_string()],
        related_notes: vec!["[[SQLite Architecture]]".to_string()],
    };

    let md = skill.to_hermes_markdown();
    assert!(md.starts_with("---\n"));
    assert!(md.contains("name: fix-sqlite-vec-linking-error\n"));
    assert!(md.contains("description: Fix SQLite Vec Linking Error\n"));
    assert!(md.contains("trigger: Building on macOS Apple Silicon\n"));
    assert!(md.contains("parameters:\n  type: object\n  properties: {}\n"));
    assert!(md.contains("tags:\n"));
    assert!(md.contains("## 🎯 触发上下文与应用场景"));
    assert!(md.contains("## 🛠️ 修复策略与执行命令"));
}

#[test]
fn test_distill_engine_with_hermes_target() {
    let temp_vault = tempdir().expect("tempdir");
    let vault = temp_vault.path();
    let temp_refs = tempdir().expect("tempdir");
    let refs = temp_refs.path();

    let engine = DistillEngine::new(vault, refs);

    let raw_trace = r#"
error[E0382]: use of moved value: `buffer`
  --> src/main.rs:42:15
   |
41 |     let data = buffer;
   |                ------ value moved here
42 |     process(buffer);
   |             ^^^^^^ value used here after move

fix: clone buffer or pass by reference
    "#;

    let opts = DistillOptions {
        title: Some("Resolve Rust Move Error".to_string()),
        context_hint: Some("Testing compiler errors in CI".to_string()),
        category: Some("skill".to_string()),
        tags: vec!["rust".to_string(), "borrow".to_string()],
        related_notes: vec![],
        dry_run: false,
        target: DistillTarget::Hermes,
    };

    let result = engine
        .distill_text(raw_trace, opts)
        .expect("distill should succeed");

    assert!(result
        .preview_markdown
        .contains("name: resolve-rust-move-error"));
    assert!(result.preview_markdown.contains("parameters:"));

    let file_content =
        std::fs::read_to_string(&result.flush_result.file_path).expect("read created skill file");
    assert!(file_content.contains("name: resolve-rust-move-error"));
    assert!(file_content.contains("trigger: Testing compiler errors in CI"));
    assert!(file_content.contains("parameters:\n  type: object"));
}

#[test]
fn test_cli_distill_hermes_dry_run() {
    use assert_cmd::Command;

    let temp_vault = tempdir().expect("tempdir");
    let vault = temp_vault.path();

    let trace = r#"
error[E0433]: failed to resolve: use of undeclared crate or module `tokio`
  --> src/main.rs:2:5
fix: cargo add tokio --features full
    "#;

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.arg("distill")
        .arg("--vault")
        .arg(vault)
        .arg("--target")
        .arg("hermes")
        .arg("--title")
        .arg("Resolve Undeclared Tokio Crate")
        .arg("--dry-run")
        .write_stdin(trace)
        .assert()
        .success()
        .stdout(predicates::str::contains("Target: hermes"))
        .stdout(predicates::str::contains(
            "name: resolve-undeclared-tokio-crate",
        ))
        .stdout(predicates::str::contains(
            "parameters:\n  type: object\n  properties: {}",
        ));
}

#[test]
fn test_cli_distill_hermes_export_with_home_sandbox() {
    use assert_cmd::Command;

    let temp_vault = tempdir().expect("tempdir");
    let vault = temp_vault.path();
    let temp_home = tempdir().expect("tempdir");
    let home = temp_home.path();

    let trace = r#"
error: linking with `cc` failed: exit status 1
fix: export LDFLAGS="-L/opt/homebrew/lib"
    "#;

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.arg("distill")
        .arg("--vault")
        .arg(vault)
        .arg("--home")
        .arg(home)
        .arg("--export-hermes")
        .arg("--title")
        .arg("Fix Homebrew Linking Error")
        .write_stdin(trace)
        .assert()
        .success()
        .stdout(predicates::str::contains("Exported Hermes dynamic skill"));

    // Verify exported file in sandbox home
    let hermes_skills = home.join("skills");
    assert!(hermes_skills.exists());
    let exported_file = hermes_skills.join("fix-homebrew-linking-error.md");
    assert!(exported_file.exists());

    let content = std::fs::read_to_string(&exported_file).expect("read exported skill file");
    assert!(content.contains("name: fix-homebrew-linking-error"));
    assert!(content.contains("parameters:\n  type: object"));
}

#[test]
fn test_mcp_distill_hermes_target() {
    use k0maru::mcp::McpServer;
    use serde_json::json;

    let temp = tempdir().unwrap();
    let mut server = McpServer::new(temp.path());

    let trace = r#"
error: failed to run custom build command for `sqlite3-sys`
fix: brew install sqlite3
    "#;

    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "distill_session_skill",
            "arguments": {
                "raw_trace": trace,
                "title": "Fix SQLite3 Build Command",
                "target": "hermes",
                "dry_run": true
            }
        }
    });

    let resp_str = server
        .handle_line(&req.to_string())
        .expect("McpServer should handle line");
    assert!(resp_str.contains("name: fix-sqlite3-build-command"));
    assert!(resp_str.contains("parameters:"));
}
