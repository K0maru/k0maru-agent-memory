use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn test_cli_search_help_smoke() {
    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Search knowledge hub"))
        .stdout(predicate::str::contains("--mode"))
        .stdout(predicate::str::contains("--limit"))
        .stdout(predicate::str::contains("--json"));
}

#[test]
fn test_cli_search_nonexistent_vault() {
    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.args([
        "search",
        "query",
        "--vault",
        "/path/to/definitely/nonexistent/vault/404",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("Vault path does not exist"));
}

#[test]
fn test_cli_search_empty_query() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let test_card = temp_dir.path().join("Note.md");
    std::fs::write(&test_card, "# Note\nSample content\n").unwrap();

    // Human-readable mode with empty query
    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.args(["search", "", "--vault", temp_dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("No results found"));

    // JSON mode with empty query
    let mut cmd_json = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    let output = cmd_json
        .args([
            "search",
            "",
            "--vault",
            temp_dir.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_val: Value = serde_json::from_slice(&output).expect("valid json array");
    assert!(json_val.is_array());
    assert_eq!(json_val.as_array().unwrap().len(), 0);
}

#[test]
fn test_cli_search_syntax_heavy_query() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let test_card = temp_dir.path().join("Note.md");
    std::fs::write(&test_card, "# Note\nSample content\n").unwrap();

    // Sync vault
    let mut sync_cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    sync_cmd
        .args(["sync", "--vault", temp_dir.path().to_str().unwrap()])
        .assert()
        .success();

    // Query with tricky SQLite FTS5 operators and punctuation
    let tricky_query = "foo AND (bar OR NOT baz) \"\" ''' [[[ {{{ *??!";
    let mut search_cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    search_cmd
        .args([
            "search",
            tricky_query,
            "--vault",
            temp_dir.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();
}

#[test]
fn test_cli_search_mode_bm25() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let note1 = temp_dir.path().join("Rust_Ownership.md");
    std::fs::write(
        &note1,
        "---\ntitle: Rust Ownership\n---\n# Rust Ownership\nInvariants: borrow checker and memory safety without GC.\n",
    )
    .unwrap();

    let note2 = temp_dir.path().join("Python_GIL.md");
    std::fs::write(
        &note2,
        "---\ntitle: Python GIL\n---\n# Python GIL\nGlobal interpreter lock details.\n",
    )
    .unwrap();

    // Sync vault first
    let mut sync_cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    sync_cmd
        .args(["sync", "--vault", temp_dir.path().to_str().unwrap()])
        .assert()
        .success();

    // BM25 search
    let mut search_cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    search_cmd
        .args([
            "search",
            "Ownership",
            "--vault",
            temp_dir.path().to_str().unwrap(),
            "--mode",
            "bm25",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rust Ownership"))
        .stdout(predicate::str::contains("Rust_Ownership.md"))
        .stdout(predicate::str::contains("#1 "));
}

#[test]
fn test_cli_search_mode_hybrid_and_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let note1 = temp_dir.path().join("Arch_Decisions.md");
    std::fs::write(
        &note1,
        "---\ntitle: Arch Decisions\n---\n# Arch Decisions\nCore invariants: Truth in files, performance in cache.\n",
    )
    .unwrap();

    // Sync vault with vector embeddings
    let mut sync_cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    sync_cmd
        .args([
            "sync",
            "--vault",
            temp_dir.path().to_str().unwrap(),
            "--vector",
        ])
        .assert()
        .success();

    // Hybrid search human-readable
    let mut search_cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    search_cmd
        .args([
            "search",
            "invariants",
            "--vault",
            temp_dir.path().to_str().unwrap(),
            "--mode",
            "hybrid",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Arch Decisions"))
        .stdout(predicate::str::contains("Truth in files"));

    // Hybrid search JSON output
    let mut search_json_cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    let assert = search_json_cmd
        .args([
            "search",
            "invariants",
            "--vault",
            temp_dir.path().to_str().unwrap(),
            "--mode",
            "hybrid",
            "--json",
        ])
        .assert()
        .success();

    let stdout_bytes = assert.get_output().stdout.clone();
    let json_val: Value = serde_json::from_slice(&stdout_bytes).expect("valid JSON output");
    assert!(json_val.is_array());
    let arr = json_val.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["title"], "Arch Decisions");
    assert!(arr[0]["score"].as_f64().unwrap() > 0.0);
    assert!(arr[0]["snippet"]
        .as_str()
        .unwrap()
        .contains("Truth in files"));
}
