//! Sub-300-token context loadout builder and CLI submodule.

pub mod builder;
pub mod metadata;

pub use builder::{
    copy_to_clipboard, estimate_tokens, L2LogSummary, L3CardSummary, LoadoutBuilder, LoadoutResult,
    ProjectInfo,
};
pub use metadata::ProjectMetadata;
