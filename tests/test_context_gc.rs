use assert_cmd::Command;
use k0maru_core::gc::{
    ContextGcEngine, GcOptions, GcResult, SessionTrajectory, TrajectoryTurn, TurnRole,
};
use serde_json::{json, Value};
use tempfile::tempdir;

/// Helper to generate a realistic 50-turn synthetic debugging trajectory.
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
        content: "Please fix the compilation errors in crates/k0maru-core/src/gc/engine.rs and ensure cargo test passes."
            .to_string(),
        tool_name: None,
        tool_call_id: None,
    });

    // Helper to generate a verbose compiler dump (~100 lines, ~2,000 tokens)
    let make_compiler_error_dump = |error_code: &str, msg: &str, turn_idx: usize| {
        let mut lines = vec![
            "$ cargo check --package k0maru-core".to_string(),
            "   Compiling k0maru-core v0.8.2 (/workspace/crates/k0maru-core)".to_string(),
            format!("error[{}]: {}", error_code, msg),
            "  --> crates/k0maru-core/src/gc/engine.rs:42:13".to_string(),
            "   |".to_string(),
            "42 |     let res = compute_tokens(&turn.content);".to_string(),
            "   |               ^^^^^^^^^^^^^^ mismatched types".to_string(),
            "   |".to_string(),
            "   = note: expected struct `Option<usize>`".to_string(),
            "              found type `usize`".to_string(),
        ];
        for i in 1..100 {
            lines.push(format!(
                "   = diagnostic trace detail #{}: inspecting symbol table turn_{} frame_{} with deeply nested traits and type bounds",
                i, turn_idx, i * 3
            ));
        }
        lines.push("error: could not compile `k0maru-core` due to previous error".to_string());
        lines.join("\n")
    };

    // Helper to generate a verbose passing test dump (~80 lines, ~1,600 tokens)
    let make_test_output_dump = |turn_idx: usize| {
        let mut lines = vec![
            "$ cargo test --package k0maru-core".to_string(),
            "   Compiling k0maru-core v0.8.2".to_string(),
            "    Finished test [unoptimized + debuginfo] target(s)".to_string(),
            "     Running unittests src/lib.rs".to_string(),
        ];
        for i in 0..80 {
            lines.push(format!(
                "test gc::tests::subsystem_suite_turn_{}_case_{} ... ok",
                turn_idx, i
            ));
        }
        lines.push("test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured".to_string());
        lines.join("\n")
    };

    // Turns 2 to 44: Historical turns (Turns 2..45)
    // Alternate between Assistant actions and Tool executions with compiler error dumps.
    for i in 2..45 {
        if i % 2 == 1 {
            // Tool turn with verbose compiler error or test run
            let tool_content = if i < 22 {
                make_compiler_error_dump("E0308", "mismatched types", i)
            } else if i < 36 {
                make_compiler_error_dump("E0425", "cannot find value in this scope", i)
            } else {
                make_test_output_dump(i)
            };

            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Tool,
                content: tool_content,
                tool_name: Some("cargo".to_string()),
                tool_call_id: Some(format!("call_{}", i)),
            });
        } else {
            // Assistant turn reporting status or patch
            let content = if i == 22 {
                "Fixed error[E0308] by aligning return types; proceeding to resolve remaining warnings."
                    .to_string()
            } else if i == 36 {
                "Compilation succeeded! All compiler errors resolved; running cargo test suite now."
                    .to_string()
            } else {
                format!(
                    "Turn {}: Applied patch to crates/k0maru-core/src/gc/engine.rs and verifying with cargo.",
                    i
                )
            };

            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Assistant,
                content,
                tool_name: None,
                tool_call_id: None,
            });
        }
    }

    // Turns 45 to 49: Hot Window (latest 5 turns untouched)
    turns.push(TrajectoryTurn {
        turn_index: 45,
        role: TurnRole::Assistant,
        content: "Hot window turn 45: Running final integration verification.".to_string(),
        tool_name: None,
        tool_call_id: None,
    });
    turns.push(TrajectoryTurn {
        turn_index: 46,
        role: TurnRole::Tool,
        content: "test result: ok. 195 passed; 0 failed; finished in 0.42s".to_string(),
        tool_name: Some("cargo test".to_string()),
        tool_call_id: Some("call_46".to_string()),
    });
    turns.push(TrajectoryTurn {
        turn_index: 47,
        role: TurnRole::Assistant,
        content: "All 195 tests pass. Let me now run cargo clippy -- -D warnings.".to_string(),
        tool_name: None,
        tool_call_id: None,
    });
    turns.push(TrajectoryTurn {
        turn_index: 48,
        role: TurnRole::Tool,
        content: "    Checking k0maru-core v0.8.2\n    Finished dev target(s) in 0.15s\n0 warnings"
            .to_string(),
        tool_name: Some("cargo clippy".to_string()),
        tool_call_id: Some("call_48".to_string()),
    });
    turns.push(TrajectoryTurn {
        turn_index: 49,
        role: TurnRole::Assistant,
        content: "Final verification complete: 0 warnings, 100% test pass rate.".to_string(),
        tool_name: None,
        tool_call_id: None,
    });

    assert_eq!(turns.len(), 50);
    SessionTrajectory {
        session_id: "test_session_50_turns".to_string(),
        turns,
    }
}

#[test]
fn test_gc_50_turn_synthetic_trajectory_achieves_trr_gte_85() {
    let trajectory = create_synthetic_50_turn_trajectory();
    let options = GcOptions {
        line_threshold: 20,
        hot_window_turns: 5,
        context_window_tokens: 32_000,
        ..Default::default()
    };

    let engine = ContextGcEngine::new();
    let result: GcResult = engine
        .compact(&trajectory, &options)
        .expect("Compaction should succeed");

    assert_eq!(result.raw_turns_count, 50);
    assert_eq!(result.compacted_turns_count, 50);
    assert!(
        result.raw_tokens > 20_000,
        "Raw tokens should be substantial"
    );
    assert!(
        result.compacted_tokens < result.raw_tokens,
        "Compacted tokens must be less than raw tokens"
    );

    // Key Acceptance Requirement: TRR >= 85.0%
    assert!(
        result.token_reduction_ratio >= 85.0,
        "Expected TRR >= 85.0%, got {:.2}% (raw: {}, compacted: {})",
        result.token_reduction_ratio,
        result.raw_tokens,
        result.compacted_tokens
    );
    assert!(
        result.trr_fraction() >= 0.85,
        "trr_fraction() should be >= 0.85"
    );
    assert!(result.folded_nodes_count > 0);
}

#[test]
fn test_gc_preserves_anchor_invariants() {
    // Inviolable Anchor Guard (Tier 0): System and User turns must NEVER be modified or pruned,
    // regardless of content length, lines, or compiler diagnostic snippets.
    let system_content = "SYSTEM PROMPT:\n".to_string() + &"You are an assistant.\n".repeat(80);
    let user_content = "USER REQUEST:\nHelp me fix error[E0308]: mismatched types in main.rs\n"
        .to_string()
        + &"Here is extra context line\n".repeat(60);

    let trajectory = SessionTrajectory {
        session_id: "test_anchors".to_string(),
        turns: vec![
            TrajectoryTurn {
                turn_index: 0,
                role: TurnRole::System,
                content: system_content.clone(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 1,
                role: TurnRole::User,
                content: user_content.clone(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 2,
                role: TurnRole::Tool,
                content: "error[E0308]: mismatched types\n".to_string()
                    + &"trace line\n".repeat(40),
                tool_name: Some("cargo check".to_string()),
                tool_call_id: Some("call_2".to_string()),
            },
            TrajectoryTurn {
                turn_index: 3,
                role: TurnRole::Assistant,
                content: "Compilation succeeded in turn 3! Finished dev".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
        ],
    };

    let options = GcOptions {
        line_threshold: 10,
        hot_window_turns: 1, // Only turn 3 in hot window
        ..Default::default()
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &options).unwrap();

    // Turn 0 (System) must remain 100% verbatim
    assert_eq!(
        result.compacted_trajectory.turns[0].content, system_content,
        "System turn must be preserved verbatim"
    );
    // Turn 1 (User) must remain 100% verbatim
    assert_eq!(
        result.compacted_trajectory.turns[1].content, user_content,
        "User turn must be preserved verbatim"
    );
    // Turn 2 (Tool) outside hot window must be folded
    assert!(
        result.compacted_trajectory.turns[2]
            .content
            .contains("Superseded Compilation Error")
            || result.compacted_trajectory.turns[2]
                .content
                .contains("Offloaded Tool Output"),
        "Historical Tool output should be compacted"
    );
}

#[test]
fn test_gc_hot_window_retention() {
    // Hot Window (Tier 1): The most recent K turns must be preserved untouched.
    let mut turns = Vec::new();
    for i in 0..10 {
        turns.push(TrajectoryTurn {
            turn_index: i,
            role: if i % 2 == 0 {
                TurnRole::Assistant
            } else {
                TurnRole::Tool
            },
            content: format!(
                "Tool or assistant verbose log line:\n{}",
                "data\n".repeat(30)
            ),
            tool_name: Some("test_tool".to_string()),
            tool_call_id: Some(format!("call_{}", i)),
        });
    }

    let trajectory = SessionTrajectory {
        session_id: "test_hot_window".to_string(),
        turns,
    };

    let k = 4;
    let options = GcOptions {
        line_threshold: 10,
        hot_window_turns: k,
        ..Default::default()
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &options).unwrap();

    // Last K turns (indices 6, 7, 8, 9) must be untouched
    for idx in (10 - k)..10 {
        assert_eq!(
            result.compacted_trajectory.turns[idx].content, trajectory.turns[idx].content,
            "Turn {} in hot window must be preserved verbatim",
            idx
        );
    }

    // Historical Tool turns before 10 - k must be folded
    assert!(result.compacted_trajectory.turns[1]
        .content
        .contains("Offloaded Tool Output"));
    assert!(result.compacted_trajectory.turns[3]
        .content
        .contains("Offloaded Tool Output"));
    assert!(result.compacted_trajectory.turns[5]
        .content
        .contains("Offloaded Tool Output"));
}

#[test]
fn test_gc_smart_zone_bounds() {
    let trajectory = create_synthetic_50_turn_trajectory();
    let engine = ContextGcEngine::new();

    // 1. Sizing context window so compacted tokens (~1,200) fall inside 15% - 60%
    let options_inside = GcOptions {
        line_threshold: 20,
        hot_window_turns: 5,
        // With compacted tokens ~1,200, a 4,000 token active window places occupancy at ~30%
        context_window_tokens: 4_000,
        smart_zone_min_ratio: 0.15,
        smart_zone_max_ratio: 0.60,
        ..Default::default()
    };

    let result = engine.compact(&trajectory, &options_inside).unwrap();
    assert!(
        result.in_smart_zone,
        "Compacted tokens ({}) in 4k window should be within 15%-60% Smart-Zone (occupancy: {:.2}%)",
        result.compacted_tokens,
        result.smart_zone_occupancy * 100.0
    );
    assert!(result.smart_zone_occupancy >= 0.15);
    assert!(result.smart_zone_occupancy <= 0.60);

    // 2. Test when context window is large (20,000 tokens) -> occupancy < 15%
    let options_too_large = GcOptions {
        context_window_tokens: 20_000,
        ..options_inside.clone()
    };
    let result_large = engine.compact(&trajectory, &options_too_large).unwrap();
    assert!(
        !result_large.in_smart_zone,
        "Occupancy in 20k window should be below 15%"
    );
    assert!(result_large.smart_zone_occupancy < 0.15);

    // 3. Test when context window is tiny (1,500 tokens) -> occupancy > 60%
    let options_too_small = GcOptions {
        context_window_tokens: 1_500,
        ..options_inside
    };
    let result_small = engine.compact(&trajectory, &options_too_small).unwrap();
    assert!(
        !result_small.in_smart_zone,
        "Occupancy in 1.5k window should exceed 60%"
    );
    assert!(result_small.smart_zone_occupancy > 0.60);
}

#[test]
fn test_gc_cli_stdin_pipe() {
    let trajectory = create_synthetic_50_turn_trajectory();
    let json_bytes = serde_json::to_vec(&trajectory).unwrap();

    // Test piping into `k0maru gc --threshold 20 --hot-window 5 --context-window 25000 --json`
    let mut cmd = Command::cargo_bin("k0maru").expect("Binary k0maru should exist");
    let output = cmd
        .args([
            "gc",
            "--threshold",
            "20",
            "--hot-window",
            "5",
            "--context-window",
            "25000",
            "--json",
        ])
        .write_stdin(json_bytes.clone())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: GcResult =
        serde_json::from_slice(&output).expect("Output should be valid GcResult JSON");
    assert_eq!(parsed.raw_turns_count, 50);
    assert!(parsed.token_reduction_ratio >= 85.0);

    // Test line-delimited `--json-stream` output
    let mut stream_cmd = Command::cargo_bin("k0maru").expect("Binary k0maru should exist");
    let stream_out = stream_cmd
        .args(["gc", "--json-stream"])
        .write_stdin(json_bytes)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout_str = String::from_utf8_lossy(&stream_out);
    let lines: Vec<&str> = stdout_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert!(
        lines.len() >= 3,
        "Expected at least session_start, turn_folded, gc_complete events"
    );

    // Verify first event is session_start and last is gc_complete
    let first_ev: Value = serde_json::from_str(lines[0]).expect("Valid JSON event");
    assert_eq!(first_ev["event"], "session_start");
    let last_ev: Value = serde_json::from_str(lines.last().unwrap()).expect("Valid JSON event");
    assert_eq!(last_ev["event"], "gc_complete");
}

#[test]
fn test_mcp_gc_session_context() {
    let temp = tempdir().unwrap();
    let mut server = k0maru::mcp::McpServer::new(temp.path());

    let trajectory = create_synthetic_50_turn_trajectory();
    let trajectory_json = serde_json::to_string(&trajectory).unwrap();

    let req = json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "tools/call",
        "params": {
            "name": "gc_session_context",
            "arguments": {
                "trajectory_json": trajectory_json,
                "threshold": 20,
                "hot_window": 5,
                "context_window": 25000
            }
        }
    });

    let resp_str = server
        .handle_line(&req.to_string())
        .expect("Server should respond to tools/call gc_session_context");

    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON-RPC response");
    assert_eq!(resp["id"], 42);
    assert!(
        resp.get("error").is_none(),
        "Response should not contain error: {:?}",
        resp
    );

    let content_text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("Result text payload");
    let gc_res: GcResult =
        serde_json::from_str(content_text).expect("Text payload should be serialized GcResult");

    assert_eq!(gc_res.raw_turns_count, 50);
    assert_eq!(gc_res.compacted_turns_count, 50);
    assert!(gc_res.token_reduction_ratio >= 85.0);
    assert!(gc_res.folded_nodes_count > 0);
}

#[test]
fn test_gc_short_trajectory_and_edge_cases() {
    let engine = ContextGcEngine::new();
    let options = GcOptions::default();

    // 1. Empty trajectory
    let empty_traj = SessionTrajectory {
        session_id: "empty".to_string(),
        turns: vec![],
    };
    let empty_res = engine.compact(&empty_traj, &options).unwrap();
    assert_eq!(empty_res.raw_turns_count, 0);
    assert_eq!(empty_res.raw_tokens, 0);
    assert_eq!(empty_res.compacted_tokens, 0);

    // 2. Trajectory with <= hot_window turns (e.g. 3 turns)
    let short_traj = SessionTrajectory {
        session_id: "short".to_string(),
        turns: vec![
            TrajectoryTurn {
                turn_index: 0,
                role: TurnRole::User,
                content: "Hello".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 1,
                role: TurnRole::Assistant,
                content: "Hi! How can I help?".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 2,
                role: TurnRole::Tool,
                content: "Short tool output".to_string(),
                tool_name: Some("tool".to_string()),
                tool_call_id: None,
            },
        ],
    };
    let short_res = engine.compact(&short_traj, &options).unwrap();
    assert_eq!(short_res.folded_nodes_count, 0);
    assert_eq!(short_res.raw_tokens, short_res.compacted_tokens);
    assert_eq!(short_res.token_reduction_ratio, 0.0);
}

#[test]
fn test_gc_persists_raw_logs_to_refs() {
    let temp = tempdir().unwrap();
    let refs_dir = temp.path().join("refs");

    let trajectory = SessionTrajectory {
        session_id: "test_refs".to_string(),
        turns: vec![
            TrajectoryTurn {
                turn_index: 0,
                role: TurnRole::User,
                content: "Run build".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 1,
                role: TurnRole::Tool,
                content: "line 1\n".to_string() + &"log entry line\n".repeat(30),
                tool_name: Some("cargo".to_string()),
                tool_call_id: Some("call_1".to_string()),
            },
            TrajectoryTurn {
                turn_index: 2,
                role: TurnRole::Assistant,
                content: "Finished".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
        ],
    };

    let options = GcOptions {
        line_threshold: 10,
        hot_window_turns: 1,
        refs_dir: Some(refs_dir.clone()),
        ..Default::default()
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &options).unwrap();

    assert_eq!(result.folded_nodes_count, 1);
    let folded_content = &result.compacted_trajectory.turns[1].content;
    assert!(folded_content.contains("[Offloaded Tool Output: NodeID node_"));

    // Verify file written to refs dir
    assert!(refs_dir.exists(), "Refs dir should be created");
    let entries: Vec<_> = std::fs::read_dir(&refs_dir).unwrap().collect();
    assert_eq!(entries.len(), 1, "Exactly one log file should be persisted");
    let log_file = entries[0].as_ref().unwrap().path();
    assert!(log_file.extension().map(|e| e == "log").unwrap_or(false));
    let content = std::fs::read_to_string(log_file).unwrap();
    assert!(content.contains("log entry line"));
}

#[test]
fn test_gc_tier3_superseded_error_citation_format() {
    let trajectory = SessionTrajectory {
        session_id: "test_tier3_format".to_string(),
        turns: vec![
            TrajectoryTurn {
                turn_index: 0,
                role: TurnRole::User,
                content: "Compile code".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 1,
                role: TurnRole::Tool,
                content: "error[E0308]: mismatched types\n  --> src/main.rs:42:13\n   | extra trace line\n".to_string()
                    + &"trace detail line\n".repeat(25),
                tool_name: Some("cargo check".to_string()),
                tool_call_id: Some("call_1".to_string()),
            },
            TrajectoryTurn {
                turn_index: 2,
                role: TurnRole::Assistant,
                content: "Fixing type annotation.".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 3,
                role: TurnRole::Tool,
                content: "Finished dev [unoptimized + debuginfo] target(s)\ntest result: ok. 10 passed; 0 failed".to_string(),
                tool_name: Some("cargo test".to_string()),
                tool_call_id: Some("call_3".to_string()),
            },
            TrajectoryTurn {
                turn_index: 4,
                role: TurnRole::Assistant,
                content: "Done!".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
        ],
    };

    let options = GcOptions {
        line_threshold: 10,
        hot_window_turns: 2, // Turns 3 and 4 in hot window
        ..Default::default()
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &options).unwrap();

    let folded_turn = &result.compacted_trajectory.turns[1];
    assert!(
        folded_turn
            .content
            .starts_with("[Superseded Compilation Error:"),
        "Content should start with citation tag: {}",
        folded_turn.content
    );
    assert!(
        folded_turn
            .content
            .contains("error[E0308]: mismatched types"),
        "Citation must include error summary: {}",
        folded_turn.content
    );
    assert!(
        folded_turn
            .content
            .contains("(Turn 1) -> Resolved in Turn 3]"),
        "Citation must indicate resolution turn: {}",
        folded_turn.content
    );
}

#[test]
fn test_gc_cli_file_argument_and_error_codes() {
    let temp = tempdir().unwrap();
    let traj_file = temp.path().join("trajectory.json");

    let trajectory = create_synthetic_50_turn_trajectory();
    let json_str = serde_json::to_string_pretty(&trajectory).unwrap();
    std::fs::write(&traj_file, json_str).unwrap();

    // 1. Valid file path
    let mut cmd = Command::cargo_bin("k0maru").expect("Binary k0maru should exist");
    let output = cmd
        .args([
            "gc",
            traj_file.to_str().unwrap(),
            "--threshold",
            "20",
            "--hot-window",
            "5",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: GcResult =
        serde_json::from_slice(&output).expect("Output should be valid GcResult JSON");
    assert_eq!(parsed.raw_turns_count, 50);
    assert!(parsed.token_reduction_ratio >= 85.0);

    // 2. Non-existent file path exits with code 2
    let mut bad_cmd = Command::cargo_bin("k0maru").expect("Binary k0maru should exist");
    bad_cmd
        .args(["gc", "/nonexistent/path/trajectory.json"])
        .assert()
        .failure()
        .code(2);
}
