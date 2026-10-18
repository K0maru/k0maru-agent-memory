use std::fs;

use assert_cmd::Command;
use k0maru::ecosystem::McpClient;
use k0maru::install::{run_install, InstallOptions, InstallTarget};
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_cli_smoke_both_binaries() {
    // 1. Verify 'k0maru' binary
    let mut cmd1 = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd1.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "k0maru {}",
            k0maru::VERSION
        )));

    let mut cmd1_help = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd1_help
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("k0maru"));

    // 2. Verify 'k0maru-memory' binary alias
    let mut cmd2 = Command::cargo_bin("k0maru-memory").expect("binary k0maru-memory should exist");
    cmd2.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(k0maru::VERSION));

    let mut cmd2_help =
        Command::cargo_bin("k0maru-memory").expect("binary k0maru-memory should exist");
    cmd2_help
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("k0maru-memory"));
}

#[test]
fn test_cli_sync_smoke() {
    let temp_dir = tempdir().expect("tempdir");
    let test_card = temp_dir.path().join("TestCard.md");
    fs::write(&test_card, "---\ntitle: Test\n---\n# Test\nBody content\n").unwrap();

    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.args([
        "sync",
        "--vault",
        temp_dir.path().to_str().unwrap(),
        "--json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"added\": 1"));
}

#[test]
fn test_doctor_diagnostic_report() {
    let temp_dir = tempdir().expect("tempdir");
    let vault_path = temp_dir.path().to_str().unwrap();

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.args(["doctor", "--vault", vault_path, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Executable Path"))
        .stdout(predicate::str::contains("k0maru Version"));
}

#[test]
fn test_install_claude_and_cursor_configs() {
    let temp_home = tempdir().unwrap();
    let home = temp_home.path();
    let vault = temp_home.path().join("Vault");
    fs::create_dir_all(&vault).unwrap();

    // Install to Claude
    let opts = InstallOptions {
        target: InstallTarget::Claude,
        vault_path: vault.clone(),
        home_override: Some(home.to_path_buf()),
        dry_run: false,
    };
    let report = run_install(opts).unwrap();
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(report.outcomes[0].client, McpClient::Claude);
    assert!(report.outcomes[0].config_path.exists());

    let claude_config = home.join(".claude.json");
    assert!(claude_config.exists());
    let content = fs::read_to_string(&claude_config).unwrap();
    assert!(content.contains("k0maru-memory"));
}
#[test]
fn test_install_omp_injects_sidecar_flag() {
    let temp_home = tempdir().unwrap();
    let home = temp_home.path();
    let vault = temp_home.path().join("Vault");
    fs::create_dir_all(&vault).unwrap();

    let opts = InstallOptions {
        target: InstallTarget::Omp,
        vault_path: vault.clone(),
        home_override: Some(home.to_path_buf()),
        dry_run: false,
    };
    let report = run_install(opts.clone()).unwrap();
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(report.outcomes[0].client, McpClient::Omp);
    assert!(report.outcomes[0].config_path.exists());

    let omp_config = home.join(".omp").join("agent").join("mcp.json");
    assert!(omp_config.exists());
    let content = fs::read_to_string(&omp_config).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    let args = parsed
        .get("mcpServers")
        .and_then(|s| s.get("k0maru-memory"))
        .and_then(|k| k.get("args"))
        .and_then(|a| a.as_array())
        .expect("args array should exist in mcpServers['k0maru-memory']");
    assert!(
        args.iter().any(|arg| arg.as_str() == Some("--sidecar")),
        "args should contain '--sidecar'"
    );

    // Verify already_configured is detected on subsequent install
    let report2 = run_install(opts).unwrap();
    assert_eq!(report2.outcomes.len(), 1);
    assert!(report2.outcomes[0].already_configured);
}
