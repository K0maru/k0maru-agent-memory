use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_version_smoke() {
    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("k0maru 0.1.0"));
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
