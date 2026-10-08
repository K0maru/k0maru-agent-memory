use std::fs;

use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn test_cli_distill_help_smoke() {
    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.arg("distill")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("--node"))
        .stdout(predicates::str::contains("--file"))
        .stdout(predicates::str::contains("--context"))
        .stdout(predicates::str::contains("--category"))
        .stdout(predicates::str::contains("--dry-run"))
        .stdout(predicates::str::contains("--json"));
}

#[test]
fn test_cli_distill_stdin_pipe() {
    let vault_tmp = tempdir().unwrap();
    let vault_root = vault_tmp.path();
    fs::create_dir_all(vault_root.join("skills")).unwrap();

    let trace = r#"
$ cargo build
error[E0432]: unresolved import `foo`
  --> src/main.rs:1:5
   |
 1 | use foo::bar;
   |     ^^^ maybe a missing crate `foo`?

$ cargo add foo
$ cargo build
Finished profile
"#;

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.arg("distill")
        .arg("--vault")
        .arg(vault_root)
        .arg("--title")
        .arg("Fix Missing Foo Crate")
        .arg("--category")
        .arg("skill")
        .write_stdin(trace)
        .assert()
        .success()
        .stdout(predicates::str::contains("Fix Missing Foo Crate"))
        .stdout(predicates::str::contains("skills/"));

    // Verify file exists on disk
    let skills_dir = vault_root.join("skills");
    let files: Vec<_> = fs::read_dir(&skills_dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    assert_eq!(files.len(), 1);

    let content = fs::read_to_string(&files[0]).unwrap();
    assert!(content.contains("# Fix Missing Foo Crate"));
    assert!(content.contains("## 🎯 触发上下文与应用场景"));
    assert!(content.contains("## 🛠️ 修复策略与执行命令"));
}

#[test]
fn test_cli_distill_file() {
    let vault_tmp = tempdir().unwrap();
    let vault_root = vault_tmp.path();
    fs::create_dir_all(vault_root.join("skills")).unwrap();

    let trace_file = vault_tmp.path().join("error_trace.log");
    fs::write(
        &trace_file,
        "Traceback (most recent call last):\n  File \"app.py\", line 12\nIndexError: list index out of range",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.arg("distill")
        .arg("--vault")
        .arg(vault_root)
        .arg("--file")
        .arg(&trace_file)
        .arg("--context")
        .arg("Empty list index access in app.py")
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "Empty list index access in app.py",
        ))
        .stdout(predicates::str::contains("skills/"));

    let skills_dir = vault_root.join("skills");
    let files: Vec<_> = fs::read_dir(&skills_dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    assert_eq!(files.len(), 1);
    let content = fs::read_to_string(&files[0]).unwrap();
    assert!(content.contains("IndexError: list index out of range"));
}

#[test]
fn test_cli_distill_dry_run_json() {
    let vault_tmp = tempdir().unwrap();
    let vault_root = vault_tmp.path();
    fs::create_dir_all(vault_root.join("skills")).unwrap();

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    let assert = cmd
        .arg("distill")
        .arg("--vault")
        .arg(vault_root)
        .arg("--title")
        .arg("Preview Dry Run Skill")
        .arg("--dry-run")
        .arg("--json")
        .write_stdin("error: test failure")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json_val["skill"]["title"], "Preview Dry Run Skill");
    assert_eq!(json_val["flush_result"]["dry_run"], true);

    // Verify nothing written on disk
    let skills_dir = vault_root.join("skills");
    let files: Vec<_> = fs::read_dir(&skills_dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    assert!(files.is_empty());
}
