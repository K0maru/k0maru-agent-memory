//! Trace-to-Skill dynamic experience distillation engine.
//!
//! Inspired by Nous Research Hermes Agent dynamic skills and Karpathy LLM-Wiki
//! bidirectional memory consolidation, this module parses raw execution traces,
//! compiler errors, stack traces, and remediation commands into structured,
//! evergreen skill notes.

pub mod extractor;
pub mod model;

pub use extractor::SkillExtractor;
pub use model::DistilledSkill;
