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
