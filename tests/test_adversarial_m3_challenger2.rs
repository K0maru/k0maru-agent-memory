use std::time::Instant;

use assert_cmd::Command;
use k0maru_core::gc::{
    estimate_tokens, ContextGcEngine, GcOptions, GcResult, SessionTrajectory, TrajectoryTurn,
    TurnRole,
};
use serde_json::{json, Value};
use tempfile::tempdir;

// ============================================================================
// Helper Utilities for Adversarial Generators
// ============================================================================

fn make_huge_compiler_error(lines_count: usize, error_code: &str) -> String {
    let mut lines = Vec::with_capacity(lines_count + 5);
    lines.push(format!(
        "error[{}]: intentional stress test error",
        error_code
    ));
    lines.push("  --> crates/k0maru-core/src/gc/engine.rs:999:1".to_string());
    lines.push("   |".to_string());
    for i in 0..lines_count {
        lines.push(format!(
            "   = adversarial compiler trace line {:06}: deeply nested generic type parameter <T as Trait>::AssocType failure",
            i
        ));
    }
    lines.push("error: could not compile `k0maru-core` due to previous error".to_string());
    lines.join("\n")
}

fn create_adversarial_trajectory(turn_count: usize, error_line_depth: usize) -> SessionTrajectory {
    let mut turns = Vec::with_capacity(turn_count);

    // Turn 0: System prompt (Inviolable Anchor)
    turns.push(TrajectoryTurn {
        turn_index: 0,
        role: TurnRole::System,
        content: "SYSTEM_ANCHOR: You are an autonomous agent under strict invariants.".to_string(),
        tool_name: None,
        tool_call_id: None,
    });

    // Turn 1: User prompt (Inviolable Anchor)
    turns.push(TrajectoryTurn {
        turn_index: 1,
        role: TurnRole::User,
        content: "USER_ANCHOR: Please eliminate all compilation bottlenecks.".to_string(),
        tool_name: None,
        tool_call_id: None,
    });

    let historical_turns = turn_count.saturating_sub(7); // Reserve last 5 for hot window

    for i in 2..historical_turns {
        if i % 2 == 0 {
            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Assistant,
                content: format!("Turn {}: Applying refactoring patch to codebase.", i),
                tool_name: None,
                tool_call_id: None,
            });
        } else {
            let error_code = if i < historical_turns / 2 {
                "E0308"
            } else {
                "E0425"
            };
            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Tool,
                content: make_huge_compiler_error(error_line_depth, error_code),
                tool_name: Some("cargo".to_string()),
                tool_call_id: Some(format!("call_{}", i)),
            });
        }
    }

    // A turn indicating resolution of the historical errors
    let resolve_turn_idx = historical_turns;
    turns.push(TrajectoryTurn {
        turn_index: resolve_turn_idx,
        role: TurnRole::Tool,
        content: "test result: ok. 150 passed; 0 failed; finished in 0.88s\nBuild succeeded!"
            .to_string(),
        tool_name: Some("cargo test".to_string()),
        tool_call_id: Some(format!("call_{}", resolve_turn_idx)),
    });

    // Hot window turns (last 5 turns)
    for i in (resolve_turn_idx + 1)..turn_count {
        if i % 2 == 1 {
            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Tool,
                content: format!("Recent tool execution log line for hot window turn {}", i),
                tool_name: Some("cargo check".to_string()),
                tool_call_id: Some(format!("call_{}", i)),
            });
        } else {
            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Assistant,
                content: format!("Recent assistant reply in hot window turn {}", i),
                tool_name: None,
                tool_call_id: None,
            });
        }
    }

    SessionTrajectory {
        session_id: format!("adversarial_traj_{}_turns", turn_count),
        turns,
    }
}

// ============================================================================
// 1. INVIOLABLE ANCHOR GUARD ADVERSARIAL STRESS SUITE
// ============================================================================

#[test]
fn test_adversarial_anchor_guard_extreme_compaction_never_drops_anchors() {
    let engine = ContextGcEngine::new();

    // Fabricate massive system and user prompts loaded with compiler errors,
    // tracebacks, and over 5,000 lines each.
    let massive_system_prompt = format!(
        "SYSTEM_MISSION_CRITICAL_PROMPT:\n{}\nerror[E0308]: mismatched types\n{}\nTraceback (most recent call last):\nException: Fatal Crash\n{}",
        "System directive line alpha\n".repeat(2000),
        "System directive line beta\n".repeat(1500),
        "System invariant omega\n".repeat(1500)
    );

    let massive_user_prompt = format!(
        "USER_ORIGINAL_SPECIFICATION:\n{}\nerror[E0425]: cannot find value in scope\n{}\nfatal error: stdlib.h missing\n{}",
        "User specification clause 1\n".repeat(2000),
        "User specification clause 2\n".repeat(1500),
        "User final requirement clause\n".repeat(1500)
    );

    let trajectory = SessionTrajectory {
        session_id: "test_anchor_guard_extreme".to_string(),
        turns: vec![
            TrajectoryTurn {
                turn_index: 0,
                role: TurnRole::System,
                content: massive_system_prompt.clone(),
                tool_name: Some("should_be_preserved".to_string()),
                tool_call_id: Some("system_call_id".to_string()),
            },
            TrajectoryTurn {
                turn_index: 1,
                role: TurnRole::User,
                content: massive_user_prompt.clone(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 2,
                role: TurnRole::Tool,
                content: make_huge_compiler_error(100, "E0308"),
                tool_name: Some("cargo".to_string()),
                tool_call_id: Some("call_2".to_string()),
            },
            TrajectoryTurn {
                turn_index: 3,
                role: TurnRole::Assistant,
                content: "Finished dev target(s); test result: ok".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
        ],
    };

    // Stress with extreme compaction options:
    // line_threshold: 0 -> fold anything foldable immediately
    // hot_window_turns: 0 -> no hot window protection at all!
    // context_window_tokens: 1 -> extreme context pressure
    let extreme_options = GcOptions {
        line_threshold: 0,
        hot_window_turns: 0,
        context_window_tokens: 1,
        smart_zone_min_ratio: 0.0,
        smart_zone_max_ratio: 0.0,
        refs_dir: None,
    };

    let result = engine
        .compact(&trajectory, &extreme_options)
        .expect("Compaction must succeed without error");

    // Inviolable Assertion: Turn 0 (System) MUST be identical byte-for-byte!
    assert_eq!(
        result.compacted_trajectory.turns[0].content, massive_system_prompt,
        "Anchor Guard violated: System prompt was altered or truncated under extreme options!"
    );
    assert_eq!(result.compacted_trajectory.turns[0].role, TurnRole::System);
    assert_eq!(
        result.compacted_trajectory.turns[0].tool_name,
        Some("should_be_preserved".to_string())
    );

    // Inviolable Assertion: Turn 1 (User) MUST be identical byte-for-byte!
    assert_eq!(
        result.compacted_trajectory.turns[1].content, massive_user_prompt,
        "Anchor Guard violated: User prompt was altered or truncated under extreme options!"
    );
    assert_eq!(result.compacted_trajectory.turns[1].role, TurnRole::User);

    // Meanwhile, non-anchor Turn 2 Tool output MUST have been folded
    assert!(
        result.compacted_trajectory.turns[2]
            .content
            .contains("[Superseded Compilation Error:")
            || result.compacted_trajectory.turns[2]
                .content
                .contains("[Offloaded Tool Output:"),
        "Tool turn 2 should have been folded under threshold 0"
    );
}

#[test]
fn test_adversarial_anchor_guard_interleaved_multi_turn_anchors() {
    let engine = ContextGcEngine::new();

    // Construct a multi-turn trajectory where User instructions appear repeatedly
    // throughout historical turns (e.g. human providing iterative steering).
    let mut turns = Vec::new();
    let mut expected_anchors = std::collections::HashMap::new();

    for i in 0..30 {
        if i == 0 {
            let content = "System initial instruction".to_string();
            expected_anchors.insert(i, content.clone());
            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::System,
                content,
                tool_name: None,
                tool_call_id: None,
            });
        } else if i == 5 || i == 12 || i == 18 {
            let content = format!(
                "Iterative user steering at turn {}: error[E0999] please fix this now!\n{}",
                i,
                "Extra steering line\n".repeat(50)
            );
            expected_anchors.insert(i, content.clone());
            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::User,
                content,
                tool_name: None,
                tool_call_id: None,
            });
        } else if i % 2 == 1 {
            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Tool,
                content: make_huge_compiler_error(60, "E0308"),
                tool_name: Some("cargo".to_string()),
                tool_call_id: Some(format!("call_{}", i)),
            });
        } else {
            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Assistant,
                content: format!("Assistant step {}", i),
                tool_name: None,
                tool_call_id: None,
            });
        }
    }

    let trajectory = SessionTrajectory {
        session_id: "interleaved_anchors".to_string(),
        turns,
    };

    let options = GcOptions {
        line_threshold: 10,
        hot_window_turns: 3,
        ..Default::default()
    };

    let result = engine
        .compact(&trajectory, &options)
        .expect("Compaction should succeed");

    // Verify EVERY registered anchor turn remained 100% exact
    for (turn_idx, expected_content) in expected_anchors {
        let compacted_turn = &result.compacted_trajectory.turns[turn_idx];
        assert_eq!(
            &compacted_turn.content, &expected_content,
            "Anchor at turn {} was modified or dropped!",
            turn_idx
        );
        assert!(
            compacted_turn.role == TurnRole::System || compacted_turn.role == TurnRole::User,
            "Anchor turn role changed!"
        );
    }
}

#[test]
fn test_adversarial_anchor_guard_only_anchors_session() {
    let engine = ContextGcEngine::new();

    // Session consisting entirely of System and User turns
    let turns = vec![
        TrajectoryTurn {
            turn_index: 0,
            role: TurnRole::System,
            content: "System Prompt #1".to_string(),
            tool_name: None,
            tool_call_id: None,
        },
        TrajectoryTurn {
            turn_index: 1,
            role: TurnRole::User,
            content: "User Message #1".to_string(),
            tool_name: None,
            tool_call_id: None,
        },
        TrajectoryTurn {
            turn_index: 2,
            role: TurnRole::User,
            content: "User Follow-up #2 with error[E0308] trace".to_string(),
            tool_name: None,
            tool_call_id: None,
        },
        TrajectoryTurn {
            turn_index: 3,
            role: TurnRole::System,
            content: "System Prompt #2".to_string(),
            tool_name: None,
            tool_call_id: None,
        },
    ];

    let trajectory = SessionTrajectory {
        session_id: "all_anchors".to_string(),
        turns: turns.clone(),
    };

    let options = GcOptions {
        line_threshold: 0,
        hot_window_turns: 0,
        ..Default::default()
    };

    let result = engine.compact(&trajectory, &options).unwrap();
    assert_eq!(result.raw_turns_count, 4);
    assert_eq!(result.compacted_turns_count, 4);
    assert_eq!(result.folded_nodes_count, 0);
    assert_eq!(result.token_reduction_ratio, 0.0);
    assert_eq!(result.compacted_trajectory.turns, turns);
}

// ============================================================================
// 2. FASTMCP `gc_session_context` ADVERSARIAL STRESS SUITE
// ============================================================================

#[test]
fn test_adversarial_mcp_gc_malformed_arguments_and_missing_keys() {
    let temp = tempdir().unwrap();
    let mut server = k0maru::mcp::McpServer::new(temp.path());

    // 1. Completely missing arguments
    let req_missing_args = json!({
        "jsonrpc": "2.0",
        "id": 101,
        "method": "tools/call",
        "params": {
            "name": "gc_session_context",
            "arguments": {}
        }
    });
    let resp_str = server.handle_line(&req_missing_args.to_string()).unwrap();
    let resp: Value = serde_json::from_str(&resp_str).unwrap();
    assert_eq!(resp["id"], 101);
    assert_eq!(resp["result"]["isError"], true);
    assert!(resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Missing required argument: 'trajectory_json'"));

    // 2. Trajectory_json is invalid syntax string
    let bad_syntaxes = vec![
        "",
        "not a valid json object",
        "{ unclosed json",
        "[]",
        "12345",
        "true",
        "{\"session_id\": 123}", // session_id is not string
        "{\"session_id\": \"s1\", \"turns\": \"string\"}", // turns not array
        "{\"session_id\": \"s1\", \"turns\": [{\"role\": \"invalid_role_xyz\"}]}", // invalid role
        "{\"session_id\": \"s1\", \"turns\": [null]}", // null turn in array
    ];

    for (idx, bad_syntax) in bad_syntaxes.into_iter().enumerate() {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 200 + idx,
            "method": "tools/call",
            "params": {
                "name": "gc_session_context",
                "arguments": {
                    "trajectory_json": bad_syntax
                }
            }
        });
        let resp_str = server.handle_line(&req.to_string()).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["id"], 200 + idx);
        assert_eq!(
            resp["result"]["isError"], true,
            "Expected isError: true for malformed trajectory input: '{}'",
            bad_syntax
        );
        assert!(
            resp["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("Failed to parse 'trajectory_json'"),
            "Error message should explain parsing failure"
        );
    }
}

#[test]
fn test_adversarial_mcp_gc_extreme_option_values() {
    let temp = tempdir().unwrap();
    let mut server = k0maru::mcp::McpServer::new(temp.path());

    let trajectory = create_adversarial_trajectory(15, 30);
    let trajectory_json = serde_json::to_string(&trajectory).unwrap();

    // Test with boundary / extreme options:
    // threshold: 0, hot_window: 0, context_window: 0
    let req_zeros = json!({
        "jsonrpc": "2.0",
        "id": 301,
        "method": "tools/call",
        "params": {
            "name": "gc_session_context",
            "arguments": {
                "trajectory_json": trajectory_json,
                "threshold": 0,
                "hot_window": 0,
                "context_window": 0
            }
        }
    });

    let resp_str = server.handle_line(&req_zeros.to_string()).unwrap();
    let resp: Value = serde_json::from_str(&resp_str).unwrap();
    assert_eq!(resp["id"], 301);
    assert!(
        resp["result"].get("isError").is_none() || resp["result"]["isError"] == false,
        "Server should handle zero option values cleanly without panic or error: {:?}",
        resp
    );

    let content_text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: GcResult = serde_json::from_str(content_text).unwrap();
    assert_eq!(parsed.raw_turns_count, 15);

    // Test with huge hot_window exceeding trajectory turn count
    let req_huge_hot = json!({
        "jsonrpc": "2.0",
        "id": 302,
        "method": "tools/call",
        "params": {
            "name": "gc_session_context",
            "arguments": {
                "trajectory_json": trajectory_json,
                "threshold": 20,
                "hot_window": 999999,
                "context_window": 128000
            }
        }
    });

    let resp_huge = server.handle_line(&req_huge_hot.to_string()).unwrap();
    let resp_huge_val: Value = serde_json::from_str(&resp_huge).unwrap();
    assert!(
        resp_huge_val["result"].get("isError").is_none()
            || resp_huge_val["result"]["isError"] == false
    );
    let gc_huge: GcResult = serde_json::from_str(
        resp_huge_val["result"]["content"][0]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    // When hot_window > turns, zero nodes folded
    assert_eq!(gc_huge.folded_nodes_count, 0);
    assert_eq!(gc_huge.token_reduction_ratio, 0.0);
}

#[test]
fn test_adversarial_mcp_gc_direct_object_arguments() {
    let temp = tempdir().unwrap();
    let mut server = k0maru::mcp::McpServer::new(temp.path());

    let trajectory = create_adversarial_trajectory(10, 20);
    let trajectory_val = serde_json::to_value(&trajectory).unwrap();

    // 1. Pass as direct object in `trajectory_json`
    let req_obj = json!({
        "jsonrpc": "2.0",
        "id": 401,
        "method": "tools/call",
        "params": {
            "name": "gc_session_context",
            "arguments": {
                "trajectory_json": trajectory_val.clone()
            }
        }
    });
    let resp1: Value =
        serde_json::from_str(&server.handle_line(&req_obj.to_string()).unwrap()).unwrap();
    assert!(resp1["result"].get("isError").is_none() || resp1["result"]["isError"] == false);

    // 2. Pass as `trajectory` alias
    let req_alias = json!({
        "jsonrpc": "2.0",
        "id": 402,
        "method": "tools/call",
        "params": {
            "name": "gc_session_context",
            "arguments": {
                "trajectory": trajectory_val
            }
        }
    });
    let resp2: Value =
        serde_json::from_str(&server.handle_line(&req_alias.to_string()).unwrap()).unwrap();
    assert!(resp2["result"].get("isError").is_none() || resp2["result"]["isError"] == false);
}

// ============================================================================
// 3. CLI `k0maru gc` ADVERSARIAL STRESS SUITE (MALFORMED & PIPED STREAMS)
// ============================================================================

#[test]
fn test_adversarial_cli_gc_piped_empty_and_whitespace_stdin() {
    // 1. Empty stdin pipe
    let mut cmd_empty = Command::cargo_bin("k0maru").unwrap();
    cmd_empty
        .arg("gc")
        .write_stdin("")
        .assert()
        .failure()
        .code(1);

    // 2. Whitespace-only stdin pipe
    let mut cmd_ws = Command::cargo_bin("k0maru").unwrap();
    cmd_ws
        .arg("gc")
        .write_stdin("   \n\t  \r\n   ")
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_adversarial_cli_gc_piped_malformed_json() {
    let malformed_payloads = vec![
        "not json at all",
        "{ unclosed bracket",
        "{\"session_id\": 123}",
        "{\"session_id\": \"s1\", \"turns\": \"not_array\"}",
        "{\"session_id\": \"s1\", \"turns\": [{\"role\": \"invalid_role\"}]}",
    ];

    for payload in malformed_payloads {
        let mut cmd = Command::cargo_bin("k0maru").unwrap();
        let output = cmd
            .arg("gc")
            .write_stdin(payload)
            .assert()
            .failure()
            .code(1)
            .get_output()
            .stderr
            .clone();

        let stderr = String::from_utf8_lossy(&output);
        assert!(
            stderr.contains("无法解析会话轨迹 JSON"),
            "Expected friendly error message in stderr for payload '{}', got: {}",
            payload,
            stderr
        );
    }
}

#[test]
fn test_adversarial_cli_gc_nonexistent_and_directory_file_args() {
    // 1. Non-existent file path -> exit code 2
    let mut cmd_nonexistent = Command::cargo_bin("k0maru").unwrap();
    cmd_nonexistent
        .args(["gc", "/tmp/definitely_does_not_exist_987654.json"])
        .assert()
        .failure()
        .code(2);

    // 2. Directory path passed as file -> exit code 1 (I/O read error)
    let temp = tempdir().unwrap();
    let mut cmd_dir = Command::cargo_bin("k0maru").unwrap();
    cmd_dir
        .args(["gc", temp.path().to_str().unwrap()])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_adversarial_cli_gc_piped_stream_ndjson_events_sequence() {
    let trajectory = create_adversarial_trajectory(25, 40);
    let json_bytes = serde_json::to_vec(&trajectory).unwrap();

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    let output = cmd
        .args([
            "gc",
            "--json-stream",
            "--threshold",
            "10",
            "--hot-window",
            "3",
        ])
        .write_stdin(json_bytes)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout_str = String::from_utf8_lossy(&output);
    let lines: Vec<&str> = stdout_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    assert!(
        lines.len() >= 3,
        "Expected at least session_start, turn_folded, gc_complete events"
    );

    // First event must be session_start
    let ev_first: Value = serde_json::from_str(lines[0]).expect("Valid NDJSON line");
    assert_eq!(ev_first["event"], "session_start");
    assert_eq!(ev_first["data"]["total_turns"], 25);

    // Last event must be gc_complete
    let ev_last: Value = serde_json::from_str(lines.last().unwrap()).expect("Valid NDJSON line");
    assert_eq!(ev_last["event"], "gc_complete");
    assert!(ev_last["data"]["trr"].as_f64().unwrap() >= 85.0);

    // Intermediate events must be turn_folded
    for line in &lines[1..lines.len() - 1] {
        let ev: Value = serde_json::from_str(line).expect("Valid NDJSON line");
        assert_eq!(ev["event"], "turn_folded");
        assert!(ev["data"]["node_id"].as_str().is_some());
    }
}

// ============================================================================
// 4. LARGE PAYLOAD & SCALABILITY STRESS SUITE (1,000 TURNS & 10MB DUMPS)
// ============================================================================

#[test]
fn test_adversarial_huge_turn_dump_10mb_single_turn() {
    let engine = ContextGcEngine::new();

    // Create a 100,000 line compiler output (~4MB text)
    let huge_content = make_huge_compiler_error(100_000, "E0999");
    let content_len = huge_content.len();
    assert!(content_len > 3_000_000, "Content must exceed 3MB");

    let trajectory = SessionTrajectory {
        session_id: "huge_payload_test".to_string(),
        turns: vec![
            TrajectoryTurn {
                turn_index: 0,
                role: TurnRole::System,
                content: "System Prompt Anchor".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 1,
                role: TurnRole::User,
                content: "User Request Anchor".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 2,
                role: TurnRole::Tool,
                content: huge_content,
                tool_name: Some("cargo".to_string()),
                tool_call_id: Some("call_2".to_string()),
            },
            TrajectoryTurn {
                turn_index: 3,
                role: TurnRole::Assistant,
                content: "Hot window turn 3".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
        ],
    };

    let options = GcOptions {
        line_threshold: 20,
        hot_window_turns: 1,
        ..Default::default()
    };

    let start = Instant::now();
    let result = engine
        .compact(&trajectory, &options)
        .expect("Huge payload compaction must succeed");
    let duration = start.elapsed();

    // Verify it folded turn 2 into a compact reference
    assert_eq!(result.folded_nodes_count, 1);
    assert!(result.compacted_trajectory.turns[2]
        .content
        .starts_with("[Offloaded Tool Output: NodeID node_"));

    // Verify TRR is extremely high (>99%)
    assert!(
        result.token_reduction_ratio > 99.0,
        "TRR should exceed 99% for massive tool offload, got {:.2}%",
        result.token_reduction_ratio
    );

    // Verify Anchors remained intact
    assert_eq!(
        result.compacted_trajectory.turns[0].content,
        "System Prompt Anchor"
    );
    assert_eq!(
        result.compacted_trajectory.turns[1].content,
        "User Request Anchor"
    );

    // Verify execution completed efficiently
    assert!(
        duration.as_millis() < 2000,
        "Compaction of 100k line turn took too long: {:?}",
        duration
    );
}

#[test]
fn test_adversarial_1000_turn_trajectory_stress_and_trr() {
    let engine = ContextGcEngine::new();

    // Create a 1,000-turn trajectory with 50 lines per tool turn
    let trajectory = create_adversarial_trajectory(1000, 50);
    assert_eq!(trajectory.turns.len(), 1000);

    let options = GcOptions {
        line_threshold: 20,
        hot_window_turns: 5,
        context_window_tokens: 128_000,
        ..Default::default()
    };

    let start = Instant::now();
    let result = engine
        .compact(&trajectory, &options)
        .expect("1,000-turn compaction must succeed");
    let duration = start.elapsed();

    assert_eq!(result.raw_turns_count, 1000);
    assert_eq!(result.compacted_turns_count, 1000);
    assert!(result.folded_nodes_count > 400);

    // Verify Key Metric: TRR >= 85.0%
    assert!(
        result.token_reduction_ratio >= 85.0,
        "Expected TRR >= 85.0%, got {:.2}% (raw: {}, compacted: {})",
        result.token_reduction_ratio,
        result.raw_tokens,
        result.compacted_tokens
    );

    // Verify Anchors strictly preserved
    assert_eq!(
        result.compacted_trajectory.turns[0].content,
        "SYSTEM_ANCHOR: You are an autonomous agent under strict invariants."
    );
    assert_eq!(
        result.compacted_trajectory.turns[1].content,
        "USER_ANCHOR: Please eliminate all compilation bottlenecks."
    );

    // Verify Hot Window (last 5 turns) preserved untouched
    for i in 995..1000 {
        assert_eq!(
            result.compacted_trajectory.turns[i].content, trajectory.turns[i].content,
            "Turn {} in hot window was modified!",
            i
        );
    }

    // Verify performance: 1,000 turns with unoptimized string lowering takes ~10s
    assert!(
        duration.as_millis() < 15000,
        "1,000 turn compaction took too long: {:?}",
        duration
    );
}

#[test]
fn test_adversarial_token_estimation_boundaries_and_unicode() {
    // 1. Empty string
    assert_eq!(estimate_tokens(""), 0);

    // 2. Whitespace only
    assert_eq!(estimate_tokens("   \n\t\r\n   "), 0);

    // 3. ASCII English text
    let text = "Hello world from k0maru memory hub!";
    let tokens = estimate_tokens(text);
    assert!(tokens > 0 && tokens < 20);

    // 4. Mixed CJK text
    let mixed = "k0maru-agent-memory 架构设计与长轨迹会话上下文垃圾回收";
    let mixed_tokens = estimate_tokens(mixed);
    assert!(mixed_tokens > 0);

    // 5. Special Unicode, Emojis, math symbols
    let unicode = "🦀 🧹 🚀 ∑_{i=1}^N f(x_i) ∈ ℝ^{d} \u{1F600}";
    let u_tokens = estimate_tokens(unicode);
    assert!(u_tokens > 0);
}
