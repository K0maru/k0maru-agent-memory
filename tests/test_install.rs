use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

use k0maru::ecosystem::McpClient;
use k0maru::install::{
    format_install_report, inject_k0maru_mcp, run_install, InstallOptions, InstallReport,
    InstallTarget,
};

#[test]
fn test_install_target_from_str() {
    assert_eq!(InstallTarget::from_str("all").unwrap(), InstallTarget::All);
    assert_eq!(InstallTarget::from_str("ALL").unwrap(), InstallTarget::All);
    assert_eq!(
        InstallTarget::from_str("claude").unwrap(),
        InstallTarget::Claude
    );
    assert_eq!(
        InstallTarget::from_str("claude-code").unwrap(),
        InstallTarget::Claude
    );
    assert_eq!(
        InstallTarget::from_str("cursor").unwrap(),
        InstallTarget::Cursor
    );
    assert_eq!(
        InstallTarget::from_str("gemini").unwrap(),
        InstallTarget::Gemini
    );
    assert_eq!(
        InstallTarget::from_str("antigravity").unwrap(),
        InstallTarget::Gemini
    );
    assert_eq!(
        InstallTarget::from_str("windsurf").unwrap(),
        InstallTarget::Windsurf
    );
    assert_eq!(
        InstallTarget::from_str("cline").unwrap(),
        InstallTarget::Cline
    );
    assert_eq!(
        InstallTarget::from_str("roo").unwrap(),
        InstallTarget::Cline
    );
    assert_eq!(
        InstallTarget::from_str("roo-cline").unwrap(),
        InstallTarget::Cline
    );

    assert!(InstallTarget::from_str("invalid-client").is_err());
}

#[test]
fn test_install_target_clients() {
    assert_eq!(InstallTarget::All.clients().len(), 5);
    assert_eq!(InstallTarget::Claude.clients(), vec![McpClient::Claude]);
    assert_eq!(InstallTarget::Cursor.clients(), vec![McpClient::Cursor]);
    assert_eq!(InstallTarget::Gemini.clients(), vec![McpClient::Gemini]);
    assert_eq!(InstallTarget::Windsurf.clients(), vec![McpClient::Windsurf]);
    assert_eq!(InstallTarget::Cline.clients(), vec![McpClient::Cline]);
}

#[test]
fn test_inject_k0maru_mcp_empty_input() {
    let vault = Path::new("/Users/dev/my-vault");
    let json_str = inject_k0maru_mcp(None, vault).expect("inject into empty");

    let val: serde_json::Value = serde_json::from_str(&json_str).expect("valid json");
    let server = &val["mcpServers"]["k0maru-memory"];

    assert_eq!(server["command"], "k0maru");
    assert_eq!(
        server["args"],
        serde_json::json!(["mcp", "--vault", "/Users/dev/my-vault"])
    );
}

#[test]
fn test_inject_k0maru_mcp_preserves_existing_keys_and_servers() {
    let initial = r#"{
        "theme": "dark",
        "mcpServers": {
            "fetch": {
                "command": "uvx",
                "args": ["mcp-server-fetch"]
            },
            "postgres": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-postgres"]
            }
        },
        "telemetry": false
    }"#;

    let vault = Path::new("/Users/dev/my-vault");
    let result = inject_k0maru_mcp(Some(initial), vault).expect("inject should succeed");

    let val: serde_json::Value = serde_json::from_str(&result).expect("valid json");

    // Existing top-level keys preserved
    assert_eq!(val["theme"], "dark");
    assert_eq!(val["telemetry"], false);

    // Existing other servers preserved
    assert_eq!(val["mcpServers"]["fetch"]["command"], "uvx");
    assert_eq!(val["mcpServers"]["postgres"]["command"], "npx");

    // Injected server added
    assert_eq!(val["mcpServers"]["k0maru-memory"]["command"], "k0maru");
    assert_eq!(
        val["mcpServers"]["k0maru-memory"]["args"],
        serde_json::json!(["mcp", "--vault", "/Users/dev/my-vault"])
    );
}

#[test]
fn test_inject_k0maru_mcp_updates_existing_k0maru_memory() {
    let initial = r#"{
        "mcpServers": {
            "k0maru-memory": {
                "command": "k0maru",
                "args": ["mcp", "--vault", "/old/vault/path"]
            }
        }
    }"#;

    let new_vault = Path::new("/new/vault/path");
    let result = inject_k0maru_mcp(Some(initial), new_vault).expect("inject should succeed");

    let val: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(
        val["mcpServers"]["k0maru-memory"]["args"],
        serde_json::json!(["mcp", "--vault", "/new/vault/path"])
    );
}

#[test]
fn test_inject_k0maru_mcp_invalid_json() {
    let invalid = "not a json string";
    assert!(inject_k0maru_mcp(Some(invalid), Path::new("/vault")).is_err());

    let array_root = "[1, 2, 3]";
    assert!(inject_k0maru_mcp(Some(array_root), Path::new("/vault")).is_err());

    let invalid_servers = r#"{"mcpServers": "not-an-object"}"#;
    assert!(inject_k0maru_mcp(Some(invalid_servers), Path::new("/vault")).is_err());
}

#[test]
fn test_run_install_dry_run() {
    let temp_home = tempdir().expect("tempdir");
    let home = temp_home.path();
    let temp_vault = tempdir().expect("tempdir");
    let vault = temp_vault.path();

    let options = InstallOptions {
        vault_path: vault.to_path_buf(),
        target: InstallTarget::All,
        dry_run: true,
        home_override: Some(home.to_path_buf()),
    };

    let report = run_install(options).expect("dry run should succeed");
    assert!(report.dry_run);
    assert_eq!(report.outcomes.len(), 5);

    for outcome in &report.outcomes {
        assert!(outcome.dry_run);
        assert!(!outcome.updated);
        assert!(outcome.preview_json.is_some());
        // Verify disk untouched
        assert!(!outcome.config_path.exists());
    }
}

#[test]
fn test_run_install_live_all_clients() {
    let temp_home = tempdir().expect("tempdir");
    let home = temp_home.path();
    let temp_vault = tempdir().expect("tempdir");
    let vault = temp_vault.path();

    let options = InstallOptions {
        vault_path: vault.to_path_buf(),
        target: InstallTarget::All,
        dry_run: false,
        home_override: Some(home.to_path_buf()),
    };

    let report = run_install(options).expect("install should succeed");
    assert!(!report.dry_run);
    assert_eq!(report.outcomes.len(), 5);

    for outcome in &report.outcomes {
        assert!(outcome.updated);
        assert!(outcome.created_file);
        assert!(!outcome.already_configured);
        assert!(outcome.config_path.exists());

        let content = fs::read_to_string(&outcome.config_path).expect("read file");
        let val: serde_json::Value = serde_json::from_str(&content).expect("valid json");
        assert_eq!(val["mcpServers"]["k0maru-memory"]["command"], "k0maru");
    }
}

#[test]
fn test_run_install_selective_client() {
    let temp_home = tempdir().expect("tempdir");
    let home = temp_home.path();
    let temp_vault = tempdir().expect("tempdir");
    let vault = temp_vault.path();

    let options = InstallOptions {
        vault_path: vault.to_path_buf(),
        target: InstallTarget::Cursor,
        dry_run: false,
        home_override: Some(home.to_path_buf()),
    };

    let report = run_install(options).expect("install cursor should succeed");
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(report.outcomes[0].client, McpClient::Cursor);
    assert!(report.outcomes[0].config_path.exists());

    // Claude should not have been created
    let claude_cfg = home.join(".claude.json");
    assert!(!claude_cfg.exists());
}

#[test]
fn test_run_install_already_configured() {
    let temp_home = tempdir().expect("tempdir");
    let home = temp_home.path();
    let temp_vault = tempdir().expect("tempdir");
    let vault = temp_vault.path();

    let options1 = InstallOptions {
        vault_path: vault.to_path_buf(),
        target: InstallTarget::Claude,
        dry_run: false,
        home_override: Some(home.to_path_buf()),
    };

    let report1 = run_install(options1).expect("first install");
    assert!(report1.outcomes[0].created_file);
    assert!(!report1.outcomes[0].already_configured);

    // Second run with same vault
    let options2 = InstallOptions {
        vault_path: vault.to_path_buf(),
        target: InstallTarget::Claude,
        dry_run: false,
        home_override: Some(home.to_path_buf()),
    };

    let report2 = run_install(options2).expect("second install");
    assert!(!report2.outcomes[0].created_file);
    assert!(report2.outcomes[0].already_configured);
    assert!(report2.outcomes[0].updated);
}

#[test]
fn test_format_install_report() {
    let temp_vault = tempdir().expect("tempdir");
    let vault = temp_vault.path();

    let report = InstallReport {
        vault_path: vault.to_path_buf(),
        dry_run: true,
        outcomes: vec![k0maru::install::ClientInstallOutcome {
            client: McpClient::Claude,
            config_path: PathBuf::from("/home/user/.claude.json"),
            created_file: true,
            already_configured: false,
            updated: false,
            dry_run: true,
            preview_json: Some(r#"{"mcpServers": {"k0maru-memory": {}}}"#.to_string()),
        }],
    };

    let formatted = format_install_report(&report);
    assert!(formatted.contains("K0maru MCP Client Configurator"));
    assert!(formatted.contains("DRY RUN"));
    assert!(formatted.contains("Claude Code"));
    assert!(formatted.contains("/home/user/.claude.json"));
}

#[test]
fn test_cli_install_help_smoke() {
    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.args(["install", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Automatically configure k0maru-memory MCP server",
        ))
        .stdout(predicate::str::contains("--vault"))
        .stdout(predicate::str::contains("--target"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--json"));
}

#[test]
fn test_cli_install_dry_run_human() {
    let temp_vault = tempdir().expect("tempdir");
    let vault = temp_vault.path();

    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.args([
        "install",
        "--vault",
        vault.to_str().unwrap(),
        "--target",
        "claude",
        "--dry-run",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("K0maru MCP Client Configurator"))
    .stdout(predicate::str::contains("Claude Code"))
    .stdout(predicate::str::contains("DRY RUN"));
}

#[test]
fn test_cli_install_dry_run_json() {
    let temp_vault = tempdir().expect("tempdir");
    let vault = temp_vault.path();

    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    let output = cmd
        .args([
            "install",
            "--vault",
            vault.to_str().unwrap(),
            "--target",
            "cursor",
            "--dry-run",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let report: InstallReport = serde_json::from_slice(&output).expect("valid InstallReport JSON");
    assert!(report.dry_run);
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(report.outcomes[0].client, McpClient::Cursor);
    assert!(report.outcomes[0].preview_json.is_some());
}

#[test]
fn test_cli_install_invalid_target() {
    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.args(["install", "--target", "invalid_client_xyz"])
        .assert()
        .failure();
}
