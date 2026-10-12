use assert_cmd::Command;
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

    // Configure user name and email for commits
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

    // Installing again should be idempotent and not duplicate
    let mut cmd2 = Command::cargo_bin("k0maru").unwrap();
    cmd2.current_dir(dir.path())
        .args(["guard", "install", "--path", dir.path().to_str().unwrap()])
        .assert()
        .success();

    let content2 = fs::read_to_string(&hook_path).unwrap();
    assert_eq!(content, content2);
}

#[test]
fn test_guard_check_cli_with_forbidden_violation_exits_code_1() {
    let dir = tempdir().unwrap();
    init_git_repo(dir.path());

    // Create .k0maru/guard.toml
    let k0maru_dir = dir.path().join(".k0maru");
    fs::create_dir_all(&k0maru_dir).unwrap();
    fs::write(
        k0maru_dir.join("guard.toml"),
        r#"
[[rules]]
id = "SEC-001"
title = "No hardcoded keys"
file_pattern = "*.rs"
severity = "deny"
forbidden = "DANGEROUS_KEY"
remediation = "Use env variables"
"#,
    )
    .unwrap();

    // Create a commit
    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let _ = StdCommand::new("git")
        .args(["add", "main.rs"])
        .current_dir(dir.path())
        .output();
    let _ = StdCommand::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output();

    // Modify file with forbidden key
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    let k = DANGEROUS_KEY;\n}\n",
    )
    .unwrap();

    // Run k0maru guard check --diff
    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.current_dir(dir.path())
        .args([
            "guard",
            "check",
            "--diff",
            "--vault",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .code(1);

    // Run with --json
    let mut cmd_json = Command::cargo_bin("k0maru").unwrap();
    let assert_out = cmd_json
        .current_dir(dir.path())
        .args([
            "guard",
            "check",
            "--diff",
            "--vault",
            dir.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .code(1);

    let output = assert_out.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"is_passed\": false") || stdout.contains("\"is_passed\":false"));
    assert!(stdout.contains("SEC-001"));
}

#[test]
fn test_guard_check_cli_clean_diff_exits_code_0() {
    let dir = tempdir().unwrap();
    init_git_repo(dir.path());

    // Create .k0maru/guard.toml
    let k0maru_dir = dir.path().join(".k0maru");
    fs::create_dir_all(&k0maru_dir).unwrap();
    fs::write(
        k0maru_dir.join("guard.toml"),
        r#"
[[rules]]
id = "SEC-001"
title = "No hardcoded keys"
file_pattern = "*.rs"
severity = "deny"
forbidden = "DANGEROUS_KEY"
remediation = "Use env variables"
"#,
    )
    .unwrap();

    fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    let _ = StdCommand::new("git")
        .args(["add", "main.rs"])
        .current_dir(dir.path())
        .output();
    let _ = StdCommand::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output();

    // Modify file cleanly
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    println!(\"Clean code\");\n}\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.current_dir(dir.path())
        .args([
            "guard",
            "check",
            "--diff",
            "--vault",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .code(0);
}
