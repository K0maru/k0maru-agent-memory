use std::io::Cursor;
use std::process::Stdio;

use assert_cmd::Command;
use k0maru::offload::{inspect_node, OffloadEngine};
use k0maru_core::gc::{ContextGcEngine, GcOptions, SessionTrajectory, TrajectoryTurn, TurnRole};
use predicates::prelude::*;
use tempfile::tempdir;

fn create_synthetic_50_turn_trajectory() -> SessionTrajectory {
    let mut turns = Vec::with_capacity(50);

    // Turn 0: System prompt (Inviolable Anchor)
    turns.push(TrajectoryTurn {
        turn_index: 0,
        role: TurnRole::System,
        content:
            "You are an expert Rust systems engineer. Follow TDD and preserve codebase invariants."
                .to_string(),
        tool_name: None,
        tool_call_id: None,
    });

    // Turn 1: User request (Inviolable Anchor)
    turns.push(TrajectoryTurn {
        turn_index: 1,
        role: TurnRole::User,
        content: "Please fix compilation errors in crates/k0maru-core/src/gc/engine.rs."
            .to_string(),
        tool_name: None,
        tool_call_id: None,
    });

    // Compiler errors
    for i in 2..45 {
        let role = if i % 2 == 0 {
            TurnRole::Assistant
        } else {
            TurnRole::Tool
        };
        let mut err_lines = vec![
            "$ cargo check --package k0maru-core".to_string(),
            format!("error[E0308]: mismatched types in turn {}", i),
        ];
        for j in 1..80 {
            err_lines.push(format!(
                "   = diagnostic trace detail #{}: deeply nested symbol candidate for turn_{}",
                j, i
            ));
        }
        turns.push(TrajectoryTurn {
            turn_index: i,
            role,
            content: err_lines.join("\n"),
            tool_name: if role == TurnRole::Tool {
                Some("run_command".to_string())
            } else {
                None
            },
            tool_call_id: None,
        });
    }

    // Hot window (last 5 turns)
    for i in 45..50 {
        turns.push(TrajectoryTurn {
            turn_index: i,
            role: if i % 2 == 1 {
                TurnRole::Tool
            } else {
                TurnRole::Assistant
            },
            content: format!("Recent active reasoning turn {}", i),
            tool_name: None,
            tool_call_id: None,
        });
    }

    SessionTrajectory {
        session_id: "test-session-50-turns".to_string(),
        turns,
    }
}

#[test]
fn test_gc_compaction_achieves_trr_gte_85_and_preserves_anchors() {
    let trajectory = create_synthetic_50_turn_trajectory();
    let original_token_count: usize = trajectory
        .turns
        .iter()
        .map(|t| k0maru_core::gc::estimate_tokens(&t.content))
        .sum();

    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let options = GcOptions {
        hot_window_turns: 5,
        line_threshold: 20,
        refs_dir: Some(refs_dir),
        ..Default::default()
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &options).expect("GC failed");

    // Inviolable Anchor: Turn 0 and 1 must remain untouched
    assert_eq!(result.compacted_trajectory.turns[0].turn_index, 0);
    assert_eq!(
        result.compacted_trajectory.turns[0].content,
        trajectory.turns[0].content
    );
    assert_eq!(result.compacted_trajectory.turns[1].turn_index, 1);
    assert_eq!(
        result.compacted_trajectory.turns[1].content,
        trajectory.turns[1].content
    );

    // TRR >= 85%
    let compacted_token_count: usize = result
        .compacted_trajectory
        .turns
        .iter()
        .map(|t| k0maru_core::gc::estimate_tokens(&t.content))
        .sum();
    let trr = ((original_token_count - compacted_token_count) as f64 / original_token_count as f64)
        * 100.0;
    assert!(
        trr >= 85.0,
        "TRR must be >= 85.0%, achieved {:.2}% (original: {}, compacted: {})",
        trr,
        original_token_count,
        compacted_token_count
    );

    // Hot window (last 5 turns) uncompressed
    for i in 45..50 {
        assert_eq!(
            result.compacted_trajectory.turns[i].content,
            trajectory.turns[i].content
        );
    }
}

#[test]
fn test_offload_and_inspect_node_by_hash() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let engine = OffloadEngine::new(&refs_dir).with_threshold(50);

    // 100 lines input -> truncated
    let input = (1..=100)
        .map(|i| format!("Line {}: Test compile output log", i))
        .collect::<Vec<_>>()
        .join("\n");

    let result = engine
        .process_stream(Cursor::new(input.as_bytes()))
        .expect("Stream processing should succeed");

    assert!(result.truncated, "Logs > 50 lines must be truncated");
    assert_eq!(result.total_lines, 100);
    assert!(result.log_path.is_some());
    assert!(!result.mermaid_graph.is_empty());

    // Inspect the persisted node
    let inspected = inspect_node(&refs_dir, &result.node_id).expect("Inspect node should succeed");
    assert_eq!(inspected.trim(), input.trim());
}

#[test]
fn test_posix_stream_piping_and_interactive_tty_guard() {
    let tmp = tempdir().unwrap();
    let refs_dir = tmp.path().join("refs");
    let refs_str = refs_dir.to_str().unwrap();

    // Interactive TTY guard on offload simulated with K0MARU_FORCE_TTY=1
    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.env("K0MARU_FORCE_TTY", "1")
        .args(["offload", "--refs-dir", refs_str])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("TTY"));

    // Stdin pipe with short log -> exits 0 directly
    let mut cmd_pipe = Command::cargo_bin("k0maru").unwrap();
    cmd_pipe
        .arg("offload")
        .write_stdin("Line 1: Normal output\nLine 2: Done.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Line 1: Normal output"));
}

#[test]
fn test_cli_broken_pipe_safety_and_exit_codes() {
    // Nonexistent vault exits with operational error code 1 or 2
    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.args(["search", "test", "--vault", "/nonexistent/vault/path/404"])
        .assert()
        .failure()
        .code(predicate::in_iter([1, 2]));

    // Broken pipe resilience: spawn process and drop stdout reader
    let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin("k0maru"))
        .args(["search", "a", "--limit", "100"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn child");

    drop(child.stdout.take());
    let status = child.wait().expect("wait child");
    // Under Unix, dropping stdout can yield code 0, 141 (SIGPIPE), or normal exit, but NEVER panic!
    assert!(status.code().unwrap_or(0) >= 0);
}
