//! K0maru: Agent Memory Hub
//!
//! Cleanroom, single-static-binary, zero-daemon agent memory hub mounting Obsidian Vaults and LLM-Wikis.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Re-export core library submodules and domain types from k0maru-core
pub use k0maru_core::storage::{
    resolve_global_vault, FederatedSearch, ReadOnlyStorage, GLOBAL_PREFIX,
};
pub use k0maru_core::{
    adapters, core, default_embedding_engine, ensure_sqlite_vec_registered, extract_elements,
    extract_tags, extract_wikilinks, gc, parse_content_hash, parse_frontmatter, parser,
    resolve_embedding_engine, scanner, storage, text, vector, vector_scope_enabled, CacheStorage,
    CachedDocMeta, ContextGcEngine, DirtySets, Document, EmbeddingEngine, EmbeddingModelType,
    GcError, GcEvent, GcOptions, GcResult, GenericWikiAdapter, HierarchyLevel, HybridSearchEngine,
    IncrementalScanner, MockEmbeddingEngine, Note, NoteLifecycleStatus, ObsidianAdapter,
    RuleCondition, RuleSeverity, SearchMode, SearchResult, SectionLocator, SectionVectorStats,
    SessionTrajectory, SqliteStorage, SyncStats, TrajectoryTurn, TurnRole, VaultAdapter,
    VectorError, VectorSyncEngine, VectorSyncStats, WikiLink, DEFAULT_BATCH_SIZE,
    VECTOR_DIMENSIONS,
};
#[cfg(feature = "fastembed")]
pub use k0maru_core::{resolve_cache_dir, FastEmbedBackend};

// Application modules
pub mod bootstrap;
pub mod config;
pub mod convention;
pub mod distill;
pub mod doctor;
pub mod ecosystem;
pub mod garden;
pub mod git;
pub mod guard;
pub mod handoff;
pub mod hook;
pub mod install;
pub mod loadout;
pub mod mcp;
pub mod offload;
pub mod pack;
pub mod ui;

pub use bootstrap::{format_bootstrap_report, run_bootstrap, BootstrapOptions, BootstrapReport};
pub use config::K0maruConfig;
pub use convention::{
    generate_filename, slugify, synthesize_markdown, ConventionSniffer, FlushEngine, FlushRequest,
    FlushResult, NamingStyle, NoteCategory, VaultConvention,
};
pub use distill::{
    DistillEngine, DistillOptions, DistillResult, DistillTarget, DistilledSkill, SkillExtractor,
};
pub use doctor::{
    compact_cache, format_report, probe_global_vault, run_diagnostics,
    run_diagnostics_with_compact, run_diagnostics_with_options, DiagnosticItem, DoctorReport,
    DoctorSummary, StatusLevel,
};
pub use ecosystem::{
    inspect_all_clients, inspect_client, ClientConfigInfo, ClientConfigStatus, McpClient,
};
pub use garden::{
    format_garden_report, run_garden, BrokenLinkReport, DuplicateReport, GardenOptions,
    GardenReport, OrphanReport, WeavePlanItem, WeaveReport, WeaveSkipItem,
};
pub use git::{
    git_preflight, is_path_dirty, parse_diff_stream, parse_git_log_times, run_git_command,
    sync_git_pre_scan, FileChangeStatus, GitInspector, GitPreflight, GitSyncOutcome, ModifiedFile,
};
pub use guard::{
    format_guard_report, format_guard_report_with_global, get_git_diff, resolve_guard_rules,
    resolve_guard_rules_with_global, run_guard_check, run_guard_check_with_global,
    run_guard_install, GlobalRuleOrigin, GuardArgs, GuardCheckArgs, GuardEngine, GuardInstallArgs,
    GuardSubcommand,
};
pub use handoff::{
    format_handoff_list_markdown, format_handoff_markdown, parse_duration_secs,
    ClaimHandoffRequest, CreateHandoffRequest, HandoffEntry, HandoffError, HandoffFileRef,
    HandoffRecord, HandoffState, HandoffStatus, HandoffStore, PruneResult, HANDOFF_SCHEMA_VERSION,
    MAX_HANDOFF_BYTES,
};
pub use hook::{
    build_timeline, inspect_note_detail, inspect_note_detail_ro,
    inspect_note_detail_ro_with_options, inspect_note_detail_with_options, parse_locator, run_hook,
    run_hook_with_global, HookEvent, HookOptions, HookReport, NoteDetailReport, ParsedLocator,
    TimelineEntry, TimelineQuery,
};

/// Opens a resolved global vault cache in strict read-only mode, warning once on failure.
pub fn open_global_storage_with_warning(global_vault: &std::path::Path) -> Option<ReadOnlyStorage> {
    let cache_path = global_vault.join(".k0maru").join("cache.sqlite");
    match ReadOnlyStorage::open(&cache_path) {
        Ok(ro) => Some(ro),
        Err(err) => {
            eprintln!(
                "[k0maru] warning: failed to open global vault cache {}: {}; continuing without global scope",
                cache_path.display(),
                err
            );
            None
        }
    }
}
pub use install::{
    format_install_report, inject_k0maru_mcp, install_claude_project_hooks, install_repo_hooks,
    run_install, ClientInstallOutcome, InstallOptions, InstallReport, InstallTarget,
};
pub use loadout::{
    copy_to_clipboard, estimate_tokens, L2LogSummary, L3CardSummary, LoadoutBuilder, LoadoutResult,
    ProjectInfo,
};
pub use mcp::McpServer;
pub use offload::{inspect_node, OffloadEngine, OffloadResult};
pub use pack::{
    export_pack, import_pack, inspect_pack, install_pack, list_installed_packs,
    normalize_wikilinks, uninstall_pack, InstalledPackManifest, MemoryPack, PackExportOptions,
    PackImportItem, PackImportOptions, PackImportResult, PackInspectReport, PackInstallOptions,
    PackInstallResult, PackNote, PackUninstallResult,
};
pub use ui::run_server;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constant() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }
}
