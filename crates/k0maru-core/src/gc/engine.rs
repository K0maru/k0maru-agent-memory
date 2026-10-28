use std::fmt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use super::models::{GcEvent, GcOptions, GcResult, SessionTrajectory, TrajectoryTurn, TurnRole};

/// Error encountered during Context GC compaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GcError {
    InvalidTrajectory(String),
    IoError(String),
}

impl fmt::Display for GcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GcError::InvalidTrajectory(msg) => write!(f, "Invalid trajectory: {}", msg),
            GcError::IoError(msg) => write!(f, "I/O error during GC: {}", msg),
        }
    }
}

impl std::error::Error for GcError {}

/// Estimates token count for mixed English and CJK text.
pub fn estimate_tokens(text: &str) -> usize {
    let mut cjk_count: usize = 0;
    let mut non_cjk_chars: usize = 0;
    for c in text.chars() {
        if ('\u{4e00}'..='\u{9fff}').contains(&c)
            || ('\u{3400}'..='\u{4dbf}').contains(&c)
            || ('\u{f900}'..='\u{faff}').contains(&c)
        {
            cjk_count += 1;
        } else if !c.is_whitespace() {
            non_cjk_chars += 1;
        }
    }
    non_cjk_chars.div_ceil(4) + (cjk_count * 2).div_ceil(3)
}

/// Extracted compiler or runtime error diagnostic signature.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DiagnosticSignature {
    summary: String,
    code: Option<String>,
}

/// Long-Trajectory Context Garbage Collection (Context GC) engine.
///
/// Implements a 4-tier session compaction protocol:
/// - Tier 0: Inviolable Anchor Guard (never prune System or User turns).
/// - Tier 1: Hot Window (preserve the latest $K=5$ turns untouched).
/// - Tier 2: Historical Tool Output Offloading (lines > 20 or tokens > 150 folded to symbolic refs).
/// - Tier 3: Repetitive Compiler Error Deduplication (superseded diagnostics folded into resolution citations).
#[derive(Debug, Default, Clone)]
pub struct ContextGcEngine;

impl ContextGcEngine {
    /// Creates a new ContextGcEngine instance.
    pub fn new() -> Self {
        Self
    }

    /// Compacts a session trajectory according to the 4-tier pruning protocol.
    pub fn compact(
        &self,
        trajectory: &SessionTrajectory,
        options: &GcOptions,
    ) -> Result<GcResult, GcError> {
        let (result, _) = self.compact_with_events(trajectory, options)?;
        Ok(result)
    }

    /// Compacts a session trajectory and returns streaming `GcEvent` notifications.
    pub fn compact_with_events(
        &self,
        trajectory: &SessionTrajectory,
        options: &GcOptions,
    ) -> Result<(GcResult, Vec<GcEvent>), GcError> {
        let total_turns = trajectory.turns.len();
        let raw_tokens: usize = trajectory
            .turns
            .iter()
            .map(|t| estimate_tokens(&t.content))
            .sum();

        let mut events = Vec::new();
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        events.push(GcEvent {
            event: "session_start".to_string(),
            timestamp: now_ms,
            data: json!({
                "session_id": trajectory.session_id,
                "total_turns": total_turns,
                "raw_tokens": raw_tokens,
            }),
        });

        // Identify historical turn boundary
        let hot_window_start = total_turns.saturating_sub(options.hot_window_turns);

        // Pre-scan all turns to build error signatures and find resolutions
        let diagnostics: Vec<Option<DiagnosticSignature>> = trajectory
            .turns
            .iter()
            .map(|t| Self::extract_diagnostic(&t.content))
            .collect();

        let is_success_list: Vec<bool> = trajectory
            .turns
            .iter()
            .map(|t| Self::indicates_success(&t.content))
            .collect();

        // Map turn index -> resolved turn index (if superseded / resolved)
        let mut resolved_map: Vec<Option<usize>> = vec![None; total_turns];
        for i in 0..hot_window_start {
            if let Some(ref diag) = diagnostics[i] {
                // Search subsequent turns for resolution
                for (j, subsequent_turn) in trajectory.turns.iter().enumerate().skip(i + 1) {
                    if is_success_list[j] {
                        // Success in turn j resolves error in turn i
                        resolved_map[i] = Some(subsequent_turn.turn_index);
                        break;
                    } else if let Some(ref sub_diag) = diagnostics[j] {
                        // If subsequent compiler run has a different diagnostic, the previous error was resolved
                        if sub_diag != diag {
                            resolved_map[i] = Some(subsequent_turn.turn_index);
                            break;
                        }
                    }
                }
            }
        }

        let mut compacted_turns = Vec::with_capacity(total_turns);
        let mut folded_nodes_count = 0;

        for (idx, turn) in trajectory.turns.iter().enumerate() {
            // Tier 0: Inviolable Anchor Guard (never prune System or User)
            if turn.role == TurnRole::System || turn.role == TurnRole::User {
                compacted_turns.push(turn.clone());
                continue;
            }

            // Tier 1: Hot Window (preserve latest K turns untouched)
            if idx >= hot_window_start {
                compacted_turns.push(turn.clone());
                continue;
            }

            // Historical turns: Assistant and Tool
            let mut folded = false;
            let mut folded_content = turn.content.clone();
            let original_lines = turn.content.lines().count();
            let turn_tokens = estimate_tokens(&turn.content);

            // Tier 3: Repetitive Compiler Error Deduplication
            if let Some(resolved_turn_idx) = resolved_map[idx] {
                if let Some(ref diag) = diagnostics[idx] {
                    let node_id = Self::generate_node_id(&turn.content);
                    Self::persist_raw_content(&node_id, &turn.content, options);

                    folded_content = format!(
                        "[Superseded Compilation Error: {} (Turn {}) -> Resolved in Turn {}]",
                        diag.summary, turn.turn_index, resolved_turn_idx
                    );
                    folded = true;

                    events.push(GcEvent {
                        event: "turn_folded".to_string(),
                        timestamp: now_ms,
                        data: json!({
                            "turn_index": turn.turn_index,
                            "role": format!("{:?}", turn.role).to_lowercase(),
                            "tool": turn.tool_name,
                            "original_lines": original_lines,
                            "node_id": node_id,
                            "fold_tier": "tier_3_error_dedup",
                            "resolved_in_turn": resolved_turn_idx,
                        }),
                    });
                }
            }

            // Tier 2: Historical Tool Output Offloading (if not already folded by Tier 3)
            if !folded
                && turn.role == TurnRole::Tool
                && (original_lines > options.line_threshold || turn_tokens > 150)
            {
                let node_id = Self::generate_node_id(&turn.content);
                Self::persist_raw_content(&node_id, &turn.content, options);

                folded_content = format!(
                    "[Offloaded Tool Output: NodeID {} | {} lines | Trace: k0maru inspect {}]",
                    node_id, original_lines, node_id
                );
                folded = true;

                events.push(GcEvent {
                    event: "turn_folded".to_string(),
                    timestamp: now_ms,
                    data: json!({
                        "turn_index": turn.turn_index,
                        "role": "tool",
                        "tool": turn.tool_name,
                        "original_lines": original_lines,
                        "node_id": node_id,
                        "fold_tier": "tier_2_tool_offload",
                    }),
                });
            }

            if folded {
                folded_nodes_count += 1;
                compacted_turns.push(TrajectoryTurn {
                    turn_index: turn.turn_index,
                    role: turn.role,
                    content: folded_content,
                    tool_name: turn.tool_name.clone(),
                    tool_call_id: turn.tool_call_id.clone(),
                });
            } else {
                compacted_turns.push(turn.clone());
            }
        }

        let compacted_trajectory = SessionTrajectory {
            session_id: trajectory.session_id.clone(),
            turns: compacted_turns,
        };

        let compacted_tokens: usize = compacted_trajectory
            .turns
            .iter()
            .map(|t| estimate_tokens(&t.content))
            .sum();

        let token_reduction_ratio = if raw_tokens > 0 {
            ((raw_tokens.saturating_sub(compacted_tokens)) as f64 / raw_tokens as f64) * 100.0
        } else {
            0.0
        };

        let smart_zone_occupancy = if options.context_window_tokens > 0 {
            compacted_tokens as f64 / options.context_window_tokens as f64
        } else {
            0.0
        };

        let in_smart_zone = smart_zone_occupancy >= options.smart_zone_min_ratio
            && smart_zone_occupancy <= options.smart_zone_max_ratio;

        let result = GcResult {
            raw_tokens,
            compacted_tokens,
            token_reduction_ratio,
            raw_turns_count: total_turns,
            compacted_turns_count: compacted_trajectory.turns.len(),
            folded_nodes_count,
            smart_zone_occupancy,
            in_smart_zone,
            compacted_trajectory,
        };

        events.push(GcEvent {
            event: "gc_complete".to_string(),
            timestamp: now_ms,
            data: json!({
                "raw_tokens": raw_tokens,
                "compacted_tokens": compacted_tokens,
                "trr": token_reduction_ratio,
                "smart_zone_occupancy": smart_zone_occupancy,
                "in_smart_zone": in_smart_zone,
            }),
        });

        Ok((result, events))
    }

    /// Computes clean deterministic node ID using xxHash3.
    fn generate_node_id(content: &str) -> String {
        let hash = xxhash_rust::xxh3::xxh3_64(content.as_bytes());
        format!("node_{:08x}", hash as u32)
    }

    /// Persists raw offloaded content to refs directory if configured or exists.
    fn persist_raw_content(node_id: &str, content: &str, options: &GcOptions) {
        if let Some(ref dir) = options.refs_dir {
            let _ = std::fs::create_dir_all(dir);
            let _ = std::fs::write(dir.join(format!("{}.log", node_id)), content);
        } else {
            let scratch_refs = Path::new(".scratch/refs");
            if scratch_refs.is_dir() {
                let _ = std::fs::write(scratch_refs.join(format!("{}.log", node_id)), content);
            }
        }
    }

    /// Extracts compiler or runtime diagnostic error signature.
    fn extract_diagnostic(content: &str) -> Option<DiagnosticSignature> {
        let lines: Vec<&str> = content.lines().collect();

        // 1. Rustc diagnostics: error[E\d+] or error:
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if let Some(pos) = trimmed.find("error[E") {
                let error_part = &trimmed[pos..];
                // Extract error code if present
                let code = if let Some(end_bracket) = error_part.find(']') {
                    let code_str = &error_part[6..end_bracket];
                    Some(code_str.to_string())
                } else {
                    None
                };

                // Check subsequent lines for source location (--> src/main.rs:42:13)
                let location = lines
                    .iter()
                    .skip(i + 1)
                    .take(3)
                    .find_map(|l| l.trim().strip_prefix("-->").map(str::trim));

                let summary = if let Some(loc) = location {
                    format!("{} on {}", error_part, loc)
                } else {
                    error_part.to_string()
                };

                return Some(DiagnosticSignature { summary, code });
            } else if trimmed.starts_with("error: ") {
                let location = lines
                    .iter()
                    .skip(i + 1)
                    .take(3)
                    .find_map(|l| l.trim().strip_prefix("-->").map(str::trim));

                let summary = if let Some(loc) = location {
                    format!("{} on {}", trimmed, loc)
                } else {
                    trimmed.to_string()
                };

                return Some(DiagnosticSignature {
                    summary,
                    code: None,
                });
            }
        }

        // 2. Python traceback
        if content.contains("Traceback (most recent call last):") {
            for line in lines.iter().rev() {
                let trimmed = line.trim();
                if trimmed.contains("Error:") || trimmed.contains("Exception:") {
                    return Some(DiagnosticSignature {
                        summary: trimmed.to_string(),
                        code: None,
                    });
                }
            }
        }

        // 3. Node/npm errors
        for line in &lines {
            let trimmed = line.trim();
            if trimmed.starts_with("npm ERR!") {
                return Some(DiagnosticSignature {
                    summary: trimmed.to_string(),
                    code: None,
                });
            }
        }

        // 4. C/C++ fatal error or undefined reference
        for line in &lines {
            let trimmed = line.trim();
            if trimmed.contains("fatal error:") || trimmed.contains("undefined reference to") {
                return Some(DiagnosticSignature {
                    summary: trimmed.to_string(),
                    code: None,
                });
            }
        }

        None
    }

    /// Checks if a turn indicates compiler / test success.
    fn indicates_success(content: &str) -> bool {
        let lower = content.to_lowercase();
        content.contains("test result: ok")
            || content.contains("Finished dev")
            || content.contains("Finished release")
            || content.contains("Build succeeded")
            || content.contains("0 failed")
            || content.contains("Passed")
            || lower.contains("compilation succeeded")
            || lower.contains("resolved")
            || lower.contains("all tests passed")
    }
}
