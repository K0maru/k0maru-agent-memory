use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Role of a message in a session trajectory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TurnRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A single turn in a multi-turn conversation trajectory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrajectoryTurn {
    #[serde(default)]
    pub turn_index: usize,
    pub role: TurnRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none", default, alias = "name")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum SessionTrajectoryWire {
    Object {
        #[serde(default = "default_session_id")]
        session_id: String,
        turns: Vec<TrajectoryTurn>,
    },
    Array(Vec<TrajectoryTurn>),
}

fn default_session_id() -> String {
    "default".to_string()
}

impl From<SessionTrajectoryWire> for SessionTrajectory {
    fn from(wire: SessionTrajectoryWire) -> Self {
        match wire {
            SessionTrajectoryWire::Object {
                session_id,
                mut turns,
            } => {
                for (idx, turn) in turns.iter_mut().enumerate() {
                    if turn.turn_index == 0 && idx > 0 {
                        turn.turn_index = idx;
                    }
                }
                SessionTrajectory { session_id, turns }
            }
            SessionTrajectoryWire::Array(mut turns) => {
                for (idx, turn) in turns.iter_mut().enumerate() {
                    if turn.turn_index == 0 && idx > 0 {
                        turn.turn_index = idx;
                    }
                }
                SessionTrajectory {
                    session_id: "default".to_string(),
                    turns,
                }
            }
        }
    }
}

/// A sequence of turns representing an agent execution session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "SessionTrajectoryWire")]
pub struct SessionTrajectory {
    pub session_id: String,
    pub turns: Vec<TrajectoryTurn>,
}

fn default_line_threshold() -> usize {
    20
}

fn default_hot_window_turns() -> usize {
    5
}

fn default_context_window_tokens() -> usize {
    128_000
}

fn default_smart_zone_max_ratio() -> f64 {
    0.60
}

fn default_smart_zone_min_ratio() -> f64 {
    0.15
}

/// Options controlling the 4-tier Context GC compaction behavior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GcOptions {
    /// Line threshold before tool output is folded (default: 20).
    #[serde(default = "default_line_threshold")]
    pub line_threshold: usize,

    /// Number of recent turns preserved untouched in hot window (default: 5).
    #[serde(default = "default_hot_window_turns")]
    pub hot_window_turns: usize,

    /// Context window size for Smart-Zone evaluation in tokens (default: 128,000).
    #[serde(default = "default_context_window_tokens")]
    pub context_window_tokens: usize,

    /// Target Smart-Zone upper bound ratio (default: 0.60).
    #[serde(default = "default_smart_zone_max_ratio")]
    pub smart_zone_max_ratio: f64,

    /// Target Smart-Zone lower bound ratio (default: 0.15).
    #[serde(default = "default_smart_zone_min_ratio")]
    pub smart_zone_min_ratio: f64,

    /// Optional directory to persist raw offloaded logs (defaults to `.scratch/refs`).
    #[serde(default)]
    pub refs_dir: Option<PathBuf>,
}

impl Default for GcOptions {
    fn default() -> Self {
        Self {
            line_threshold: default_line_threshold(),
            hot_window_turns: default_hot_window_turns(),
            context_window_tokens: default_context_window_tokens(),
            smart_zone_max_ratio: default_smart_zone_max_ratio(),
            smart_zone_min_ratio: default_smart_zone_min_ratio(),
            refs_dir: None,
        }
    }
}

/// Result of Context GC compaction including metrics and compacted trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GcResult {
    /// Total estimated tokens before compaction.
    pub raw_tokens: usize,

    /// Total estimated tokens after compaction.
    pub compacted_tokens: usize,

    /// Token Reduction Ratio (TRR) as a percentage (e.g. 87.5 for 87.5%).
    #[serde(alias = "trr")]
    pub token_reduction_ratio: f64,

    /// Number of turns before GC.
    pub raw_turns_count: usize,

    /// Number of turns after GC.
    pub compacted_turns_count: usize,

    /// Number of turns whose content was folded or offloaded.
    pub folded_nodes_count: usize,

    /// Fraction of context window occupied by compacted tokens.
    pub smart_zone_occupancy: f64,

    /// Whether compacted tokens fall within the cognitive Smart-Zone (15%-60%).
    pub in_smart_zone: bool,

    /// The resulting compacted session trajectory.
    pub compacted_trajectory: SessionTrajectory,
}

impl GcResult {
    /// Token reduction ratio as percentage (e.g. 87.5%).
    pub fn trr(&self) -> f64 {
        self.token_reduction_ratio
    }

    /// Token reduction ratio as a fraction in [0.0, 1.0].
    pub fn trr_fraction(&self) -> f64 {
        self.token_reduction_ratio / 100.0
    }
}

/// Structured streaming event emitted during `--json-stream` execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GcEvent {
    pub event: String,
    pub timestamp: u64,
    pub data: serde_json::Value,
}
