//! K0maru: Agent Memory Hub
//!
//! Cleanroom, single-static-binary, zero-daemon agent memory hub mounting Obsidian SecondBrain and LLM-Wiki.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod adapters;
pub mod core;
pub mod loadout;
pub mod mcp;
pub mod offload;
pub mod parser;
pub mod scanner;
pub mod storage;
pub mod ui;
pub mod vector;

pub use adapters::{GenericWikiAdapter, ObsidianAdapter};
pub use loadout::{
    copy_to_clipboard, estimate_tokens, L2LogSummary, L3CardSummary, LoadoutBuilder, LoadoutResult,
    ProjectInfo,
};
pub use mcp::McpServer;
pub use offload::{inspect_node, OffloadEngine, OffloadResult};
pub use scanner::{IncrementalScanner, VectorSyncEngine, VectorSyncStats};
pub use storage::{HybridSearchEngine, SearchMode, SearchResult, SqliteStorage};
pub use ui::run_server;
#[cfg(feature = "fastembed")]
pub use vector::FastEmbedBackend;
pub use vector::{default_embedding_engine, EmbeddingEngine, MockEmbeddingEngine, VectorError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constant() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }
}
