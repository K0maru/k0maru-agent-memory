//! Long-Trajectory Context Garbage Collection (Context GC) module.
//!
//! Provides automated session context compaction and pruning for multi-turn agent sessions (50+ turns),
//! enforcing the 4-tier pruning protocol:
//! - Tier 0: Inviolable Anchor Guard (never prune System or User turns)
//! - Tier 1: Hot Window (preserve latest $K=5$ turns untouched)
//! - Tier 2: Historical Tool Output Offloading (lines > 20 or tokens > 150 folded to symbolic refs)
//! - Tier 3: Repetitive Compiler Error Deduplication (superseded diagnostics folded into resolution citations)

pub mod engine;
pub mod models;

pub use engine::{estimate_tokens, ContextGcEngine, GcError};
pub use models::{GcEvent, GcOptions, GcResult, SessionTrajectory, TrajectoryTurn, TurnRole};
