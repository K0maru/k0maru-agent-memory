use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

use k0maru::doctor::{
    format_report, probe_binary, probe_ecosystem, probe_storage, probe_vault, run_diagnostics,
    DoctorReport, StatusLevel,
};
use k0maru::ecosystem::{inspect_all_clients, inspect_client, McpClient};
use k0maru::storage::SqliteStorage;

#[test]
fn test_mcp_client_metadata() {
    assert_eq!(McpClient::Claude.name(), "Claude Code");
    assert_eq!(McpClient::Cursor.name(), "Cursor");
    assert_eq!(McpClient::Gemini.name(), "Antigravity / Gemini CLI");
    assert_eq!(McpClient::Windsurf.name(), "Windsurf");
    assert_eq!(McpClient::Cline.name(), "Cline / Roo Code");

    assert_eq!(McpClient::all().len(), 5);
}

#[test]
fn test_mcp_client_config_path_resolution() {
    let temp_home = tempdir().expect("tempdir");
    let home = temp_home.path();

    assert_eq!(
        McpClient::Claude.config_path(Some(home)),
        Some(home.join(".claude.json"))
    );
    assert_eq!(
        McpClient::Cursor.config_path(Some(home)),
        Some(home.join(".cursor").join("mcp.json"))
    );
    assert_eq!(
        McpClient::Windsurf.config_path(Some(home)),
        Some(
            home.join(".codeium")
                .join("windsurf")
                .join("mcp_config.json")
        )
    );

    // Default Gemini should be .gemini/config/mcp_config.json
    assert_eq!(
        McpClient::Gemini.config_path(Some(home)),
        Some(home.join(".gemini").join("config").join("mcp_config.json"))
    );

    // If antigravity-cli exists instead of config:
    let antigravity_dir = home.join(".gemini").join("antigravity-cli");
    fs::create_dir_all(&antigravity_dir).unwrap();
    let antigravity_file = antigravity_dir.join("mcp_config.json");
    fs::write(&antigravity_file, "{}").unwrap();

    assert_eq!(
        McpClient::Gemini.config_path(Some(home)),
        Some(antigravity_file)
    );
}

#[test]
fn test_inspect_client_uninstalled() {
    let temp_home = tempdir().expect("tempdir");
    let home = temp_home.path();

    let status = inspect_client(McpClient::Claude, Some(home));
    assert_eq!(status.client, McpClient::Claude);
    assert!(!status.installed);
    assert!(!status.configured);
    assert_eq!(status.vault_path, None);
}

#[test]
fn test_inspect_client_installed_unconfigured() {
    let temp_home = tempdir().expect("tempdir");
    let home = temp_home.path();

    let claude_cfg = home.join(".claude.json");
    fs::write(&claude_cfg, r#"{"mcpServers": {}}"#).unwrap();

    let status = inspect_client(McpClient::Claude, Some(home));
    assert!(status.installed);
    assert!(!status.configured);
    assert_eq!(status.vault_path, None);
}

#[test]
fn test_inspect_client_installed_configured() {
    let temp_home = tempdir().expect("tempdir");
    let home = temp_home.path();

    let claude_cfg = home.join(".claude.json");
    let config_content = r#"{
        "mcpServers": {
            "k0maru-memory": {
                "command": "k0maru",
                "args": ["mcp", "--vault", "/path/to/my-vault"]
            }
        }
    }"#;
    fs::write(&claude_cfg, config_content).unwrap();

    let status = inspect_client(McpClient::Claude, Some(home));
    assert!(status.installed);
    assert!(status.configured);
    assert_eq!(status.vault_path, Some(PathBuf::from("/path/to/my-vault")));
}

#[test]
fn test_inspect_client_configured_inline_arg() {
    let temp_home = tempdir().expect("tempdir");
    let home = temp_home.path();

    let cursor_dir = home.join(".cursor");
    fs::create_dir_all(&cursor_dir).unwrap();
    let cursor_cfg = cursor_dir.join("mcp.json");
    let config_content = r#"{
        "mcpServers": {
            "k0maru": {
                "command": "k0maru",
                "args": ["mcp", "--vault=/custom/vault"]
            }
        }
    }"#;
    fs::write(&cursor_cfg, config_content).unwrap();

    let status = inspect_client(McpClient::Cursor, Some(home));
    assert!(status.installed);
    assert!(status.configured);
    assert_eq!(status.vault_path, Some(PathBuf::from("/custom/vault")));
}

#[test]
fn test_inspect_all_clients() {
    let temp_home = tempdir().expect("tempdir");
    let home = temp_home.path();

    let statuses = inspect_all_clients(Some(home));
    assert_eq!(statuses.len(), 5);
}

#[test]
fn test_probe_binary() {
    let items = probe_binary();
    assert!(!items.is_empty());
    assert!(items.iter().any(|i| i.name == "Executable Path"));
    assert!(items.iter().any(|i| i.name == "k0maru Version"));
    assert!(items.iter().any(|i| i.name == "Operating System"));
}

#[test]
fn test_probe_vault_nonexistent() {
    let temp = tempdir().expect("tempdir");
    let fake_path = temp.path().join("does_not_exist");

    let items = probe_vault(&fake_path);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].status, StatusLevel::Fail);
    assert!(items[0].detail.contains("Path does not exist"));
}

#[test]
fn test_probe_vault_empty() {
    let temp = tempdir().expect("tempdir");

    let items = probe_vault(temp.path());
    assert!(items
        .iter()
        .any(|i| i.name == "Vault Path" && i.status == StatusLevel::Pass));
    assert!(items
        .iter()
        .any(|i| i.name == "Markdown Documents" && i.status == StatusLevel::Warn));
    assert!(items
        .iter()
        .any(|i| i.name == "Knowledge Hierarchy" && i.status == StatusLevel::Warn));
}

#[test]
fn test_probe_vault_populated() {
    let temp = tempdir().expect("tempdir");
    let vault = temp.path();

    fs::create_dir_all(vault.join("10_Projects")).unwrap();
    fs::create_dir_all(vault.join("20_Cards")).unwrap();
    fs::write(vault.join("10_Projects").join("ProjectA.md"), "# Project A").unwrap();
    fs::write(vault.join("20_Cards").join("Card1.md"), "# Card 1").unwrap();

    let items = probe_vault(vault);
    let doc_item = items
        .iter()
        .find(|i| i.name == "Markdown Documents")
        .unwrap();
    assert_eq!(doc_item.status, StatusLevel::Pass);
    assert!(doc_item.detail.contains("2 markdown notes"));

    let hier_item = items
        .iter()
        .find(|i| i.name == "Knowledge Hierarchy")
        .unwrap();
    assert_eq!(hier_item.status, StatusLevel::Pass);
    assert!(hier_item.detail.contains("10_Projects"));
}

#[test]
fn test_probe_storage_uninitialized() {
    let temp = tempdir().expect("tempdir");
    let items = probe_storage(temp.path());

    let cache_item = items.iter().find(|i| i.name == "SQLite Cache").unwrap();
    assert_eq!(cache_item.status, StatusLevel::Warn);
    assert!(cache_item.detail.contains("Cache file does not exist"));
}

#[test]
fn test_probe_storage_initialized() {
    let temp = tempdir().expect("tempdir");
    let vault = temp.path();
    let cache_file = vault.join(".k0maru").join("cache.sqlite");

    // Initialize sqlite storage
    let _storage = SqliteStorage::open(&cache_file).expect("create storage");

    let items = probe_storage(vault);
    let cache_item = items.iter().find(|i| i.name == "SQLite Cache").unwrap();
    assert_eq!(cache_item.status, StatusLevel::Pass);

    let fts_item = items
        .iter()
        .find(|i| i.name == "FTS5 Search Index")
        .unwrap();
    assert_eq!(fts_item.status, StatusLevel::Pass);

    let vec_item = items
        .iter()
        .find(|i| i.name == "Vector Embeddings")
        .unwrap();
    assert_eq!(vec_item.status, StatusLevel::Pass);
}

#[test]
fn test_probe_ecosystem_matching_and_mismatched() {
    let temp = tempdir().expect("tempdir");
    let home = temp.path().join("home");
    let vault1 = temp.path().join("vault1");
    let vault2 = temp.path().join("vault2");

    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&vault1).unwrap();
    fs::create_dir_all(&vault2).unwrap();

    // Claude configured to vault1
    let claude_cfg = home.join(".claude.json");
    fs::write(
        &claude_cfg,
        format!(
            r#"{{"mcpServers": {{"k0maru-memory": {{"command": "k0maru", "args": ["mcp", "--vault", "{}"]}}}}}}"#,
            vault1.display()
        ),
    )
    .unwrap();

    // Cursor configured to vault2
    let cursor_dir = home.join(".cursor");
    fs::create_dir_all(&cursor_dir).unwrap();
    fs::write(
        cursor_dir.join("mcp.json"),
        format!(
            r#"{{"mcpServers": {{"k0maru": {{"command": "k0maru", "args": ["mcp", "--vault", "{}"]}}}}}}"#,
            vault2.display()
        ),
    )
    .unwrap();

    let items = probe_ecosystem(Some(&home), &vault1);

    let claude_item = items.iter().find(|i| i.name == "Claude Code").unwrap();
    assert_eq!(claude_item.status, StatusLevel::Pass);
    assert!(claude_item
        .detail
        .contains("Mounted and targeting current vault"));

    let cursor_item = items.iter().find(|i| i.name == "Cursor").unwrap();
    assert_eq!(cursor_item.status, StatusLevel::Warn);
    assert!(cursor_item.detail.contains("Mounted to a different vault"));

    let windsurf_item = items.iter().find(|i| i.name == "Windsurf").unwrap();
    assert_eq!(windsurf_item.status, StatusLevel::Pass);
    assert!(windsurf_item.detail.contains("Not installed"));
}

#[test]
fn test_run_diagnostics_and_formatting() {
    let temp = tempdir().expect("tempdir");
    let vault = temp.path();

    fs::create_dir_all(vault.join("10_Projects")).unwrap();
    fs::write(vault.join("10_Projects").join("Note.md"), "# Hello").unwrap();
    let cache_file = vault.join(".k0maru").join("cache.sqlite");
    let _storage = SqliteStorage::open(&cache_file).unwrap();

    let report = run_diagnostics(vault, Some(temp.path()));
    assert_eq!(
        report.summary.total,
        report.summary.passed + report.summary.warnings + report.summary.failures
    );
    assert_eq!(report.summary.failures, 0);

    let formatted = format_report(&report);
    assert!(formatted.contains("k0maru doctor"));
    assert!(formatted.contains("[Binary & System]"));
    assert!(formatted.contains("[Vault & Hierarchy]"));
    assert!(formatted.contains("[Storage & Search Engine]"));
    assert!(formatted.contains("[Agent MCP Ecosystem]"));
    assert!(formatted.contains("Summary:"));
}

#[test]
fn test_cli_doctor_help_smoke() {
    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Diagnose system health"));
}

#[test]
fn test_cli_doctor_json_output() {
    let temp = tempdir().expect("tempdir");
    let vault = temp.path();
    fs::create_dir_all(vault.join("10_Projects")).unwrap();
    fs::write(vault.join("10_Projects").join("Note.md"), "# Hello").unwrap();

    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    let output = cmd
        .args(["doctor", "--vault", vault.to_str().unwrap(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let report: DoctorReport = serde_json::from_slice(&output).expect("valid DoctorReport JSON");
    assert_eq!(
        report.summary.total,
        report.summary.passed + report.summary.warnings + report.summary.failures
    );
    assert!(report
        .items
        .iter()
        .any(|i| i.category == "Vault & Hierarchy"));
}

#[test]
fn test_cli_doctor_human_output() {
    let temp = tempdir().expect("tempdir");
    let vault = temp.path();

    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.args(["doctor", "--vault", vault.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "k0maru doctor — System & Environment Health Report",
        ))
        .stdout(predicate::str::contains("Summary:"));
}

#[test]
fn test_cli_doctor_failure_exit_code() {
    let temp = tempdir().expect("tempdir");
    let non_existent = temp.path().join("does-not-exist");

    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.args(["doctor", "--vault", non_existent.to_str().unwrap()])
        .assert()
        .failure();
}
