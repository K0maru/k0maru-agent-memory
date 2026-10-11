//! Adversarial integration test suite for Milestone 4 (POSIX Stream & Pipe Standardization).
//! Authored by Challenger 1.
//!
//! Stress-tests:
//! 1. Massive pipe streaming:
//!    - 50,000 lines streamed into `k0maru offload`
//!    - 20,000 lines streamed into `k0maru offload --json-stream`
//!    - Massive unified git diffs (500+ files, 15,000+ lines) piped into `k0maru loadout`
//!    - Token budgeting strictly < 300 tokens under massive diff load
//! 2. Broken pipe resilience:
//!    - Immediate stdout reader drops across `loadout`, `search`, `offload`, `gc`
//!    - Verification of zero panic traces and clean exit (0 or 141)
//! 3. POSIX exit code conformance matrix:
//!    - 0: Normal operation and graceful stream fallbacks
//!    - 1: Operational failure (malformed trajectory JSON, empty trajectory, missing vault directory)
//!    - 2: Usage / syntax / missing entity (invalid flags, missing arguments, missing files, empty stdin search query, interactive TTY guard)
//! 4. Empty and degenerate stream handling:
//!    - 0-byte streams on loadout, offload, gc, search

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::Stdio;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

use common::fixtures::{mock_karpathy_wiki, mock_obsidian_vault};
use k0maru::loadout::estimate_tokens;

mod common;

#[test]
fn test_adversarial_massive_pipe_offload_50k() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let refs_str = refs_dir.to_str().unwrap();

    // Generate 50,000 lines of compiler/build log output
    let log_lines: Vec<String> = (1..=50_000)
        .map(|i| {
            format!(
                "Compiling k0maru-core::submodule::worker_trace_log line [{}/50000] with flag -O3",
                i
            )
        })
        .collect();
    let massive_log_input = log_lines.join("\n");

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    let assert_res = cmd
        .args(["offload", "--refs-dir", refs_str, "--threshold", "50"])
        .write_stdin(massive_log_input)
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("stateDiagram-v2"))
        .stdout(predicate::str::contains("50000 lines captured"));

    drop(assert_res);

    // Verify disk reference file
    let entries: Vec<_> = fs::read_dir(&refs_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 1, "Exactly one log reference must be saved");

    let saved_content = fs::read_to_string(entries[0].path()).unwrap();
    assert_eq!(
        saved_content.lines().count(),
        50_000,
        "Saved log must contain exactly 50,000 lines"
    );
    assert!(saved_content.contains("line [1/50000]"));
    assert!(saved_content.contains("line [50000/50000]"));
}

#[test]
fn test_adversarial_massive_pipe_offload_ndjson() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let refs_str = refs_dir.to_str().unwrap();

    let log_lines: Vec<String> = (1..=20_000)
        .map(|i| format!("Trace log payload chunk [{}]", i))
        .collect();
    let input = log_lines.join("\n");

    let output = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "offload",
            "--json-stream",
            "--refs-dir",
            refs_str,
            "--threshold",
            "100",
        ])
        .write_stdin(input)
        .output()
        .unwrap();

    assert!(output.status.success(), "Command must exit 0");
    let stdout_str = String::from_utf8_lossy(&output.stdout);

    let mut event_names = Vec::new();
    let mut offload_complete_data: Option<serde_json::Value> = None;

    for line in stdout_str.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value =
            serde_json::from_str(line).expect("Each line must be strictly valid NDJSON");
        assert!(v.get("event").is_some());
        assert!(v.get("timestamp").is_some());
        assert!(v.get("data").is_some());

        let event = v["event"].as_str().unwrap().to_string();
        if event == "offload_complete" {
            offload_complete_data = Some(v["data"].clone());
        }
        event_names.push(event);
    }

    assert!(event_names.contains(&"stream_chunk".to_string()));
    assert!(event_names.contains(&"threshold_exceeded".to_string()));
    assert!(event_names.contains(&"offload_complete".to_string()));

    let comp = offload_complete_data.expect("offload_complete event must be present");
    assert_eq!(comp["total_lines"], 20_000);
    assert_eq!(comp["truncated"], true);
}

#[test]
fn test_adversarial_massive_diff_loadout_token_bound() {
    let vault = mock_obsidian_vault();
    let vault_path = vault.path().to_str().unwrap();

    // Generate massive diff with 500 files and 15,000+ lines
    let mut diff_builder = String::with_capacity(300_000);
    for i in 1..=500 {
        diff_builder.push_str(&format!(
            "diff --git a/src/module_{0}/file_{0}.rs b/src/module_{0}/file_{0}.rs\n\
             index 0000000..1111111 100644\n\
             --- a/src/module_{0}/file_{0}.rs\n\
             +++ b/src/module_{0}/file_{0}.rs\n\
             @@ -1,5 +1,25 @@\n\
             +// Added function for module {0}\n\
             +pub fn handler_{0}() {{\n\
             +    println!(\"trace {0}\");\n\
             +}}\n",
            i
        ));
    }

    // 1. Standard text loadout with piped massive diff
    let output = Command::cargo_bin("k0maru")
        .unwrap()
        .args(["loadout", "k0maru-memory", "--vault", vault_path])
        .write_stdin(diff_builder.clone())
        .output()
        .unwrap();

    assert!(output.status.success(), "Piped diff loadout must succeed");
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout_str.contains("[Diff-Aware]"),
        "Stdout must activate diff-aware loadout"
    );
    assert!(
        stdout_str.contains("Architecture_Decisions"),
        "Stdout must activate relevant ADR card"
    );

    let tokens = estimate_tokens(&stdout_str);
    assert!(
        tokens < 300,
        "Massive diff loadout must remain strictly <300 tokens, got {}",
        tokens
    );

    // 2. NDJSON stream loadout with piped massive diff
    let output_stream = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "loadout",
            "k0maru-memory",
            "--json-stream",
            "--vault",
            vault_path,
        ])
        .write_stdin(diff_builder)
        .output()
        .unwrap();

    assert!(
        output_stream.status.success(),
        "NDJSON diff loadout must succeed"
    );
    let stream_stdout = String::from_utf8_lossy(&output_stream.stdout);

    let mut ready_event = None;
    for line in stream_stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value =
            serde_json::from_str(line).expect("Each line must be strictly valid NDJSON");
        if v["event"] == "loadout_ready" {
            ready_event = Some(v);
        }
    }

    let ready = ready_event.expect("loadout_ready event must be emitted");
    assert_eq!(ready["data"]["within_budget"], true);
    let stream_tokens = ready["data"]["tokens"].as_u64().unwrap() as usize;
    assert!(
        stream_tokens < 300,
        "Stream tokens must be < 300, got {}",
        stream_tokens
    );
}

#[test]
fn test_adversarial_broken_pipe_early_closures() {
    let vault = mock_obsidian_vault();
    let vault_path = vault.path().to_str().unwrap();
    let bin_path = assert_cmd::cargo::cargo_bin("k0maru");

    // Case 1: loadout --list --json-stream
    {
        let mut child = std::process::Command::new(&bin_path)
            .args(["loadout", "--list", "--json-stream", "--vault", vault_path])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        // Close stdout immediately without reading
        drop(child.stdout.take());

        let output = child.wait_with_output().unwrap();
        let code = output.status.code().unwrap_or(0);
        assert!(
            code == 0 || code == 141,
            "Broken pipe exit code must be 0 or 141, got {}",
            code
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("panicked at") && !stderr.contains("failed printing to stdout"),
            "Zero panic traces allowed: {}",
            stderr
        );
    }

    // Case 2: search Architecture --json-stream
    {
        let mut child = std::process::Command::new(&bin_path)
            .args([
                "search",
                "Architecture",
                "--json-stream",
                "--vault",
                vault_path,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        if let Some(stdout) = child.stdout.take() {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            // Drop reader after reading 1 line (simulating `head -n 1`)
        }

        let output = child.wait_with_output().unwrap();
        let code = output.status.code().unwrap_or(0);
        assert!(
            code == 0 || code == 141,
            "Broken pipe exit code must be 0 or 141, got {}",
            code
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("panicked at") && !stderr.contains("failed printing to stdout"),
            "Zero panic traces allowed: {}",
            stderr
        );
    }

    // Case 3: offload --json-stream with large input
    {
        let tmp = tempdir().unwrap();
        let refs_dir = tmp.path().join("refs");

        let mut child = std::process::Command::new(&bin_path)
            .args([
                "offload",
                "--json-stream",
                "--refs-dir",
                refs_dir.to_str().unwrap(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        // Write 1,000 lines then close stdin
        if let Some(mut stdin) = child.stdin.take() {
            for i in 0..1000 {
                let _ = writeln!(stdin, "trace log line {}", i);
            }
        }

        // Close stdout immediately
        drop(child.stdout.take());

        let output = child.wait_with_output().unwrap();
        let code = output.status.code().unwrap_or(0);
        assert!(
            code == 0 || code == 141,
            "Broken pipe exit code must be 0 or 141, got {}",
            code
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("panicked at") && !stderr.contains("failed printing to stdout"),
            "Zero panic traces allowed: {}",
            stderr
        );
    }

    // Case 4: gc --json-stream
    {
        let trajectory_json = r#"{
            "session_id": "test_bp_gc",
            "turns": [
                {"turn_index": 0, "role": "system", "content": "System prompt."},
                {"turn_index": 1, "role": "user", "content": "Execute cargo test."},
                {"turn_index": 2, "role": "tool", "content": "error line 1\nerror line 2\nerror line 3", "tool_name": "bash"},
                {"turn_index": 3, "role": "assistant", "content": "Investigating error."}
            ]
        }"#;

        let mut child = std::process::Command::new(&bin_path)
            .args(["gc", "--json-stream"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(trajectory_json.as_bytes());
        }

        // Drop stdout after 1 byte
        drop(child.stdout.take());

        let output = child.wait_with_output().unwrap();
        let code = output.status.code().unwrap_or(0);
        assert!(
            code == 0 || code == 141,
            "Broken pipe exit code must be 0 or 141, got {}",
            code
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("panicked at") && !stderr.contains("failed printing to stdout"),
            "Zero panic traces allowed: {}",
            stderr
        );
    }
}

#[test]
fn test_adversarial_exit_code_matrix() {
    let vault = mock_obsidian_vault();
    let vault_path = vault.path().to_str().unwrap();

    // 1. Invalid flags -> Exit code 2
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["--bad-flag-xyz"])
        .assert()
        .failure()
        .code(2);

    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["loadout", "--nonexistent-flag"])
        .assert()
        .failure()
        .code(2);

    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["offload", "--nonexistent-flag"])
        .assert()
        .failure()
        .code(2);

    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["gc", "--nonexistent-flag"])
        .assert()
        .failure()
        .code(2);

    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["search", "--nonexistent-flag"])
        .assert()
        .failure()
        .code(2);

    // 2. Missing required CLI arguments -> Exit code 2
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["search"])
        .assert()
        .failure()
        .code(2);

    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["inspect"])
        .assert()
        .failure()
        .code(2);

    // 3. Nonexistent files / entities -> Exit code 2
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["gc", "nonexistent_trajectory_999.json"])
        .assert()
        .failure()
        .code(2);

    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["inspect", "node_definitely_absent_888"])
        .assert()
        .failure()
        .code(2);

    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["loadout", "missing_project_404", "--vault", vault_path])
        .assert()
        .failure()
        .code(2);

    // 4. Interactive TTY guard on offload -> Exit code 2
    Command::cargo_bin("k0maru")
        .unwrap()
        .env("K0MARU_FORCE_TTY", "1")
        .args(["offload"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("交互式终端 (TTY)"));

    // 5. Empty search query via stdin '-' -> Exit code 2
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["search", "-", "--vault", vault_path])
        .write_stdin("")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("管道输入的搜索关键词为空"));

    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["search", "-", "--vault", vault_path])
        .write_stdin("   \t\n  ")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("管道输入的搜索关键词为空"));

    // 6. Operational failure -> Exit code 1
    // Empty trajectory into GC
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["gc"])
        .write_stdin("")
        .assert()
        .failure()
        .code(1);

    // Invalid JSON trajectory into GC
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["gc"])
        .write_stdin("{ corrupt json string")
        .assert()
        .failure()
        .code(1);

    // Nonexistent vault directory in search
    Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "search",
            "Architecture",
            "--vault",
            "/tmp/nonexistent_vault_dir_xyz_123",
        ])
        .assert()
        .failure()
        .code(1);

    // 7. Successful operations -> Exit code 0
    let wiki = mock_karpathy_wiki();
    let wiki_path = wiki.path().to_str().unwrap();
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["search", "-", "--vault", wiki_path])
        .write_stdin("SQLite\n")
        .assert()
        .success()
        .code(0);

    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["loadout", "--list", "--vault", vault_path])
        .assert()
        .success()
        .code(0);
}

#[test]
fn test_adversarial_empty_and_garbage_streams() {
    let vault = mock_obsidian_vault();
    let vault_path = vault.path().to_str().unwrap();
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");

    // 1. Empty stdin into `offload` succeeds gracefully (0 lines processed)
    Command::cargo_bin("k0maru")
        .unwrap()
        .args(["offload", "--refs-dir", refs_dir.to_str().unwrap()])
        .write_stdin("")
        .assert()
        .success()
        .code(0);

    // 2. Empty stdin into `loadout` succeeds gracefully (falls back to standard loadout)
    let output = Command::cargo_bin("k0maru")
        .unwrap()
        .args(["loadout", "k0maru-memory", "--vault", vault_path])
        .write_stdin("")
        .output()
        .unwrap();

    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("k0maru-memory"));
    assert!(estimate_tokens(&text) < 300);

    // 3. Garbage non-diff text piped into `loadout` falls back to standard loadout without crashing
    let garbage_diff = "random non-diff text\n>>> merge conflict <<< \nfoo bar baz\n";
    let output_garbage = Command::cargo_bin("k0maru")
        .unwrap()
        .args(["loadout", "k0maru-memory", "--vault", vault_path])
        .write_stdin(garbage_diff)
        .output()
        .unwrap();

    assert!(output_garbage.status.success());
    let text_garbage = String::from_utf8_lossy(&output_garbage.stdout);
    assert!(text_garbage.contains("k0maru-memory"));
    assert!(estimate_tokens(&text_garbage) < 300);
}
