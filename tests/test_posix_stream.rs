//! Integration tests for Milestone 4: POSIX Stream & Pipe Standardization.
//!
//! Covers:
//! - Stdin diff stream ingestion (`git diff | k0maru loadout`) under 300 tokens
//! - Stdin log stream ingestion (`cat log.txt | k0maru offload`)
//! - Non-blocking interactive TTY guard on `offload` (exit code 2)
//! - Line-delimited NDJSON streaming (`--json-stream`) across subcommands
//! - Standard POSIX exit codes conformance (0, 1, 2)
//! - Broken pipe (SIGPIPE / BrokenPipe) defensive handling without panics

use std::fs;
use std::io::{BufRead, BufReader};
use std::process::Stdio;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

use common::fixtures::{mock_karpathy_wiki, mock_obsidian_vault};
use k0maru::loadout::estimate_tokens;

mod common;

#[test]
fn test_cli_loadout_git_diff_pipe() {
    let vault = mock_obsidian_vault();
    let vault_path = vault.path().to_str().unwrap();

    let diff_input = r#"
diff --git a/src/storage/sqlite.rs b/src/storage/sqlite.rs
index 1111111..2222222 100644
--- a/src/storage/sqlite.rs
+++ b/src/storage/sqlite.rs
@@ -10,3 +10,4 @@ pub struct SqliteStorage;
+pub fn new_query() {}
"#;

    // Case 1: Piped stdin diff with explicit query
    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.args(["loadout", "k0maru-memory", "--vault", vault_path])
        .write_stdin(diff_input)
        .assert()
        .success()
        .stdout(predicate::str::contains("k0maru-memory"))
        .stdout(predicate::str::contains("[Diff-Aware]"))
        .stdout(predicate::str::contains("Architecture_Decisions"));

    let output = Command::cargo_bin("k0maru")
        .unwrap()
        .args(["loadout", "k0maru-memory", "--vault", vault_path])
        .write_stdin(diff_input)
        .output()
        .unwrap();

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let tokens = estimate_tokens(&stdout_str);
    assert!(
        tokens < 300,
        "Loadout with piped diff must be under 300 tokens, got {}",
        tokens
    );

    // Case 2: Piped stdin diff with implicit project (no positional query)
    let output2 = Command::cargo_bin("k0maru")
        .unwrap()
        .args(["loadout", "--vault", vault_path])
        .write_stdin(diff_input)
        .output()
        .unwrap();

    assert!(
        output2.status.success(),
        "Piped loadout without query must succeed"
    );
    let stdout_str2 = String::from_utf8_lossy(&output2.stdout);
    assert!(
        stdout_str2.contains("[Diff-Aware]"),
        "Stdout must activate diff-aware loadout"
    );
    let tokens2 = estimate_tokens(&stdout_str2);
    assert!(
        tokens2 < 300,
        "Piped diff loadout without query must be under 300 tokens, got {}",
        tokens2
    );
}

#[test]
fn test_cli_offload_stdin_pipe() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let refs_str = refs_dir.to_str().unwrap();

    // 60 lines of build output (exceeding default 50 threshold)
    let log_input = (1..=60)
        .map(|i| format!("Compiling crates/k0maru-core step [{}/60]", i))
        .collect::<Vec<_>>()
        .join("\n");

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.args(["offload", "--refs-dir", refs_str])
        .write_stdin(log_input)
        .assert()
        .success()
        .stdout(predicate::str::contains("stateDiagram-v2"))
        .stdout(predicate::str::contains("k0maru inspect node_"))
        .stdout(predicate::str::contains("60 lines captured"));

    // Verify disk persistence
    let log_files: Vec<_> = fs::read_dir(&refs_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        log_files.len(),
        1,
        "Exactly one offload reference must be written"
    );
    let content = fs::read_to_string(log_files[0].path()).unwrap();
    assert!(content.contains("Compiling crates/k0maru-core step [1/60]"));
    assert!(content.contains("Compiling crates/k0maru-core step [60/60]"));
}

#[test]
fn test_cli_offload_interactive_tty_guard() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let refs_str = refs_dir.to_str().unwrap();

    // When run interactively without pipe (simulated via K0MARU_FORCE_TTY=1)
    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.env("K0MARU_FORCE_TTY", "1")
        .args(["offload", "--refs-dir", refs_str])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("交互式终端 (TTY)"))
        .stderr(predicate::str::contains("cat log.txt | k0maru offload"));
}

#[test]
fn test_cli_json_stream_events() {
    let vault = mock_obsidian_vault();
    let vault_path = vault.path().to_str().unwrap();
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let refs_str = refs_dir.to_str().unwrap();

    // 1. loadout --json-stream
    let diff_input = "diff --git a/src/storage/sqlite.rs b/src/storage/sqlite.rs\n--- a/src/storage/sqlite.rs\n+++ b/src/storage/sqlite.rs\n@@ -1,1 +1,2 @@\n+pub fn test() {}\n";
    let output_loadout = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "loadout",
            "k0maru-memory",
            "--json-stream",
            "--vault",
            vault_path,
        ])
        .write_stdin(diff_input)
        .output()
        .unwrap();

    assert!(output_loadout.status.success());
    let stdout_loadout = String::from_utf8_lossy(&output_loadout.stdout);
    let mut loadout_events = Vec::new();
    for line in stdout_loadout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let val: serde_json::Value =
            serde_json::from_str(line).expect("Each line must be valid NDJSON");
        assert!(val.get("event").is_some());
        assert!(val.get("timestamp").is_some());
        assert!(val.get("data").is_some());
        loadout_events.push(val["event"].as_str().unwrap().to_string());
    }
    assert!(loadout_events.contains(&"loadout_start".to_string()));
    assert!(loadout_events.contains(&"loadout_ready".to_string()));

    // 2. loadout --list --json-stream
    let output_list = Command::cargo_bin("k0maru")
        .unwrap()
        .args(["loadout", "--list", "--json-stream", "--vault", vault_path])
        .output()
        .unwrap();

    assert!(output_list.status.success());
    let stdout_list = String::from_utf8_lossy(&output_list.stdout);
    let mut list_events = Vec::new();
    for line in stdout_list.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let val: serde_json::Value =
            serde_json::from_str(line).expect("Each line must be valid NDJSON");
        list_events.push(val["event"].as_str().unwrap().to_string());
    }
    assert!(list_events.contains(&"loadout_start".to_string()));
    assert!(list_events.contains(&"project_listed".to_string()));
    assert!(list_events.contains(&"loadout_ready".to_string()));

    // 3. offload --json-stream (truncated case)
    let log_input = (1..=60)
        .map(|i| format!("Test trace line {}", i))
        .collect::<Vec<_>>()
        .join("\n");
    let output_offload = Command::cargo_bin("k0maru")
        .unwrap()
        .args(["offload", "--json-stream", "--refs-dir", refs_str])
        .write_stdin(log_input)
        .output()
        .unwrap();

    assert!(output_offload.status.success());
    let stdout_offload = String::from_utf8_lossy(&output_offload.stdout);
    let mut offload_events = Vec::new();
    for line in stdout_offload.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let val: serde_json::Value =
            serde_json::from_str(line).expect("Each line must be valid NDJSON");
        offload_events.push(val["event"].as_str().unwrap().to_string());
    }
    assert!(offload_events.contains(&"stream_chunk".to_string()));
    assert!(offload_events.contains(&"threshold_exceeded".to_string()));
    assert!(offload_events.contains(&"offload_complete".to_string()));

    // 4. offload --json-stream (untruncated case)
    let short_input = "line 1\nline 2\n";
    let output_offload_short = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "offload",
            "--threshold",
            "10",
            "--json-stream",
            "--refs-dir",
            refs_str,
        ])
        .write_stdin(short_input)
        .output()
        .unwrap();

    assert!(output_offload_short.status.success());
    let stdout_short = String::from_utf8_lossy(&output_offload_short.stdout);
    let mut short_events = Vec::new();
    for line in stdout_short.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let val: serde_json::Value =
            serde_json::from_str(line).expect("Each line must be valid NDJSON");
        short_events.push(val["event"].as_str().unwrap().to_string());
    }
    assert!(short_events.contains(&"stream_chunk".to_string()));
    assert!(short_events.contains(&"offload_complete".to_string()));

    // 5. search --json-stream
    let output_search = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "search",
            "Architecture",
            "--json-stream",
            "--vault",
            vault_path,
        ])
        .output()
        .unwrap();

    assert!(output_search.status.success());
    let stdout_search = String::from_utf8_lossy(&output_search.stdout);
    let mut search_events = Vec::new();
    for line in stdout_search.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let val: serde_json::Value =
            serde_json::from_str(line).expect("Each line must be valid NDJSON");
        search_events.push(val["event"].as_str().unwrap().to_string());
    }
    assert!(search_events.contains(&"search_start".to_string()));
    assert!(search_events.contains(&"search_complete".to_string()));

    // 6. gc --json-stream
    let trajectory_json = r#"{
        "session_id": "test_stream_sess",
        "turns": [
            {"turn_index": 0, "role": "system", "content": "You are a coding assistant."},
            {"turn_index": 1, "role": "user", "content": "Please run the tests."},
            {"turn_index": 2, "role": "tool", "content": "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\nline 11\nline 12\nline 13\nline 14\nline 15\nline 16\nline 17\nline 18\nline 19\nline 20\nline 21\nline 22", "tool_name": "cargo test"},
            {"turn_index": 3, "role": "assistant", "content": "Tests completed successfully."}
        ]
    }"#;

    let output_gc = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "gc",
            "--json-stream",
            "--threshold",
            "10",
            "--hot-window",
            "1",
        ])
        .write_stdin(trajectory_json)
        .output()
        .unwrap();

    assert!(output_gc.status.success());
    let stdout_gc = String::from_utf8_lossy(&output_gc.stdout);
    let mut gc_events = Vec::new();
    for line in stdout_gc.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let val: serde_json::Value =
            serde_json::from_str(line).expect("Each line must be valid NDJSON");
        gc_events.push(val["event"].as_str().unwrap().to_string());
    }
    assert!(gc_events.contains(&"session_start".to_string()));
    assert!(gc_events.contains(&"gc_complete".to_string()));
}

#[test]
fn test_cli_posix_exit_codes_conformance() {
    let vault = mock_obsidian_vault();
    let vault_path = vault.path().to_str().unwrap();
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let refs_str = refs_dir.to_str().unwrap();

    // Exit Code 0: Successful commands
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["loadout", "--list", "--vault", vault_path])
        .assert()
        .success()
        .code(0);

    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["search", "Architecture", "--vault", vault_path])
        .assert()
        .success()
        .code(0);

    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["offload", "--refs-dir", refs_str])
        .write_stdin("short log line\n")
        .assert()
        .success()
        .code(0);

    // Stdin query search via '-'
    let wiki = mock_karpathy_wiki();
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["search", "-", "--vault", wiki.path().to_str().unwrap()])
        .write_stdin("SQLite\n")
        .assert()
        .success()
        .code(0);

    // Exit Code 1: Operational failure
    // Empty trajectory to gc is an operational error
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["gc"])
        .write_stdin("   \n")
        .assert()
        .failure()
        .code(1);

    // Exit Code 2: Usage / Syntax / Resource Not Found
    // A: Syntax / invalid argument
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["--invalid-syntax-flag-404"])
        .assert()
        .failure()
        .code(2);

    // B: Missing required subcommand argument
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["search"])
        .assert()
        .failure()
        .code(2);

    // C: Missing inspect node ID
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["inspect", "node_missing_99999", "--refs-dir", refs_str])
        .assert()
        .failure()
        .code(2);

    // D: Missing project in loadout
    Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "loadout",
            "definitely_nonexistent_project_xyz",
            "--vault",
            vault_path,
        ])
        .assert()
        .failure()
        .code(2);

    // E: Missing trajectory file in gc
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["gc", "nonexistent_trajectory_file_xyz.json"])
        .assert()
        .failure()
        .code(2);

    // F: Interactive TTY on offload without piped input
    Command::cargo_bin("k0maru")
        .unwrap()
        .env("K0MARU_FORCE_TTY", "1")
        .args(["offload"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn test_cli_broken_pipe_safety() {
    let vault = mock_obsidian_vault();
    let vault_path = vault.path().to_str().unwrap();

    // Spawn k0maru search which produces multiple lines, but pipe into a reader that closes early
    let bin_path = assert_cmd::cargo::cargo_bin("k0maru");
    let mut child = std::process::Command::new(bin_path)
        .args(["search", "Architecture", "--vault", vault_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child");

    // Close the stdout pipe immediately
    if let Some(stdout) = child.stdout.take() {
        let mut reader = BufReader::new(stdout);
        let mut first_line = String::new();
        let _ = reader.read_line(&mut first_line);
        // Dropping reader closes the read end of stdout pipe
    }

    let output = child
        .wait_with_output()
        .expect("Child must terminate cleanly");
    // Under Unix broken pipe, process exits cleanly (code 0 or 141)
    let code = output.status.code().unwrap_or(0);
    assert!(
        code == 0 || code == 141,
        "Broken pipe should exit cleanly with 0 or 141, got code {}",
        code
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked at"),
        "Broken pipe must not dump panic stack trace, stderr: {}",
        stderr
    );
    assert!(
        !stderr.contains("Broken pipe (os error 32)"),
        "Broken pipe must not dump Rust standard library broken pipe panic: {}",
        stderr
    );
}
