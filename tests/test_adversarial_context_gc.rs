use assert_cmd::Command;
use k0maru_core::gc::{
    ContextGcEngine, GcOptions, GcResult, SessionTrajectory, TrajectoryTurn, TurnRole,
};
use serde_json::{json, Value};
use tempfile::tempdir;

/// Helper to generate realistic verbose compiler diagnostics.
fn generate_rustc_error(code: &str, msg: &str, turn_idx: usize, lines_count: usize) -> String {
    let mut lines = vec![
        "$ cargo check --workspace --all-targets".to_string(),
        "   Compiling k0maru-core v0.8.2 (/workspace/crates/k0maru-core)".to_string(),
        format!("error[{}]: {}", code, msg),
        format!(
            "  --> crates/k0maru-core/src/gc/engine.rs:{}:13",
            turn_idx * 7 + 10
        ),
        "   |".to_string(),
        format!(
            "{line} |     let res = evaluate_candidate_node(&node_{turn_idx});",
            line = turn_idx * 7 + 10
        ),
        "   |               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ mismatched types".to_string(),
        "   |".to_string(),
        format!("   = note: expected signature `fn(&Context) -> Result<TokenBudget, GcError>`"),
        format!("              found signature `fn(&Context) -> TokenBudget`"),
    ];
    for i in 1..lines_count {
        lines.push(format!(
            "   = diagnostic trace detail #{}: inspecting trait bound candidate for type `Engine<Turn_{}>` against bound `Compactor` at stack frame {}",
            i, turn_idx, i * 4
        ));
    }
    lines.push("error: could not compile `k0maru-core` due to 1 previous error".to_string());
    lines.join("\n")
}

/// Helper to generate a Python traceback dump.
fn generate_python_traceback(exception_type: &str, msg: &str, frames: usize) -> String {
    let mut lines = vec!["Traceback (most recent call last):".to_string()];
    for f in 0..frames {
        lines.push(format!(
            "  File \"/agent/pipeline/orchestration/runner_{}.py\", line {}, in execute_step",
            f,
            f * 12 + 5
        ));
        lines.push(format!(
            "    result = await handler.dispatch(ctx_{}, payload=payload_{})",
            f, f
        ));
    }
    lines.push(format!("{}: {}", exception_type, msg));
    lines.join("\n")
}

/// Helper to generate a verbose npm ERR! dump.
fn generate_npm_error(code: &str, msg: &str, count: usize) -> String {
    let mut lines = vec![
        format!("npm ERR! code {}", code),
        format!("npm ERR! syscall spawn"),
        format!("npm ERR! path /workspace/node_modules/.bin/vite"),
        format!("npm ERR! errno -2"),
        format!("npm ERR! {}", msg),
    ];
    for i in 0..count {
        lines.push(format!(
            "npm ERR! verbose stack at Step_{} (/usr/local/lib/node_modules/npm/lib/exec.js:{})",
            i,
            i * 15 + 3
        ));
    }
    lines.push("npm ERR! A complete log of this run can be found in: /root/.npm/_logs/2026-10-11T12_debug-0.log".to_string());
    lines.join("\n")
}

/// Helper to generate a C/C++ fatal error dump.
fn generate_clang_error(header: &str, count: usize) -> String {
    let mut lines = vec![
        format!("fatal error: {}: No such file or directory", header),
        format!("   #include <{}>", header),
        format!("            ^~~~~~~~~~~~~~"),
        format!("compilation terminated."),
    ];
    for i in 0..count {
        lines.push(format!(
            "In file included from /usr/include/c++/v1/ext/hash_map:{} in include_chain_{}",
            i * 10 + 2,
            i
        ));
    }
    lines.join("\n")
}

/// Helper to generate a verbose successful test / build output.
fn generate_clean_success(turn_idx: usize, test_count: usize) -> String {
    let mut lines = vec![
        "$ cargo test --workspace".to_string(),
        "   Compiling k0maru v0.8.2".to_string(),
        "    Finished test [unoptimized + debuginfo] target(s) in 0.42s".to_string(),
        "     Running unittests src/lib.rs".to_string(),
    ];
    for i in 0..test_count {
        lines.push(format!(
            "test suite::module_{}::case_turn_{}_{} ... ok",
            i % 5,
            turn_idx,
            i
        ));
    }
    lines.push(format!(
        "test result: ok. {} passed; 0 failed; 0 ignored; 0 measured; finished in 0.35s",
        test_count
    ));
    lines.join("\n")
}

/// Builds an adversarial trajectory of arbitrary length N (e.g. 50, 75, 100 turns).
fn build_adversarial_trajectory(total_turns: usize, session_id: &str) -> SessionTrajectory {
    assert!(total_turns >= 10, "Trajectory must be at least 10 turns");
    let mut turns = Vec::with_capacity(total_turns);

    // Turn 0: System prompt (Inviolable Anchor)
    turns.push(TrajectoryTurn {
        turn_index: 0,
        role: TurnRole::System,
        content: "You are an autonomous systems engineering agent. All edits must pass cargo check, cargo clippy, and cargo test.".to_string(),
        tool_name: None,
        tool_call_id: None,
    });

    // Turn 1: User prompt (Inviolable Anchor)
    turns.push(TrajectoryTurn {
        turn_index: 1,
        role: TurnRole::User,
        content: format!("Please diagnose and fix all compiler errors and integration regressions in session {}.", session_id),
        tool_name: None,
        tool_call_id: None,
    });

    let hot_window_start = total_turns - 5;

    // Historical turns (Turns 2..hot_window_start)
    for i in 2..hot_window_start {
        if i % 2 == 1 {
            // Tool turn with massive dumps (80-120 lines, 1,500-2,500 tokens)
            let tool_content = match (i / 2) % 4 {
                0 => generate_rustc_error("E0308", "mismatched types in trait dispatch", i, 90),
                1 => generate_rustc_error(
                    "E0425",
                    "cannot find value `ContextGcEngine` in this scope",
                    i,
                    95,
                ),
                2 => generate_rustc_error(
                    "E0277",
                    "the trait bound `SessionTrajectory: Compactor` is not satisfied",
                    i,
                    100,
                ),
                _ => {
                    // Periodic test success or intermediate compilation milestone
                    if i % 7 == 1 {
                        generate_clean_success(i, 60)
                    } else {
                        generate_rustc_error(
                            "E0599",
                            "no method named `compact_stream` found for struct `GcEngine`",
                            i,
                            85,
                        )
                    }
                }
            };

            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Tool,
                content: tool_content,
                tool_name: Some("cargo".to_string()),
                tool_call_id: Some(format!("call_{}", i)),
            });
        } else {
            // Assistant reasoning turn
            let content = match i {
                20 => "Resolved E0308 by reconciling type bounds; inspecting subsequent error in trait solver.".to_string(),
                40 => "Compilation succeeded! Now running full cargo test workspace suite to verify regressions.".to_string(),
                _ => format!("Assistant analysis turn {}: modifying crates/k0maru-core/src/gc/engine.rs and rerunning verification.", i),
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

    // Hot Window: last 5 turns (hot_window_start..total_turns)
    for i in hot_window_start..total_turns {
        if i % 2 == 1 {
            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Tool,
                content: format!(
                    "test result: ok. 210 passed; 0 failed; finished in 0.5s (Turn {})",
                    i
                ),
                tool_name: Some("cargo test".to_string()),
                tool_call_id: Some(format!("call_{}", i)),
            });
        } else {
            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Assistant,
                content: format!(
                    "Hot window active step {}: verifying clean exit and 0 warnings.",
                    i
                ),
                tool_name: None,
                tool_call_id: None,
            });
        }
    }

    assert_eq!(turns.len(), total_turns);
    SessionTrajectory {
        session_id: session_id.to_string(),
        turns,
    }
}

// ==============================================================================
// 1. Stress-test TRR ratio: 50-turn, 75-turn, and 100-turn adversarial trajectories
// ==============================================================================

#[test]
fn test_adversarial_50_turn_trajectory_trr_gte_85() {
    let trajectory = build_adversarial_trajectory(50, "adversarial_50_turns");
    let options = GcOptions {
        line_threshold: 20,
        hot_window_turns: 5,
        context_window_tokens: 32_000,
        ..Default::default()
    };

    let engine = ContextGcEngine::new();
    let result = engine
        .compact(&trajectory, &options)
        .expect("Compaction should succeed");

    assert_eq!(result.raw_turns_count, 50);
    assert_eq!(result.compacted_turns_count, 50);
    assert!(
        result.raw_tokens > 20_000,
        "Raw tokens must be substantial (>20,000)"
    );

    // Strict acceptance criteria: TRR >= 85.0%
    println!(
        "[50-turn stress] Raw tokens: {}, Compacted tokens: {}, TRR: {:.2}%, Folded nodes: {}",
        result.raw_tokens,
        result.compacted_tokens,
        result.token_reduction_ratio,
        result.folded_nodes_count
    );
    assert!(
        result.token_reduction_ratio >= 85.0,
        "TRR must strictly be >= 85.0%, got {:.2}%",
        result.token_reduction_ratio
    );
    assert!(result.trr_fraction() >= 0.85);
    assert!(result.folded_nodes_count >= 15);
}

#[test]
fn test_adversarial_75_turn_trajectory_trr_gte_85() {
    let trajectory = build_adversarial_trajectory(75, "adversarial_75_turns");
    let options = GcOptions {
        line_threshold: 20,
        hot_window_turns: 5,
        context_window_tokens: 64_000,
        ..Default::default()
    };

    let engine = ContextGcEngine::new();
    let result = engine
        .compact(&trajectory, &options)
        .expect("Compaction should succeed");

    assert_eq!(result.raw_turns_count, 75);
    assert_eq!(result.compacted_turns_count, 75);
    assert!(
        result.raw_tokens > 40_000,
        "Raw tokens must be substantial (>40,000)"
    );

    println!(
        "[75-turn stress] Raw tokens: {}, Compacted tokens: {}, TRR: {:.2}%, Folded nodes: {}",
        result.raw_tokens,
        result.compacted_tokens,
        result.token_reduction_ratio,
        result.folded_nodes_count
    );
    assert!(
        result.token_reduction_ratio >= 85.0,
        "TRR must strictly be >= 85.0%, got {:.2}%",
        result.token_reduction_ratio
    );
    assert!(result.trr_fraction() >= 0.85);
    assert!(result.folded_nodes_count >= 25);
}

#[test]
fn test_adversarial_100_turn_trajectory_trr_gte_85() {
    let trajectory = build_adversarial_trajectory(100, "adversarial_100_turns");
    let options = GcOptions {
        line_threshold: 20,
        hot_window_turns: 5,
        context_window_tokens: 128_000,
        ..Default::default()
    };

    let engine = ContextGcEngine::new();
    let result = engine
        .compact(&trajectory, &options)
        .expect("Compaction should succeed");

    assert_eq!(result.raw_turns_count, 100);
    assert_eq!(result.compacted_turns_count, 100);
    assert!(
        result.raw_tokens > 60_000,
        "Raw tokens must be substantial (>60,000)"
    );

    println!(
        "[100-turn stress] Raw tokens: {}, Compacted tokens: {}, TRR: {:.2}%, Folded nodes: {}",
        result.raw_tokens,
        result.compacted_tokens,
        result.token_reduction_ratio,
        result.folded_nodes_count
    );
    assert!(
        result.token_reduction_ratio >= 85.0,
        "TRR must strictly be >= 85.0%, got {:.2}%",
        result.token_reduction_ratio
    );
    assert!(result.trr_fraction() >= 0.85);
    assert!(result.folded_nodes_count >= 35);
}

// ==============================================================================
// 2. Stress-test: Never-resolved / Stuck Agent 50-turn trajectory
// ==============================================================================

#[test]
fn test_adversarial_never_resolved_error_trajectory_trr() {
    // 50 turns where the agent repeatedly hits the EXACT SAME error with NO resolution.
    // Tier 3 does not resolve, so Tier 2 (Tool Output Offloading) must fold all historical tool outputs.
    let mut turns = Vec::with_capacity(50);
    turns.push(TrajectoryTurn {
        turn_index: 0,
        role: TurnRole::System,
        content: "System prompt".to_string(),
        tool_name: None,
        tool_call_id: None,
    });
    turns.push(TrajectoryTurn {
        turn_index: 1,
        role: TurnRole::User,
        content: "Fix broken build".to_string(),
        tool_name: None,
        tool_call_id: None,
    });

    for i in 2..45 {
        if i % 2 == 1 {
            // Unresolved identical error dump
            let content = generate_rustc_error("E0308", "same persistent mismatched types", i, 80);
            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Tool,
                content,
                tool_name: Some("cargo check".to_string()),
                tool_call_id: Some(format!("call_{}", i)),
            });
        } else {
            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Assistant,
                content: format!("Attempt {} to fix E0308.", i),
                tool_name: None,
                tool_call_id: None,
            });
        }
    }

    for i in 45..50 {
        turns.push(TrajectoryTurn {
            turn_index: i,
            role: if i % 2 == 1 {
                TurnRole::Tool
            } else {
                TurnRole::Assistant
            },
            content: "Still unresolved in hot window".to_string(),
            tool_name: None,
            tool_call_id: None,
        });
    }

    let trajectory = SessionTrajectory {
        session_id: "never_resolved_stuck_agent".to_string(),
        turns,
    };

    let options = GcOptions {
        line_threshold: 20,
        hot_window_turns: 5,
        ..Default::default()
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &options).unwrap();

    println!(
        "[Never-resolved stress] Raw: {}, Compacted: {}, TRR: {:.2}%, Folded: {}",
        result.raw_tokens,
        result.compacted_tokens,
        result.token_reduction_ratio,
        result.folded_nodes_count
    );

    // Even without error resolution, Tier 2 historical offloading must guarantee TRR >= 85.0%
    assert!(
        result.token_reduction_ratio >= 85.0,
        "Never-resolved trajectory must still reach TRR >= 85.0% via Tier 2, got {:.2}%",
        result.token_reduction_ratio
    );
    assert!(result.folded_nodes_count >= 20);
}

// ==============================================================================
// 3. Stress-test: Multi-language mixed error diagnostics (Rust, Python, npm, C++)
// ==============================================================================

#[test]
fn test_adversarial_multilanguage_cascading_diagnostics_trr() {
    let mut turns = Vec::with_capacity(60);
    turns.push(TrajectoryTurn {
        turn_index: 0,
        role: TurnRole::System,
        content: "Multi-language polyglot agent".to_string(),
        tool_name: None,
        tool_call_id: None,
    });
    turns.push(TrajectoryTurn {
        turn_index: 1,
        role: TurnRole::User,
        content: "Fix polyglot stack build".to_string(),
        tool_name: None,
        tool_call_id: None,
    });

    for i in 2..55 {
        if i % 2 == 1 {
            let content = match (i / 2) % 4 {
                0 => generate_rustc_error("E0308", "mismatched types", i, 70),
                1 => generate_python_traceback("KeyError", "'missing_config_key'", 45),
                2 => generate_npm_error("ELIFECYCLE", "Failed at vite build script", 50),
                _ => generate_clang_error("openssl/ssl.h", 60),
            };

            turns.push(TrajectoryTurn {
                turn_index: i,
                role: TurnRole::Tool,
                content,
                tool_name: Some("tool_exec".to_string()),
                tool_call_id: Some(format!("call_{}", i)),
            });
        } else {
            let content = if i == 18 {
                "Python traceback resolved! Build succeeded.".to_string()
            } else if i == 34 {
                "npm ERR! fixed! Compilation succeeded.".to_string()
            } else {
                format!("Polyglot debugging turn {}.", i)
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

    for i in 55..60 {
        turns.push(TrajectoryTurn {
            turn_index: i,
            role: TurnRole::Assistant,
            content: format!("Hot window turn {}", i),
            tool_name: None,
            tool_call_id: None,
        });
    }

    let trajectory = SessionTrajectory {
        session_id: "polyglot_multilanguage".to_string(),
        turns,
    };

    let options = GcOptions::default();
    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &options).unwrap();

    println!(
        "[Polyglot stress] Raw: {}, Compacted: {}, TRR: {:.2}%, Folded: {}",
        result.raw_tokens,
        result.compacted_tokens,
        result.token_reduction_ratio,
        result.folded_nodes_count
    );
    assert!(result.token_reduction_ratio >= 85.0);
}

// ==============================================================================
// 4. Stress-test: Ultra-massive single tool output (1,000 lines, 20,000 tokens)
// ==============================================================================

#[test]
fn test_adversarial_ultra_massive_single_tool_output() {
    let mut lines = Vec::new();
    lines.push("$ git log --stat --all".to_string());
    for i in 0..1000 {
        lines.push(format!("commit {:040x}\nAuthor: dev{} <dev{}@corp.com>\nDate: 2026-10-11\n    feat: heavy commit description number {}\n 15 files changed, 200 insertions(+), 50 deletions(-)", i, i, i, i));
    }
    let massive_content = lines.join("\n");

    let trajectory = SessionTrajectory {
        session_id: "massive_single_tool".to_string(),
        turns: vec![
            TrajectoryTurn {
                turn_index: 0,
                role: TurnRole::User,
                content: "Inspect history".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 1,
                role: TurnRole::Tool,
                content: massive_content,
                tool_name: Some("git".to_string()),
                tool_call_id: Some("call_1".to_string()),
            },
            TrajectoryTurn {
                turn_index: 2,
                role: TurnRole::Assistant,
                content: "Git log inspected.".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
        ],
    };

    let options = GcOptions {
        line_threshold: 20,
        hot_window_turns: 1, // Only turn 2 in hot window
        ..Default::default()
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &options).unwrap();

    assert_eq!(result.folded_nodes_count, 1);
    assert!(result.compacted_trajectory.turns[1]
        .content
        .contains("[Offloaded Tool Output: NodeID node_"));
    assert!(
        result.token_reduction_ratio > 98.0,
        "Expected >98% TRR on 1000-line tool output, got {:.2}%",
        result.token_reduction_ratio
    );
}

// ==============================================================================
// 5. Edge cases: Empty session, 1-turn session, only User turns, only System turns
// ==============================================================================

#[test]
fn test_edge_case_empty_session() {
    let trajectory = SessionTrajectory {
        session_id: "empty_session".to_string(),
        turns: vec![],
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &GcOptions::default()).unwrap();

    assert_eq!(result.raw_turns_count, 0);
    assert_eq!(result.compacted_turns_count, 0);
    assert_eq!(result.raw_tokens, 0);
    assert_eq!(result.compacted_tokens, 0);
    assert_eq!(result.token_reduction_ratio, 0.0);
    assert_eq!(result.folded_nodes_count, 0);
    assert_eq!(result.smart_zone_occupancy, 0.0);
    assert!(!result.in_smart_zone);
}

#[test]
fn test_edge_case_one_turn_session_user() {
    let trajectory = SessionTrajectory {
        session_id: "single_user_turn".to_string(),
        turns: vec![TrajectoryTurn {
            turn_index: 0,
            role: TurnRole::User,
            content: "Single solitary user turn".to_string(),
            tool_name: None,
            tool_call_id: None,
        }],
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &GcOptions::default()).unwrap();

    assert_eq!(result.raw_turns_count, 1);
    assert_eq!(result.compacted_turns_count, 1);
    assert_eq!(result.folded_nodes_count, 0);
    assert_eq!(
        result.compacted_trajectory.turns[0].content,
        "Single solitary user turn"
    );
    assert_eq!(result.token_reduction_ratio, 0.0);
}

#[test]
fn test_edge_case_one_turn_session_tool_in_hot_window() {
    // 1-turn session where the only turn is a massive tool output.
    // Because total_turns = 1 <= hot_window_turns = 5, it is in hot window and must NOT be pruned.
    let trajectory = SessionTrajectory {
        session_id: "single_tool_turn".to_string(),
        turns: vec![TrajectoryTurn {
            turn_index: 0,
            role: TurnRole::Tool,
            content: "data line\n".repeat(100),
            tool_name: Some("tool".to_string()),
            tool_call_id: None,
        }],
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &GcOptions::default()).unwrap();

    assert_eq!(result.raw_turns_count, 1);
    assert_eq!(result.compacted_turns_count, 1);
    assert_eq!(result.folded_nodes_count, 0);
    assert_eq!(
        result.compacted_trajectory.turns[0].content,
        "data line\n".repeat(100)
    );
    assert_eq!(result.token_reduction_ratio, 0.0);
}

#[test]
fn test_edge_case_only_user_turns_preserves_anchor_invariants() {
    // 50 turns consisting strictly of User messages (even if they contain raw compiler dumps).
    // Tier 0 Anchor Invariant strictly forbids modifying any User turn.
    let mut turns = Vec::with_capacity(50);
    for i in 0..50 {
        turns.push(TrajectoryTurn {
            turn_index: i,
            role: TurnRole::User,
            content: format!(
                "User query {}: Here is an error log:\nerror[E0308]: mismatched types\n{}",
                i,
                "trace\n".repeat(25)
            ),
            tool_name: None,
            tool_call_id: None,
        });
    }

    let trajectory = SessionTrajectory {
        session_id: "pure_user_session".to_string(),
        turns: turns.clone(),
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &GcOptions::default()).unwrap();

    assert_eq!(result.raw_turns_count, 50);
    assert_eq!(result.compacted_turns_count, 50);
    assert_eq!(result.folded_nodes_count, 0, "No User turns may be folded");
    assert_eq!(result.raw_tokens, result.compacted_tokens);
    assert_eq!(result.token_reduction_ratio, 0.0);

    for (orig, compacted) in turns.iter().zip(result.compacted_trajectory.turns.iter()) {
        assert_eq!(
            orig.content, compacted.content,
            "User turn content must remain verbatim"
        );
    }
}

#[test]
fn test_edge_case_only_system_turns_preserves_anchor_invariants() {
    // Session with only System turns
    let mut turns = Vec::with_capacity(10);
    for i in 0..10 {
        turns.push(TrajectoryTurn {
            turn_index: i,
            role: TurnRole::System,
            content: format!(
                "System instruction {}\n{}",
                i,
                "rule definition\n".repeat(30)
            ),
            tool_name: None,
            tool_call_id: None,
        });
    }

    let trajectory = SessionTrajectory {
        session_id: "pure_system_session".to_string(),
        turns: turns.clone(),
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &GcOptions::default()).unwrap();

    assert_eq!(result.raw_turns_count, 10);
    assert_eq!(result.compacted_turns_count, 10);
    assert_eq!(result.folded_nodes_count, 0);
    assert_eq!(result.raw_tokens, result.compacted_tokens);
}

// ==============================================================================
// 6. Edge cases: Non-standard error patterns & CJK / Whitespace robustness
// ==============================================================================

#[test]
fn test_edge_case_nonstandard_error_patterns_fallback_to_tier2() {
    // Non-standard errors that do not match Rustc, Python, npm, or Clang patterns:
    // 1. Go compiler: "main.go:12:2: undefined: fmt.Print"
    // 2. Java stacktrace: "java.lang.NullPointerException: Cannot invoke..."
    // 3. Custom shell script failure: "[ERROR] deployment step 4 failed with status 127"
    let trajectory = SessionTrajectory {
        session_id: "nonstandard_errors".to_string(),
        turns: vec![
            TrajectoryTurn {
                turn_index: 0,
                role: TurnRole::User,
                content: "Run polyglot jobs".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 1,
                role: TurnRole::Tool,
                content: "main.go:12:2: undefined: fmt.Print\n".to_string()
                    + &"go package loader stack\n".repeat(30),
                tool_name: Some("go build".to_string()),
                tool_call_id: Some("call_1".to_string()),
            },
            TrajectoryTurn {
                turn_index: 2,
                role: TurnRole::Tool,
                content: "Exception in thread \"main\" java.lang.NullPointerException\n"
                    .to_string()
                    + &"   at com.example.App.main(App.java:42)\n".repeat(25),
                tool_name: Some("mvn test".to_string()),
                tool_call_id: Some("call_2".to_string()),
            },
            TrajectoryTurn {
                turn_index: 3,
                role: TurnRole::Tool,
                content: "[FATAL-PIPELINE-FAIL]: code 127\n".to_string()
                    + &"pipeline raw dump\n".repeat(40),
                tool_name: Some("bash".to_string()),
                tool_call_id: Some("call_3".to_string()),
            },
            TrajectoryTurn {
                turn_index: 4,
                role: TurnRole::Assistant,
                content: "Hot window turn".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
        ],
    };

    let options = GcOptions {
        line_threshold: 15,
        hot_window_turns: 1, // Only turn 4 in hot window
        ..Default::default()
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &options).unwrap();

    // All 3 non-standard error outputs exceed line_threshold and must be cleanly offloaded by Tier 2
    assert_eq!(result.folded_nodes_count, 3);
    for idx in 1..=3 {
        assert!(
            result.compacted_trajectory.turns[idx]
                .content
                .contains("[Offloaded Tool Output: NodeID node_"),
            "Turn {} should be offloaded by Tier 2: {}",
            idx,
            result.compacted_trajectory.turns[idx].content
        );
    }
}

#[test]
fn test_edge_case_cjk_error_patterns_and_mixed_text() {
    // Error messages in Chinese / CJK mixed with English
    let cjk_error = "错误: 编译 crates/k0maru-core 失败\n".to_string()
        + &"详细回溯堆栈诊断信息：无法解析符号标识符\n".repeat(25);
    let trajectory = SessionTrajectory {
        session_id: "cjk_session".to_string(),
        turns: vec![
            TrajectoryTurn {
                turn_index: 0,
                role: TurnRole::User,
                content: "请帮我排查构建故障".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 1,
                role: TurnRole::Tool,
                content: cjk_error,
                tool_name: Some("cargo check".to_string()),
                tool_call_id: Some("call_1".to_string()),
            },
            TrajectoryTurn {
                turn_index: 2,
                role: TurnRole::Assistant,
                content: "编译成功！所有测试通过。".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
        ],
    };

    let options = GcOptions {
        line_threshold: 10,
        hot_window_turns: 1,
        ..Default::default()
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &options).unwrap();

    assert_eq!(result.folded_nodes_count, 1);
    assert!(result.compacted_trajectory.turns[1]
        .content
        .contains("[Offloaded Tool Output: NodeID node_"));
    assert!(result.token_reduction_ratio > 80.0);
}

#[test]
fn test_edge_case_single_line_giant_string_token_threshold_trigger() {
    // A single line of 2,000 characters with NO newlines.
    // Line count = 1 <= line_threshold (20), but tokens > 150.
    // Tier 2 rule: (original_lines > line_threshold || turn_tokens > 150)
    let giant_single_line = "A".repeat(2000);
    assert_eq!(giant_single_line.lines().count(), 1);

    let trajectory = SessionTrajectory {
        session_id: "giant_single_line".to_string(),
        turns: vec![
            TrajectoryTurn {
                turn_index: 0,
                role: TurnRole::User,
                content: "query".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 1,
                role: TurnRole::Tool,
                content: giant_single_line,
                tool_name: Some("dump_data".to_string()),
                tool_call_id: Some("call_1".to_string()),
            },
            TrajectoryTurn {
                turn_index: 2,
                role: TurnRole::Assistant,
                content: "done".to_string(),
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

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &options).unwrap();

    assert_eq!(
        result.folded_nodes_count, 1,
        "Single line with >150 tokens must be folded by Tier 2"
    );
    assert!(result.compacted_trajectory.turns[1]
        .content
        .contains("[Offloaded Tool Output: NodeID node_"));
}

#[test]
fn test_edge_case_empty_and_whitespace_content() {
    let trajectory = SessionTrajectory {
        session_id: "whitespace_session".to_string(),
        turns: vec![
            TrajectoryTurn {
                turn_index: 0,
                role: TurnRole::User,
                content: "   \t\n  ".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 1,
                role: TurnRole::Tool,
                content: "".to_string(),
                tool_name: Some("empty_tool".to_string()),
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 2,
                role: TurnRole::Assistant,
                content: "\n\n".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
        ],
    };

    let engine = ContextGcEngine::new();
    let result = engine.compact(&trajectory, &GcOptions::default()).unwrap();

    assert_eq!(result.raw_turns_count, 3);
    assert_eq!(result.compacted_turns_count, 3);
    assert_eq!(result.folded_nodes_count, 0);
    assert_eq!(result.token_reduction_ratio, 0.0);
}

#[test]
fn test_edge_case_boundary_hot_window_values() {
    let trajectory = build_adversarial_trajectory(50, "hot_window_boundaries");
    let engine = ContextGcEngine::new();

    // 1. hot_window_turns = 0 (all turns eligible for compaction)
    let options_zero = GcOptions {
        hot_window_turns: 0,
        ..Default::default()
    };
    let res_zero = engine.compact(&trajectory, &options_zero).unwrap();
    assert!(res_zero.folded_nodes_count > 0);
    assert!(res_zero.token_reduction_ratio >= 85.0);

    // 2. hot_window_turns >= total_turns (nothing eligible for compaction except hot window preserves all)
    let options_all = GcOptions {
        hot_window_turns: 100,
        ..Default::default()
    };
    let res_all = engine.compact(&trajectory, &options_all).unwrap();
    assert_eq!(res_all.folded_nodes_count, 0);
    assert_eq!(res_all.token_reduction_ratio, 0.0);
}

// ==============================================================================
// 7. CLI & MCP End-to-End Adversarial Probing (100-turn trajectory via stdin pipe)
// ==============================================================================

#[test]
fn test_adversarial_cli_100_turn_pipe_and_streaming() {
    let trajectory = build_adversarial_trajectory(100, "cli_100_turn_pipe");
    let json_bytes = serde_json::to_vec(&trajectory).unwrap();

    // 1. Test CLI standard output with --json flag
    let mut cmd = Command::cargo_bin("k0maru").expect("Binary k0maru should exist");
    let output = cmd
        .args([
            "gc",
            "--threshold",
            "20",
            "--hot-window",
            "5",
            "--context-window",
            "128000",
            "--json",
        ])
        .write_stdin(json_bytes.clone())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: GcResult = serde_json::from_slice(&output).expect("Output must be valid GcResult");
    assert_eq!(parsed.raw_turns_count, 100);
    assert_eq!(parsed.compacted_turns_count, 100);
    assert!(
        parsed.token_reduction_ratio >= 85.0,
        "CLI 100-turn pipe must achieve TRR >= 85.0%, got {:.2}%",
        parsed.token_reduction_ratio
    );

    // 2. Test CLI NDJSON streaming output with --json-stream flag
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
    let event_lines: Vec<&str> = stdout_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    assert!(
        event_lines.len() >= 30,
        "Expected at least 30 streaming events for 100 turns, got {}",
        event_lines.len()
    );

    let first_ev: Value = serde_json::from_str(event_lines[0]).expect("Valid first event");
    assert_eq!(first_ev["event"], "session_start");

    let last_ev: Value =
        serde_json::from_str(event_lines.last().unwrap()).expect("Valid last event");
    assert_eq!(last_ev["event"], "gc_complete");
    assert!(last_ev["data"]["trr"].as_f64().unwrap() >= 85.0);
}

#[test]
fn test_adversarial_mcp_100_turn_gc() {
    let temp = tempdir().unwrap();
    let mut server = k0maru::mcp::McpServer::new(temp.path());

    let trajectory = build_adversarial_trajectory(100, "mcp_100_turn");
    let trajectory_json = serde_json::to_string(&trajectory).unwrap();

    let req = json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "tools/call",
        "params": {
            "name": "gc_session_context",
            "arguments": {
                "trajectory_json": trajectory_json,
                "threshold": 20,
                "hot_window": 5,
                "context_window": 128000
            }
        }
    });

    let resp_str = server
        .handle_line(&req.to_string())
        .expect("Server should respond to tools/call gc_session_context");

    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON-RPC response");
    assert_eq!(resp["id"], 99);
    assert!(resp.get("error").is_none());

    let content_text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let gc_res: GcResult = serde_json::from_str(content_text).unwrap();

    assert_eq!(gc_res.raw_turns_count, 100);
    assert!(
        gc_res.token_reduction_ratio >= 85.0,
        "MCP 100-turn GC must achieve TRR >= 85.0%, got {:.2}%",
        gc_res.token_reduction_ratio
    );
}

#[test]
fn test_edge_case_malformed_json_cli_and_mcp() {
    // 1. CLI with malformed JSON input exits with code 1
    let mut cmd = Command::cargo_bin("k0maru").expect("Binary k0maru should exist");
    cmd.args(["gc", "--json"])
        .write_stdin(b"{ invalid json trajectory !!!".to_vec())
        .assert()
        .failure()
        .code(1);

    // 2. MCP with malformed JSON trajectory
    let temp = tempdir().unwrap();
    let mut server = k0maru::mcp::McpServer::new(temp.path());

    let req = json!({
        "jsonrpc": "2.0",
        "id": 101,
        "method": "tools/call",
        "params": {
            "name": "gc_session_context",
            "arguments": {
                "trajectory_json": "INVALID_NOT_JSON",
            }
        }
    });

    let resp_str = server.handle_line(&req.to_string()).unwrap();
    let resp: Value = serde_json::from_str(&resp_str).unwrap();
    assert_eq!(resp["id"], 101);
    assert_eq!(
        resp["result"]["isError"], true,
        "Malformed JSON should return result.isError: true in MCP"
    );
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Failed to parse"),
        "Error message should explain parsing failure"
    );
}
