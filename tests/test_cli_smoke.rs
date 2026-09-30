use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_version_smoke() {
    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "k0maru {}",
            k0maru::VERSION
        )));
}

#[test]
fn test_cli_help_smoke() {
    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("k0maru"));
}

#[test]
fn test_cli_mcp_help_smoke() {
    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.args(["mcp", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("FastMCP stdio server"));
}

#[test]
fn test_cli_sync_smoke() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let test_card = temp_dir.path().join("TestCard.md");
    std::fs::write(&test_card, "---\ntitle: Test\n---\n# Test\nBody content\n").unwrap();

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
