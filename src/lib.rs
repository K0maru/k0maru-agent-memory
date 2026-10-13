//! K0maru: Agent Memory Hub
//!
//! Cleanroom, single-static-binary, zero-daemon agent memory hub mounting Obsidian SecondBrain and LLM-Wiki.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Re-export core library submodules and domain types from k0maru-core
pub use k0maru_core::{
    adapters, core, default_embedding_engine, ensure_sqlite_vec_registered, extract_elements,
    extract_tags, extract_wikilinks, gc, parse_content_hash, parse_frontmatter, parser,
    resolve_embedding_engine, scanner, storage, vector, CacheStorage, CachedDocMeta,
    ContextGcEngine, DirtySets, Document, EmbeddingEngine, EmbeddingModelType, GcError, GcEvent,
    GcOptions, GcResult, GenericWikiAdapter, HierarchyLevel, HybridSearchEngine,
    IncrementalScanner, MockEmbeddingEngine, Note, ObsidianAdapter, SearchMode, SearchResult,
    SessionTrajectory, SqliteStorage, SyncStats, TrajectoryTurn, TurnRole, VaultAdapter,
    VectorError, VectorSyncEngine, VectorSyncStats, WikiLink, DEFAULT_BATCH_SIZE,
    VECTOR_DIMENSIONS,
};
#[cfg(feature = "fastembed")]
pub use k0maru_core::{resolve_cache_dir, FastEmbedBackend};

// Application modules
pub mod convention;
pub mod crystal;
pub mod distill;
pub mod doctor;
pub mod ecosystem;
pub mod git;
pub mod guard;
pub mod install;
pub mod loadout;
pub mod mcp;
pub mod offload;
pub mod ui;

pub use convention::{
    generate_filename, slugify, synthesize_markdown, ConventionSniffer, FlushEngine, FlushRequest,
    FlushResult, NamingStyle, NoteCategory, VaultConvention,
};
pub use crystal::{
    find_tuples, resolve_tuple_dir, CrystallizeResult, LindaTuple, TaskPayload, TupleCrystallizer,
};
pub use distill::{
    DistillEngine, DistillOptions, DistillResult, DistillTarget, DistilledSkill, SkillExtractor,
};
pub use doctor::{
    format_report, run_diagnostics, DiagnosticItem, DoctorReport, DoctorSummary, StatusLevel,
};
pub use ecosystem::{
    inspect_all_clients, inspect_client, ClientConfigInfo, ClientConfigStatus, McpClient,
};
pub use git::{parse_diff_stream, FileChangeStatus, GitInspector, ModifiedFile};
pub use guard::{
    format_guard_report, get_git_diff, resolve_guard_rules, run_guard_check, run_guard_install,
    GuardArgs, GuardCheckArgs, GuardInstallArgs, GuardSubcommand,
};
pub use install::{
    format_install_report, inject_k0maru_mcp, run_install, ClientInstallOutcome, InstallOptions,
    InstallReport, InstallTarget,
};
pub use loadout::{
    copy_to_clipboard, estimate_tokens, L2LogSummary, L3CardSummary, LoadoutBuilder, LoadoutResult,
    ProjectInfo,
};
pub use mcp::McpServer;
pub use offload::{inspect_node, OffloadEngine, OffloadResult};
pub use ui::run_server;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constant() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }
}
