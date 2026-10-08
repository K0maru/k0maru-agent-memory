//! Trace-to-Skill dynamic experience distillation engine.
//!
//! Inspired by Nous Research Hermes Agent dynamic skills and Karpathy LLM-Wiki
//! bidirectional memory consolidation, this module parses raw execution traces,
//! compiler errors, stack traces, and remediation commands into structured,
//! evergreen skill notes.

pub mod engine;
pub mod extractor;
pub mod model;

pub use engine::{DistillEngine, DistillOptions, DistillResult};
pub use extractor::SkillExtractor;
pub use model::{DistillTarget, DistilledSkill};
