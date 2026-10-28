use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand};

use k0maru::adapters::{GenericWikiAdapter, ObsidianAdapter};
use k0maru::convention::{FlushEngine, FlushRequest};
use k0maru::core::traits::{CacheStorage, VaultAdapter};
use k0maru::distill::{DistillEngine, DistillOptions, DistillTarget};
use k0maru::doctor::{format_report, run_diagnostics};
use k0maru::git::{parse_diff_stream, GitInspector};
use k0maru::guard::{
    format_guard_report, run_guard_check, run_guard_install, GuardArgs, GuardSubcommand,
};
use k0maru::install::{format_install_report, run_install, InstallOptions, InstallTarget};
use k0maru::loadout::{copy_to_clipboard, LoadoutBuilder};
use k0maru::offload::{inspect_node, OffloadEngine};
use k0maru::pack::{
    export_pack, import_pack, inspect_pack, install_pack, list_installed_packs, uninstall_pack,
    MemoryPack, PackExportOptions, PackImportOptions, PackInstallOptions,
};
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::{HybridSearchEngine, SearchMode, SqliteStorage};
use k0maru::vector::resolve_embedding_engine;

#[derive(Parser, Debug)]
#[command(
    name = "k0maru",
    version,
    about = "Zero-daemon agent memory hub mounting Obsidian and LLM-Wiki",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a sub-300-token context loadout for a project
    Loadout(LoadoutArgs),
    /// Bootstrap a zero-config LLM-Wiki & Living ADR Guard from any Git repository
    #[command(
        visible_alias = "bootstrap",
        about = "Bootstrap a zero-config LLM-Wiki & Living ADR Guard from any Git repository"
    )]
    Init(InitArgs),

    /// Truncate and offload long command output to symbolic Mermaid graph
    Offload(OffloadArgs),

    /// Inspect offloaded raw log content by node ID
    Inspect(InspectArgs),

    /// Start zero-daemon FastMCP stdio server for AI agents (Claude Code, Cursor, Windsurf)
    Mcp(McpArgs),

    /// Synchronize vault documents incrementally into disposable SQLite cache
    Sync(SyncArgs),

    /// Search knowledge hub memories and notes using BM25, semantic vector, or hybrid retrieval
    Search(SearchArgs),

    /// Diagnose system health, vault integrity, storage indices, and MCP client configurations
    Doctor(DoctorArgs),

    /// Automatically configure k0maru-memory MCP server in AI coding agents (Claude, Cursor, etc.)
    Install(InstallArgs),

    /// Flush crystallized memory or notes into the target vault according to conventions
    Flush(FlushArgs),

    /// Distill troubleshooting traces or execution logs into reusable skills
    Distill(DistillArgs),

    /// Long-trajectory context garbage collection (Context GC) with 4-tier pruning protocol
    Gc(GcArgs),

    /// Living ADR guard and architectural invariant sentinel
    Guard(GuardArgs),

    /// Export, import, or inspect portable LLM-Wiki Memory Packs (.pack.json)
    Pack(PackArgs),

    #[command(about = "Launch the local developer dashboard and visual memory explorer")]
    Ui {
        #[arg(short, long, help = "Path to the markdown vault")]
        vault: Option<PathBuf>,

        #[arg(
            short,
            long,
            default_value_t = 3721,
            help = "Port to bind the local dashboard server"
        )]
        port: u16,

        #[arg(long, help = "Automatically open default browser")]
        open: bool,

        #[arg(long, visible_alias = "embedder", help = "Embedding model to use")]
        embed_model: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct LoadoutArgs {
    /// Target project name or search query
    #[arg(value_name = "QUERY")]
    pub query: Option<String>,

    /// Path to vault root directory (defaults to current dir or detects nearest vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Copy formatted loadout to OS clipboard
    #[arg(short, long)]
    pub copy: bool,

    /// Output structured JSON instead of Markdown
    #[arg(long)]
    pub json: bool,

    /// Output line-delimited JSON stream (NDJSON)
    #[arg(long)]
    pub json_stream: bool,

    /// List all active projects in the vault
    #[arg(short, long)]
    pub list: bool,

    /// Inspect Git working tree / index changes and dynamically activate module-relevant ADRs
    #[arg(short, long)]
    pub diff: bool,

    /// Target Git base branch for comparison (e.g. 'main', 'dev')
    #[arg(short, long, value_name = "BRANCH")]
    pub branch: Option<String>,
}

#[derive(Args, Debug)]
pub struct OffloadArgs {
    /// Line threshold before offloading logs (defaults to 50)
    #[arg(short = 't', long, default_value = "50")]
    pub threshold: usize,

    /// Optional task identifier to prefix log files
    #[arg(long)]
    pub task_id: Option<String>,

    /// Directory to store offloaded references (defaults to .scratch/refs or <vault>/.k0maru/refs)
    #[arg(short = 'r', long, value_name = "PATH")]
    pub refs_dir: Option<PathBuf>,

    /// Output line-delimited JSON stream (NDJSON)
    #[arg(long)]
    pub json_stream: bool,
}

#[derive(Args, Debug)]
pub struct InspectArgs {
    /// Node ID or log reference to inspect
    #[arg(value_name = "NODE_ID")]
    pub node_id: String,

    /// Directory where offloaded references are stored (defaults to .scratch/refs or <vault>/.k0maru/refs)
    #[arg(short = 'r', long, value_name = "PATH")]
    pub refs_dir: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct McpArgs {
    /// Path to vault root directory (defaults to current dir or detects nearest vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Embedding model to use for semantic vector retrieval (e.g. all-minilm-l6-v2, bge-small-zh-v1.5, mock)
    #[arg(long, visible_alias = "embedder", value_name = "MODEL")]
    pub embed_model: Option<String>,

    /// Run in orthogonal sidecar mode (omits offload_context, inspect_log_node, and gc_session_context tools)
    #[arg(long)]
    pub sidecar: bool,
}

#[derive(Args, Debug)]
pub struct SyncArgs {
    /// Path to vault root directory (defaults to current dir or detects nearest vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Force full rebuild of all documents, ignoring cached mtime and hashes
    #[arg(short, long)]
    pub force: bool,

    /// Output structured JSON instead of human-readable text
    #[arg(long)]
    pub json: bool,

    /// Generate vector embeddings for synchronized documents
    #[arg(long)]
    pub vector: bool,

    /// Embedding model to use for vector generation (e.g. all-minilm-l6-v2, bge-small-zh-v1.5, mock)
    #[arg(long, visible_alias = "embedder", value_name = "MODEL")]
    pub embed_model: Option<String>,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search query string (use '-' to read query from stdin)
    #[arg(value_name = "QUERY")]
    pub query: String,

    /// Path to vault root directory (defaults to current dir or detects nearest vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Search execution mode: hybrid, bm25, or vector
    #[arg(short, long, default_value = "hybrid")]
    pub mode: String,

    /// Maximum number of search results to return
    #[arg(short, long, default_value = "5")]
    pub limit: usize,

    /// Output full machine-readable JSON array of search hits
    #[arg(long)]
    pub json: bool,

    /// Output line-delimited JSON stream (NDJSON)
    #[arg(long)]
    pub json_stream: bool,

    /// Embedding model to use for vector query (e.g. all-minilm-l6-v2, bge-small-zh-v1.5, mock)
    #[arg(long, visible_alias = "embedder", value_name = "MODEL")]
    pub embed_model: Option<String>,
}

#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Path to vault root directory (defaults to current dir or detects nearest vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Home directory override for inspecting client configs (defaults to system user home)
    #[arg(long, value_name = "PATH")]
    pub home: Option<PathBuf>,

    /// Output structured JSON instead of human-readable report
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    /// Path to target git repository (defaults to current working directory)
    #[arg(short, long, value_name = "PATH")]
    pub repo: Option<PathBuf>,

    /// Path to destination knowledge vault (defaults to <repo>/.k0maru/wiki)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Override project name / slug
    #[arg(short, long, value_name = "NAME")]
    pub project: Option<String>,

    /// Automatically configure agent MCP clients and proactive lifecycle hooks (.git & .claude)
    #[arg(short, long)]
    pub install: bool,

    /// Home directory override for installing client configs (defaults to system user home)
    #[arg(long, value_name = "PATH")]
    pub home: Option<PathBuf>,

    /// Output structured JSON report instead of human-readable text
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Path to vault root directory (defaults to current dir or detects nearest vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Target client to configure: all, claude, cursor, gemini, windsurf, cline, hermes, openclaw, omp
    #[arg(short, long, default_value = "all")]
    pub target: String,

    /// Home directory override for installing client configs (defaults to system user home)
    #[arg(long, value_name = "PATH")]
    pub home: Option<PathBuf>,

    /// Also install proactive lifecycle hooks (.claude/settings.local.json SessionStart & Git pre-commit guard)
    #[arg(
        long,
        help = "Also install proactive lifecycle hooks (.claude/settings.local.json SessionStart & Git pre-commit guard)"
    )]
    pub hooks: bool,
    /// Preview configuration changes without writing to disk
    #[arg(long)]
    pub dry_run: bool,

    /// Output structured JSON instead of human-readable text
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct FlushArgs {
    /// Target vault path (defaults to detected vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Title of the decision or log note
    #[arg(short, long)]
    pub title: String,

    /// Summary or executive conclusion
    #[arg(short, long)]
    pub summary: Option<String>,

    /// Detailed Markdown content (can also be piped from stdin)
    #[arg(short, long)]
    pub content: Option<String>,

    /// Category: decision, log, concept (alias: card), skill, generic
    #[arg(long, default_value = "log")]
    pub category: String,

    /// Comma-separated tags
    #[arg(long, value_delimiter = ',')]
    pub tags: Vec<String>,

    /// Comma-separated titles of related notes to link
    #[arg(long, value_delimiter = ',')]
    pub related: Vec<String>,

    /// Preview the generated note and destination path without writing to disk
    #[arg(long)]
    pub dry_run: bool,

    /// Comma-separated notes or ADRs that this note supersedes (automatically marks old notes as status: superseded)
    #[arg(long, value_delimiter = ',')]
    pub supersedes: Vec<String>,

    /// Output structured JSON result
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct DistillArgs {
    /// Target vault path (defaults to detected vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Offloaded node ID to distill from refs
    #[arg(long, value_name = "NODE_ID")]
    pub node: Option<String>,

    /// Path to a log/trace file to distill
    #[arg(short, long, value_name = "PATH")]
    pub file: Option<PathBuf>,

    /// Explicit skill note title (auto-inferred if omitted)
    #[arg(short, long)]
    pub title: Option<String>,

    /// Context hint or prompt explaining the troubleshooting trace
    #[arg(short = 'c', long)]
    pub context: Option<String>,

    /// Category: skill, playbook, recipe, etc.
    #[arg(long, default_value = "skill")]
    pub category: String,

    /// Comma-separated tags
    #[arg(long, value_delimiter = ',')]
    pub tags: Vec<String>,

    /// Comma-separated titles of related notes to link
    #[arg(long, value_delimiter = ',')]
    pub related: Vec<String>,

    /// Directory where offloaded references are stored
    #[arg(short = 'r', long, value_name = "PATH")]
    pub refs_dir: Option<PathBuf>,

    /// Override user home directory for configuration discovery (sandbox testing)
    #[arg(long, value_name = "PATH")]
    pub home: Option<PathBuf>,

    /// Target ecosystem schema: default, hermes, omp
    #[arg(long, default_value = "default")]
    pub target: String,

    /// Export crystallized dynamic skill directly to Hermes skill directory
    #[arg(long)]
    pub export_hermes: bool,

    /// Export crystallized skill directly to omp skill directory (~/.omp/agent/skills/<slug>/SKILL.md)
    #[arg(long)]
    pub export_omp: bool,

    /// Preview the generated skill note without writing to disk
    #[arg(long)]
    pub dry_run: bool,

    /// Output structured JSON result
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct GcArgs {
    /// Path to trajectory JSON file (defaults to stdin if omitted)
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Path to the vault root (accepted for CLI compatibility; GC is vault-independent)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Line threshold for folding tool outputs (default: 20)
    #[arg(short = 't', long, default_value = "20")]
    pub threshold: usize,

    /// Number of recent turns preserved in hot window (default: 5)
    #[arg(short = 'k', long, default_value = "5")]
    pub hot_window: usize,

    /// Context window size in tokens for Smart-Zone evaluation (default: 128000)
    #[arg(long, default_value = "128000")]
    pub context_window: usize,

    /// Output structured JSON
    #[arg(long)]
    pub json: bool,

    /// Output line-delimited JSON stream
    #[arg(long)]
    pub json_stream: bool,
}

#[derive(Args, Debug)]
pub struct PackArgs {
    #[command(subcommand)]
    pub command: PackSubcommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum PackSubcommand {
    /// Export domain knowledge cards and Living ADR invariants into a portable Memory Pack (.pack.json)
    Export(PackExportArgs),

    /// Import a portable Memory Pack into an LLM-Wiki
    Import(PackImportArgs),

    /// Inspect a portable Memory Pack structure and predicted target paths
    Inspect(PackInspectArgs),

    /// Install a portable Memory Pack (.pack.json) or OpenHarness DSH directory into isolated pack storage
    Install(PackInstallArgs),

    /// Uninstall an installed Memory Pack or OpenHarness DSH and clean up skills
    Uninstall(PackUninstallArgs),

    /// List all installed Memory Packs and OpenHarness DSH packs
    List(PackListArgs),
}

#[derive(Args, Debug, Clone)]
pub struct PackExportArgs {
    /// Name of the memory pack
    #[arg(long)]
    pub name: String,

    /// Target vault path (defaults to detected vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Version of the memory pack
    #[arg(long, default_value = "0.1.0")]
    pub version: String,

    /// Optional description of the memory pack
    #[arg(long)]
    pub description: Option<String>,

    /// Export notes matching specific tag (e.g. topic/redis)
    #[arg(long)]
    pub tag: Option<String>,

    /// Export notes matching search query
    #[arg(long)]
    pub query: Option<String>,

    /// Specific markdown files to include in export
    #[arg(long = "files", value_name = "FILE", num_args = 1..)]
    pub files: Vec<PathBuf>,

    /// Write pack JSON to output file instead of stdout
    #[arg(short = 'o', long = "output", value_name = "OUTPUT_FILE")]
    pub output: Option<PathBuf>,

    /// Output raw JSON to stdout (even if --output is specified)
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct PackImportArgs {
    /// Path to memory pack JSON file
    #[arg(value_name = "PACK_FILE")]
    pub pack_file: PathBuf,

    /// Target vault path (defaults to detected vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Preview imported notes and target paths without writing to disk
    #[arg(long)]
    pub dry_run: bool,

    /// Output result as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct PackInspectArgs {
    /// Path to memory pack JSON file
    #[arg(value_name = "PACK_FILE")]
    pub pack_file: PathBuf,

    /// Target vault path to predict destination paths
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Output report as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct PackInstallArgs {
    /// Path to .pack.json file or OpenHarness DSH directory
    #[arg(value_name = "SOURCE")]
    pub source: PathBuf,

    /// Target vault path (defaults to detected vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Override home directory for skill mounting (sandbox testing)
    #[arg(long, value_name = "PATH")]
    pub home: Option<PathBuf>,

    /// Preview installed pack files without writing to disk
    #[arg(long)]
    pub dry_run: bool,

    /// Output result as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct PackUninstallArgs {
    /// Name or slug of the pack to uninstall
    #[arg(value_name = "NAME")]
    pub name: String,

    /// Target vault path (defaults to detected vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Override home directory for skill removal (sandbox testing)
    #[arg(long, value_name = "PATH")]
    pub home: Option<PathBuf>,

    /// Output result as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct PackListArgs {
    /// Target vault path (defaults to detected vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Output list as JSON
    #[arg(long)]
    pub json: bool,
}

/// Resolves the markdown vault `k0maru` should operate on.
///
/// Detection is deliberately conservative: `~/.k0maru` is the global model
/// cache directory, **not** a markdown vault, and an uninitialized directory
/// must never receive a `cache.sqlite` directly. When nothing is detected the
/// returned path is always a repo-local `.k0maru/wiki`.
fn detect_vault_path(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(p) = explicit {
        if p.is_relative() && !p.exists() {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let repo_root = k0maru::guard::resolve_repo_root(&cwd);
            let candidate = repo_root.join(&p);
            if candidate.exists() {
                return candidate;
            }
        }
        return p;
    }
    if let Ok(v) = std::env::var("K0MARU_VAULT") {
        if !v.trim().is_empty() {
            return PathBuf::from(v);
        }
    }
    if let Ok(dir) = std::env::var("CLAUDE_PROJECT_DIR") {
        if !dir.trim().is_empty() {
            let candidate = PathBuf::from(&dir).join(".k0maru").join("wiki");
            if candidate.is_dir() {
                return candidate;
            }
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let home_dir = std::env::var_os("HOME").map(PathBuf::from);

    // Walk up the tree looking for an already-initialized repo vault or a real
    // markdown vault (Obsidian / Johnny.Decimal layout).
    for curr in cwd.ancestors() {
        let repo_wiki = curr.join(".k0maru").join("wiki");
        if repo_wiki.is_dir() {
            return repo_wiki;
        }

        // `$HOME` itself is never a vault marker: `~/.k0maru` may merely be the
        // global model cache, and returning `$HOME` would hijack the user's
        // home directory as a knowledge base.
        if home_dir.as_deref() != Some(curr)
            && (curr.join(".obsidian").is_dir()
                || (curr.join("10_Projects").is_dir() && curr.join("20_Cards").is_dir()))
        {
            return curr.to_path_buf();
        }
    }

    // Never fall back to `.` or `$HOME`: bind to the repository-local vault so
    // cache/model artifacts stay inside `<repo>/.k0maru`.
    let repo_root = k0maru::guard::resolve_repo_root(&cwd);
    let repo_wiki = repo_root.join(".k0maru").join("wiki");
    if repo_wiki.is_dir() || repo_root.join(".git").exists() {
        return repo_wiki;
    }

    cwd.join(".k0maru").join("wiki")
}

fn detect_refs_dir(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(p) = explicit {
        return p;
    }
    if let Ok(mut current) = std::env::current_dir() {
        loop {
            if current.join(".k0maru").exists() {
                return current.join(".k0maru").join("refs");
            }
            if current.join(".scratch").exists() {
                return current.join(".scratch").join("refs");
            }
            if current.join(".obsidian").exists()
                || current.join("10_Projects").is_dir()
                || current.join("20_Cards").is_dir()
            {
                return current.join(".k0maru").join("refs");
            }
            if !current.pop() {
                break;
            }
        }
    }
    PathBuf::from(".scratch/refs")
}

fn detect_hermes_dir(home_override: Option<&Path>) -> PathBuf {
    if let Some(h) = home_override {
        return h.to_path_buf();
    }
    if let Ok(hermes_home) = std::env::var("HERMES_HOME") {
        return PathBuf::from(hermes_home);
    }
    if let Some(user_home) = dirs::home_dir() {
        return user_home.join(".hermes");
    }
    PathBuf::from(".hermes")
}

fn copy_and_notify(text: &str) {
    match copy_to_clipboard(text) {
        Ok(()) => eprintln!("📋 Copied loadout to clipboard."),
        Err(e) => eprintln!("⚠️  Could not copy to clipboard: {}", e),
    }
}

fn safe_print(msg: &str) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    if let Err(e) = write!(handle, "{}", msg) {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
    }
    if let Err(e) = handle.flush() {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
    }
}

fn safe_println(msg: &str) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    if let Err(e) = writeln!(handle, "{}", msg) {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
    }
    if let Err(e) = handle.flush() {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
    }
}

fn current_timestamp_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn emit_json_stream_event(event: &str, data: serde_json::Value) {
    let payload = serde_json::json!({
        "event": event,
        "timestamp": current_timestamp_millis(),
        "data": data,
    });
    if let Ok(line) = serde_json::to_string(&payload) {
        safe_println(&line);
    }
}

fn run_loadout_with_adapter<A: VaultAdapter>(
    adapter: &A,
    vault_path: &Path,
    args: LoadoutArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let cache_dir = vault_path.join(".k0maru");
    let cache_path = cache_dir.join("cache.sqlite");

    let mut storage = match SqliteStorage::open(&cache_path) {
        Ok(s) => s,
        Err(_) => SqliteStorage::in_memory()?,
    };

    let mut scanner = IncrementalScanner::new(adapter, &mut storage);
    let _ = scanner.sync(false);

    let builder = LoadoutBuilder::new(adapter, &storage);

    if args.list {
        let projects = builder.list_projects()?;
        if args.json_stream {
            emit_json_stream_event(
                "loadout_start",
                serde_json::json!({
                    "mode": "list",
                }),
            );
            for p in &projects {
                emit_json_stream_event(
                    "project_listed",
                    serde_json::json!({
                        "name": p.name,
                        "status": p.status,
                        "path": p.path,
                    }),
                );
            }
            emit_json_stream_event(
                "loadout_ready",
                serde_json::json!({
                    "total_projects": projects.len(),
                }),
            );
        } else if args.json {
            let json_str = serde_json::to_string_pretty(&projects)?;
            safe_println(&json_str);
            if args.copy {
                copy_and_notify(&json_str);
            }
        } else {
            safe_println("📂 Available Projects:");
            safe_println("--------------------------------------------------");
            for p in &projects {
                safe_println(&format!("  • {}  [{}] ({})", p.name, p.status, p.path));
            }
            safe_println("--------------------------------------------------");
            safe_println("💡 Usage: k0maru loadout <project-keyword>");
            if args.copy {
                let text = projects
                    .iter()
                    .map(|p| format!("  • {}  [{}] ({})", p.name, p.status, p.path))
                    .collect::<Vec<_>>()
                    .join("\n");
                copy_and_notify(&text);
            }
        }
        return Ok(());
    }

    // An uninitialized directory has no projects. A diff-mode loadout is the
    // Claude SessionStart hook, so it must remain a no-op instead of blocking
    // the session with an error.
    if args.diff {
        let has_projects = !builder.list_projects()?.is_empty();
        if !has_projects {
            if !args.json && !args.json_stream {
                safe_println(
                    "[k0maru] No project initialized in vault. Run 'k0maru init --install' to bootstrap.",
                );
            }
            return Ok(());
        }
    }

    let mut piped_files = Vec::new();
    if !std::io::stdin().is_terminal() {
        let stdin = std::io::stdin();
        if let Ok(files) = parse_diff_stream(stdin.lock()) {
            piped_files = files;
        }
    }

    if args.diff || args.branch.is_some() || !piped_files.is_empty() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut modified_files = GitInspector::inspect_diff(&cwd, args.branch.as_deref())?;

        for f in piped_files {
            if !modified_files
                .iter()
                .any(|existing| existing.path == f.path)
            {
                modified_files.push(f);
            }
        }

        let query = match args.query {
            Some(q) => q,
            None => {
                let cwd_name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if !cwd_name.is_empty() && builder.find_project(cwd_name).ok().flatten().is_some() {
                    cwd_name.to_string()
                } else {
                    let projects = builder.list_projects()?;
                    if let Some(first) = projects.first() {
                        first.name.clone()
                    } else {
                        eprintln!(
                            "❌ No projects found in vault. Use '--list' to see all initialized projects."
                        );
                        std::process::exit(2);
                    }
                }
            }
        };

        match builder.build_with_diff(&query, &modified_files)? {
            Some(loadout) => {
                if args.json_stream {
                    emit_json_stream_event(
                        "loadout_start",
                        serde_json::json!({
                            "project": query,
                            "mode": "diff",
                        }),
                    );
                    let file_paths: Vec<String> = modified_files
                        .iter()
                        .map(|f| f.path.to_string_lossy().to_string())
                        .collect();
                    emit_json_stream_event(
                        "diff_inspected",
                        serde_json::json!({
                            "modified_files": file_paths,
                        }),
                    );
                    for card in &loadout.l3_cards {
                        emit_json_stream_event(
                            "rule_activated",
                            serde_json::json!({
                                "rule": card.name,
                                "concept": card.concept.clone().unwrap_or_default(),
                            }),
                        );
                    }
                    emit_json_stream_event(
                        "loadout_ready",
                        serde_json::json!({
                            "tokens": loadout.estimated_tokens,
                            "within_budget": loadout.estimated_tokens <= 300,
                            "markdown": loadout.markdown,
                        }),
                    );
                } else if args.json {
                    let output = serde_json::to_string_pretty(&loadout)?;
                    safe_println(&output);
                    if args.copy {
                        copy_and_notify(&output);
                    }
                } else {
                    safe_println(&loadout.markdown);
                    if args.copy {
                        copy_and_notify(&loadout.markdown);
                    }
                }
            }
            None => {
                eprintln!(
                    "❌ No project matched '{}'. Use '--list' to see all available projects.",
                    query
                );
                std::process::exit(2);
            }
        }

        return Ok(());
    }

    let effective_query = if let Some(q) = args.query {
        Some(q)
    } else {
        let projects = builder.list_projects()?;
        if projects.len() == 1 && !args.json && !args.json_stream {
            Some(projects[0].name.clone())
        } else {
            if args.json_stream {
                emit_json_stream_event(
                    "loadout_start",
                    serde_json::json!({
                        "mode": "list",
                    }),
                );
                for p in &projects {
                    emit_json_stream_event(
                        "project_listed",
                        serde_json::json!({
                            "name": p.name,
                            "status": p.status,
                            "path": p.path,
                        }),
                    );
                }
                emit_json_stream_event(
                    "loadout_ready",
                    serde_json::json!({
                        "total_projects": projects.len(),
                    }),
                );
            } else if args.json {
                let json_str = serde_json::to_string_pretty(&projects)?;
                safe_println(&json_str);
                if args.copy {
                    copy_and_notify(&json_str);
                }
            } else if vault_path.join("00_Daily").is_dir() {
                safe_println("📂 Available Projects:");
                safe_println("--------------------------------------------------");
                for p in &projects {
                    safe_println(&format!("  • {}  [{}] ({})", p.name, p.status, p.path));
                }
                safe_println("--------------------------------------------------");
                safe_println("💡 Usage: k0maru loadout <project-keyword>");
                if args.copy {
                    let text = projects
                        .iter()
                        .map(|p| format!("  • {}  [{}] ({})", p.name, p.status, p.path))
                        .collect::<Vec<_>>()
                        .join("\n");
                    copy_and_notify(&text);
                }
            } else {
                safe_println("📂 Available Projects:");
                safe_println("--------------------------------------------------");
                for p in &projects {
                    safe_println(&format!("  • {}  [{}] ({})", p.name, p.status, p.path));
                }
                safe_println("--------------------------------------------------");
                safe_println("💡 Usage: k0maru loadout <project-keyword>");
                if args.copy {
                    let text = projects
                        .iter()
                        .map(|p| format!("  • {}  [{}] ({})", p.name, p.status, p.path))
                        .collect::<Vec<_>>()
                        .join("\n");
                    copy_and_notify(&text);
                }
            }
            return Ok(());
        }
    };

    if let Some(query) = effective_query {
        match builder.build(&query)? {
            Some(loadout) => {
                if args.json_stream {
                    emit_json_stream_event(
                        "loadout_start",
                        serde_json::json!({
                            "project": query,
                            "mode": "standard",
                        }),
                    );
                    for card in &loadout.l3_cards {
                        emit_json_stream_event(
                            "rule_activated",
                            serde_json::json!({
                                "rule": card.name,
                                "concept": card.concept.clone().unwrap_or_default(),
                            }),
                        );
                    }
                    emit_json_stream_event(
                        "loadout_ready",
                        serde_json::json!({
                            "tokens": loadout.estimated_tokens,
                            "within_budget": loadout.estimated_tokens <= 300,
                            "markdown": loadout.markdown,
                        }),
                    );
                } else if args.json {
                    let output = serde_json::to_string_pretty(&loadout)?;
                    safe_println(&output);
                    if args.copy {
                        copy_and_notify(&output);
                    }
                } else {
                    safe_println(&loadout.markdown);
                    if args.copy {
                        copy_and_notify(&loadout.markdown);
                    }
                }
            }
            None => {
                eprintln!(
                    "❌ No project matched '{}'. Use '--list' to see all available projects.",
                    query
                );
                std::process::exit(2);
            }
        }
    }

    Ok(())
}

fn run_loadout(args: LoadoutArgs) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = detect_vault_path(args.vault.clone());
    if vault_path.join("10_Projects").is_dir() || vault_path.join(".obsidian").exists() {
        let adapter = ObsidianAdapter::new(&vault_path);
        run_loadout_with_adapter(&adapter, &vault_path, args)
    } else {
        let adapter = GenericWikiAdapter::new(&vault_path);
        run_loadout_with_adapter(&adapter, &vault_path, args)
    }
}

fn run_offload(args: OffloadArgs) -> Result<(), Box<dyn std::error::Error>> {
    let is_terminal = std::io::stdin().is_terminal()
        || std::env::var("K0MARU_FORCE_TTY")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false);

    if is_terminal {
        eprintln!("❌ Error: no log input provided and stdin is an interactive terminal (TTY).");
        eprintln!("💡 Usage: cat log.txt | k0maru offload");
        std::process::exit(2);
    }

    let refs_dir = detect_refs_dir(args.refs_dir);
    let mut engine = OffloadEngine::new(refs_dir).with_threshold(args.threshold);
    if let Some(task_id) = args.task_id {
        engine = engine.with_task_id(task_id);
    }

    let stdin = std::io::stdin();
    let reader = stdin.lock();
    let result = engine.process_stream(reader)?;

    if args.json_stream {
        emit_json_stream_event(
            "stream_chunk",
            serde_json::json!({
                "lines_processed": result.total_lines,
            }),
        );
        if result.truncated {
            emit_json_stream_event(
                "threshold_exceeded",
                serde_json::json!({
                    "threshold": args.threshold,
                    "action": "offloading",
                }),
            );
            emit_json_stream_event(
                "offload_complete",
                serde_json::json!({
                    "node_id": result.node_id,
                    "total_lines": result.total_lines,
                    "truncated": true,
                    "log_path": result.log_path.as_ref().map(|p| p.to_string_lossy().to_string()),
                    "mermaid_graph": result.mermaid_graph,
                }),
            );
        } else {
            emit_json_stream_event(
                "offload_complete",
                serde_json::json!({
                    "total_lines": result.total_lines,
                    "truncated": false,
                }),
            );
        }
    } else if result.truncated {
        safe_println(&result.mermaid_graph);
        safe_println(&result.summary_text);
    } else if !result.summary_text.is_empty() {
        safe_println(&result.summary_text);
    }

    Ok(())
}

fn run_inspect(args: InspectArgs) -> Result<(), Box<dyn std::error::Error>> {
    let refs_dir = detect_refs_dir(args.refs_dir);
    match inspect_node(&refs_dir, &args.node_id) {
        Ok(content) => {
            safe_print(&content);
            if !content.ends_with('\n') {
                safe_println("");
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Log node '{}' not found: {}", args.node_id, e);
            std::process::exit(2);
        }
    }
}

fn run_mcp(args: McpArgs) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = detect_vault_path(args.vault);
    let mut server =
        k0maru::mcp::McpServer::new_with_model(&vault_path, args.embed_model.as_deref())
            .with_sidecar(args.sidecar);
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    server.run_stdio(stdin.lock(), stdout.lock())
}

fn run_sync(args: SyncArgs) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = detect_vault_path(args.vault.clone());
    let cache_dir = vault_path.join(".k0maru");
    let cache_path = cache_dir.join("cache.sqlite");

    let mut storage = match SqliteStorage::open(&cache_path) {
        Ok(s) => s,
        Err(_) => SqliteStorage::in_memory()?,
    };

    let embedder: Option<std::sync::Arc<dyn k0maru::vector::EmbeddingEngine>> = if args.vector {
        let model_cache = if vault_path.join(".k0maru").join("models").exists() {
            Some(vault_path.join(".k0maru").join("models"))
        } else {
            None
        };
        let engine = resolve_embedding_engine(args.embed_model.as_deref(), model_cache)?;
        let migrated = storage.ensure_vector_dimension(engine.dimension())?;
        if migrated && !args.json {
            safe_println(&format!(
                "🔄 Vector dimension migration triggered: table recreated for model '{}' ({} dimensions)",
                engine.model_name(),
                engine.dimension()
            ));
        }
        Some(engine)
    } else {
        None
    };

    let (stats, vec_stats) =
        if vault_path.join("10_Projects").is_dir() || vault_path.join(".obsidian").exists() {
            let adapter = ObsidianAdapter::new(&vault_path);
            let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
            if args.force {
                let sync_stats = scanner.sync(true)?;
                let vec_stats = if let Some(ref engine) = embedder {
                    k0maru::scanner::VectorSyncEngine::sync(scanner.storage_mut(), &**engine)?
                } else {
                    k0maru::scanner::VectorSyncStats::default()
                };
                (sync_stats, vec_stats)
            } else {
                scanner.sync_vault_with_vector(&vault_path, embedder)?
            }
        } else {
            let adapter = GenericWikiAdapter::new(&vault_path);
            let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
            if args.force {
                let sync_stats = scanner.sync(true)?;
                let vec_stats = if let Some(ref engine) = embedder {
                    k0maru::scanner::VectorSyncEngine::sync(scanner.storage_mut(), &**engine)?
                } else {
                    k0maru::scanner::VectorSyncStats::default()
                };
                (sync_stats, vec_stats)
            } else {
                scanner.sync_vault_with_vector(&vault_path, embedder)?
            }
        };

    if args.json {
        if args.vector {
            let json_str = serde_json::to_string_pretty(&serde_json::json!({
                "cache": stats,
                "vector": vec_stats,
            }))?;
            println!("{}", json_str);
        } else {
            let json_str = serde_json::to_string_pretty(&stats)?;
            println!("{}", json_str);
        }
    } else if args.vector {
        println!(
            "⚡ Vault synced in {}ms (added: {}, modified: {}, deleted: {}, unchanged: {}) | Vector (embedded: {}, deleted: {}, skipped: {})",
            stats.duration_ms, stats.added, stats.modified, stats.deleted, stats.unchanged,
            vec_stats.embedded_count, vec_stats.deleted_count, vec_stats.skipped_count
        );
    } else {
        println!(
            "⚡ Vault synced in {}ms (added: {}, modified: {}, deleted: {}, unchanged: {})",
            stats.duration_ms, stats.added, stats.modified, stats.deleted, stats.unchanged
        );
    }

    Ok(())
}

fn handle_search(args: SearchArgs) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;

    let query = if args.query == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        let trimmed = buf.trim().to_string();
        if trimmed.is_empty() {
            eprintln!("❌ Error: piped search query is empty.");
            std::process::exit(2);
        }
        trimmed
    } else {
        args.query
    };

    let mode = match args.mode.to_lowercase().as_str() {
        "hybrid" => SearchMode::Hybrid,
        "bm25" => SearchMode::Bm25,
        "vector" => SearchMode::Vector,
        other => {
            eprintln!(
                "❌ Invalid search mode '{}'. Supported modes: hybrid, bm25, vector",
                other
            );
            std::process::exit(2);
        }
    };

    let vault_path = detect_vault_path(args.vault.clone());
    if !vault_path.exists() {
        return Err(format!("Vault path does not exist: {}", vault_path.display()).into());
    }

    let cache_dir = vault_path.join(".k0maru");
    let cache_path = cache_dir.join("cache.sqlite");

    std::fs::create_dir_all(&cache_dir)?;
    let mut storage = SqliteStorage::open(&cache_path)?;
    let _ = storage.initialize();

    // Keep the index fresh (and self-heal a deleted `cache.sqlite`) before
    // answering, so `k0maru search` always sees newly written markdown notes.
    if vault_path.join("10_Projects").is_dir() || vault_path.join(".obsidian").exists() {
        let adapter = ObsidianAdapter::new(&vault_path);
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        let _ = scanner.sync(false);
    } else {
        let adapter = GenericWikiAdapter::new(&vault_path);
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        let _ = scanner.sync(false);
    }

    if mode == SearchMode::Vector && storage.vector_count().unwrap_or(0) == 0 {
        return Err("Vector index is empty. Run 'k0maru sync --vector' first.".into());
    }
    let has_vectors = storage.has_vectors().unwrap_or(false);
    let embedder = if mode != SearchMode::Bm25 && has_vectors {
        let model_cache = if vault_path.join(".k0maru").join("models").exists() {
            Some(vault_path.join(".k0maru").join("models"))
        } else {
            None
        };
        resolve_embedding_engine(args.embed_model.as_deref(), model_cache).ok()
    } else {
        None
    };
    let engine = HybridSearchEngine::new(&storage, embedder);

    let results = engine.search(&query, mode, args.limit)?;

    if args.json_stream {
        emit_json_stream_event(
            "search_start",
            serde_json::json!({
                "query": query,
                "mode": args.mode,
                "limit": args.limit,
            }),
        );
        for (i, r) in results.iter().enumerate() {
            emit_json_stream_event(
                "search_hit",
                serde_json::json!({
                    "rank": i + 1,
                    "score": r.score,
                    "title": r.title,
                    "path": r.path,
                    "snippet": r.snippet,
                }),
            );
        }
        emit_json_stream_event(
            "search_complete",
            serde_json::json!({
                "total_hits": results.len(),
            }),
        );
    } else if args.json {
        let json_str = serde_json::to_string_pretty(&results)?;
        safe_println(&json_str);
    } else {
        if results.is_empty() {
            safe_println(&format!("🔍 No results found matching query: '{}'", query));
            return Ok(());
        }

        safe_println(&format!(
            "🔍 Search Results for '{}' ({} results, mode: {}):",
            query,
            results.len(),
            args.mode
        ));
        safe_println("--------------------------------------------------");
        for (i, r) in results.iter().enumerate() {
            let rank = i + 1;
            safe_println(&format!(
                "#{:<2} [{:.4}] {} ({})",
                rank, r.score, r.title, r.path
            ));
            if !r.snippet.is_empty() {
                safe_println(&format!("    {}", r.snippet));
            }
        }
        safe_println("--------------------------------------------------");
    }

    Ok(())
}

fn run_search(args: SearchArgs) -> Result<(), Box<dyn std::error::Error>> {
    handle_search(args)
}

fn run_doctor(args: DoctorArgs) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = detect_vault_path(args.vault);
    let report = run_diagnostics(&vault_path, args.home.as_deref());

    if args.json {
        let json_str = serde_json::to_string_pretty(&report)?;
        println!("{}", json_str);
    } else {
        println!("{}", format_report(&report));
    }

    if report.summary.failures > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn run_init_cmd(args: InitArgs) -> Result<(), Box<dyn std::error::Error>> {
    let repo_path = args
        .repo
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let options = k0maru::bootstrap::BootstrapOptions {
        repo_path,
        vault_path: args.vault,
        project_name: args.project,
        install_clients: args.install,
        home_override: args.home,
    };

    let report = k0maru::bootstrap::run_bootstrap(options)?;

    if args.json {
        let json_str = serde_json::to_string_pretty(&report)?;
        println!("{}", json_str);
    } else {
        println!("{}", k0maru::bootstrap::format_bootstrap_report(&report));
    }

    Ok(())
}

fn run_install_cmd(args: InstallArgs) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = detect_vault_path(args.vault);
    let target: InstallTarget = args.target.parse()?;

    let options = InstallOptions {
        vault_path: vault_path.clone(),
        target,
        dry_run: args.dry_run,
        home_override: args.home,
    };

    let report = run_install(options).map_err(|e| e as Box<dyn std::error::Error>)?;

    let mut guard_hook = None;
    let mut claude_hook = None;
    if args.hooks && !args.dry_run {
        let repo_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let project_slug = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(k0maru::convention::slugify)
            .unwrap_or_else(|| "project".to_string());
        let (gh, ch) = k0maru::install::install_repo_hooks(&repo_path, &vault_path, &project_slug);
        guard_hook = gh;
        claude_hook = ch;
    }

    if args.json {
        let mut val = serde_json::to_value(&report)?;
        if args.hooks {
            if let Some(obj) = val.as_object_mut() {
                obj.insert(
                    "guard_hook_installed".to_string(),
                    serde_json::to_value(&guard_hook)?,
                );
                obj.insert(
                    "claude_hook_installed".to_string(),
                    serde_json::to_value(&claude_hook)?,
                );
            }
        }
        let json_str = serde_json::to_string_pretty(&val)?;
        println!("{}", json_str);
    } else {
        println!("{}", format_install_report(&report));
        if args.hooks {
            if let Some(gh) = &guard_hook {
                println!("⚓ Git guard hook installed: {}", gh.display());
            }
            if let Some(ch) = &claude_hook {
                println!("🤖 Claude session hook installed: {}", ch.display());
            }
        }
    }

    Ok(())
}

fn run_flush(args: FlushArgs) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = detect_vault_path(args.vault);

    let content = if let Some(c) = args.content {
        c
    } else if !std::io::stdin().is_terminal() {
        let mut buffer = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buffer)?;
        buffer
    } else {
        String::new()
    };

    let engine = FlushEngine::new(vault_path).map_err(|e| e as Box<dyn std::error::Error>)?;

    let request = FlushRequest {
        title: args.title,
        summary: args.summary,
        content,
        category: args.category,
        tags: args.tags,
        related_notes: args.related,
        dry_run: args.dry_run,
        supersedes: args.supersedes,
    };

    let result = engine
        .flush(request)
        .map_err(|e| e as Box<dyn std::error::Error>)?;

    if args.json {
        let json_str = serde_json::to_string_pretty(&result)?;
        println!("{}", json_str);
    } else if result.dry_run {
        println!("🔍 [Dry Run] Note preview (no file written):");
        println!("Path: {}", result.file_path.display());
        println!("Relative: {}", result.relative_path.display());
        println!("Category: {}", result.category);
        println!("--------------------------------------------------");
        println!("{}", result.content_preview);
        println!("--------------------------------------------------");
    } else if !result.created {
        println!(
            "ℹ️  Note content identical to existing file; no write needed: {}",
            result.relative_path.display()
        );
    } else {
        println!(
            "✓ Crystallized note successfully into: {}",
            result.relative_path.display()
        );
        println!("Path: {}", result.file_path.display());
        println!("Category: {}", result.category);
        if !result.superseded_files.is_empty() {
            let list = result
                .superseded_files
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!("Superseded: {}", list);
        }
    }

    Ok(())
}

fn run_distill(args: DistillArgs) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = detect_vault_path(args.vault);
    let refs_dir = detect_refs_dir(args.refs_dir);

    let target = if args.export_hermes && args.target == "default" {
        DistillTarget::Hermes
    } else if args.export_omp && args.target == "default" {
        DistillTarget::Omp
    } else {
        std::str::FromStr::from_str(&args.target)?
    };

    let engine = DistillEngine::new(&vault_path, &refs_dir);
    let opts = DistillOptions {
        title: args.title,
        context_hint: args.context,
        category: Some(args.category),
        tags: args.tags,
        related_notes: args.related,
        dry_run: args.dry_run,
        target,
        home_override: args.home.clone(),
    };

    let mut result = if let Some(ref nid) = args.node {
        engine
            .distill_node(nid, opts)
            .map_err(|e| e as Box<dyn std::error::Error>)?
    } else if let Some(ref fpath) = args.file {
        engine
            .distill_file(fpath, opts)
            .map_err(|e| e as Box<dyn std::error::Error>)?
    } else {
        use std::io::Read;
        let mut raw_trace = String::new();
        std::io::stdin().read_to_string(&mut raw_trace)?;
        if raw_trace.trim().is_empty() {
            return Err("No trace input provided via stdin, --node, or --file".into());
        }
        engine
            .distill_text(&raw_trace, opts)
            .map_err(|e| e as Box<dyn std::error::Error>)?
    };

    if args.export_hermes {
        let hermes_dir = detect_hermes_dir(args.home.as_deref());
        let skills_dir = hermes_dir.join("skills");
        let slug = k0maru::convention::slugify(&result.skill.title);
        let filename = format!(
            "{}.md",
            if slug.is_empty() {
                "distilled-skill"
            } else {
                &slug
            }
        );
        let dest = skills_dir.join(&filename);

        if !args.dry_run {
            std::fs::create_dir_all(&skills_dir)?;
            let hermes_md = result.skill.to_hermes_markdown();
            std::fs::write(&dest, hermes_md)?;
        }
        result.export_path = Some(dest);
    }

    if args.json {
        let json_str = serde_json::to_string_pretty(&result)?;
        println!("{}", json_str);
    } else if result.flush_result.dry_run {
        println!("🔍 [Dry Run] Distilled skill preview (no file written):");
        println!("Title: {}", result.skill.title);
        println!("Target: {}", target);
        println!("Path: {}", result.flush_result.file_path.display());
        println!("Relative: {}", result.flush_result.relative_path.display());
        println!("Category: {}", result.flush_result.category);
        if let Some(ep) = &result.export_path {
            if target == DistillTarget::Omp {
                println!("omp Export: {} (simulated)", ep.display());
            } else {
                println!("Hermes Export: {} (simulated)", ep.display());
            }
        }
        println!("--------------------------------------------------");
        println!("{}", result.preview_markdown);
        println!("--------------------------------------------------");
    } else if !result.flush_result.created {
        println!(
            "ℹ️  Skill content identical to existing note; no write needed: {}",
            result.flush_result.relative_path.display()
        );
        if let Some(ep) = &result.export_path {
            if target == DistillTarget::Omp {
                println!("✓ Exported omp skill to: {}", ep.display());
            } else {
                println!("✓ Exported Hermes dynamic skill to: {}", ep.display());
            }
        }
    } else {
        println!(
            "✓ Crystallized skill note successfully into: {}",
            result.flush_result.relative_path.display()
        );
        println!("Title: {}", result.skill.title);
        println!("Target: {}", target);
        println!("Path: {}", result.flush_result.file_path.display());
        println!("Category: {}", result.flush_result.category);
        if let Some(ep) = &result.export_path {
            if target == DistillTarget::Omp {
                println!("✓ Exported omp skill to: {}", ep.display());
            } else {
                println!("✓ Exported Hermes dynamic skill to: {}", ep.display());
            }
        }
    }

    Ok(())
}

fn run_gc(args: GcArgs) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;

    let input_str = if let Some(ref path) = args.file {
        if !path.exists() {
            eprintln!("❌ Error: trajectory file not found: {}", path.display());
            std::process::exit(2);
        }
        std::fs::read_to_string(path)?
    } else if !std::io::stdin().is_terminal() {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        buffer
    } else {
        eprintln!(
            "❌ Error: no trajectory file provided and stdin is not piped. Usage: k0maru gc <FILE> or pipe input: cat session.json | k0maru gc"
        );
        std::process::exit(2);
    };

    if input_str.trim().is_empty() {
        eprintln!("❌ Error: session trajectory input is empty");
        std::process::exit(1);
    }

    let trajectory: k0maru::gc::SessionTrajectory = serde_json::from_str(&input_str)
        .map_err(|e| format!("Failed to parse session trajectory JSON: {}", e))?;

    let options = k0maru::gc::GcOptions {
        line_threshold: args.threshold,
        hot_window_turns: args.hot_window,
        context_window_tokens: args.context_window,
        ..Default::default()
    };

    let engine = k0maru::gc::ContextGcEngine::new();
    let (result, events) = engine.compact_with_events(&trajectory, &options)?;

    if args.json_stream {
        for event in events {
            safe_println(&serde_json::to_string(&event)?);
        }
    } else if args.json {
        safe_println(&serde_json::to_string_pretty(&result)?);
    } else {
        safe_println("🧹 Session Context GC Compression Report:");
        safe_println(&format!("• Session ID:         {}", trajectory.session_id));
        safe_println(&format!("• Raw Tokens:         {}", result.raw_tokens));
        safe_println(&format!(
            "• Compacted Tokens:   {}",
            result.compacted_tokens
        ));
        safe_println(&format!(
            "• Token Reduction:    {:.2}% (TRR >= 85%: {})",
            result.token_reduction_ratio,
            if result.token_reduction_ratio >= 85.0 {
                "✅ yes"
            } else {
                "⚠️ no"
            }
        ));
        safe_println(&format!(
            "• Smart-Zone Usage:   {:.1}% (range: 15% - 60%, compliant: {})",
            result.smart_zone_occupancy * 100.0,
            if result.in_smart_zone {
                "✅ yes"
            } else {
                "⚠️ no"
            }
        ));
        safe_println(&format!(
            "• Turns:              raw {} / compacted {} (folded/offloaded {})",
            result.raw_turns_count, result.compacted_turns_count, result.folded_nodes_count
        ));
    }

    Ok(())
}

fn run_pack(args: PackArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        PackSubcommand::Export(export_args) => {
            let vault_path = detect_vault_path(export_args.vault);
            let options = PackExportOptions {
                vault_path,
                name: export_args.name,
                version: export_args.version,
                description: export_args.description,
                tag: export_args.tag,
                query: export_args.query,
                files: export_args.files,
            };
            let pack = export_pack(&options)?;
            let json_str = serde_json::to_string_pretty(&pack)?;

            if let Some(out_path) = &export_args.output {
                if let Some(parent) = out_path.parent() {
                    if !parent.as_os_str().is_empty() {
                        std::fs::create_dir_all(parent)?;
                    }
                }
                std::fs::write(out_path, &json_str)?;
                if export_args.json {
                    println!("{}", json_str);
                } else {
                    println!(
                        "📦 Exported memory pack '{}' (v{}) to {}",
                        pack.name,
                        pack.version,
                        out_path.display()
                    );
                    println!("   Notes: {}", pack.notes.len());
                    println!("   Guard rules: {}", pack.guard_rules.len());
                }
            } else {
                println!("{}", json_str);
            }
            Ok(())
        }
        PackSubcommand::Import(import_args) => {
            let content = std::fs::read_to_string(&import_args.pack_file).map_err(|e| {
                format!(
                    "Failed to read pack file '{}': {}",
                    import_args.pack_file.display(),
                    e
                )
            })?;
            let pack: MemoryPack = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse pack file as MemoryPack JSON: {}", e))?;
            let vault_path = detect_vault_path(import_args.vault);
            let options = PackImportOptions {
                vault_path,
                dry_run: import_args.dry_run,
            };
            let result = import_pack(&pack, &options)?;

            if import_args.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let dry_str = if result.dry_run { " [dry-run]" } else { "" };
                println!("📦 Imported memory pack '{}'{}:", result.pack_name, dry_str);
                println!("   Vault path: {}", result.vault_path.display());
                println!("   Notes imported: {}", result.imported_notes.len());
                for item in &result.imported_notes {
                    let status = if item.dry_run {
                        "preview"
                    } else if item.created {
                        "created"
                    } else {
                        "updated"
                    };
                    println!(
                        "     - {} [{}] -> {} ({})",
                        item.title,
                        item.category,
                        item.relative_path.display(),
                        status
                    );
                }
                println!("   Guard rules active: {}", result.guard_rules_count);
            }
            Ok(())
        }
        PackSubcommand::Inspect(inspect_args) => {
            let content = std::fs::read_to_string(&inspect_args.pack_file).map_err(|e| {
                format!(
                    "Failed to read pack file '{}': {}",
                    inspect_args.pack_file.display(),
                    e
                )
            })?;
            let pack: MemoryPack = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse pack file as MemoryPack JSON: {}", e))?;
            let report = inspect_pack(&pack, inspect_args.vault.as_deref());

            if inspect_args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("📦 Memory Pack: {} (v{})", report.name, report.version);
                if let Some(desc) = &report.description {
                    println!("   Description: {}", desc);
                }
                println!("   Notes count: {}", report.notes_count);
                println!("   Guard rules count: {}", report.guard_rules_count);
                if !report.note_titles.is_empty() {
                    println!("   Notes:");
                    for (i, title) in report.note_titles.iter().enumerate() {
                        if let Some(predicted) = report.predicted_paths.get(i) {
                            println!("     - {} -> {}", title, predicted.display());
                        } else {
                            println!("     - {}", title);
                        }
                    }
                }
            }
            Ok(())
        }
        PackSubcommand::Install(install_args) => {
            let vault_path = detect_vault_path(install_args.vault);
            let options = PackInstallOptions {
                vault_path,
                source_path: install_args.source,
                home_override: install_args.home,
                dry_run: install_args.dry_run,
            };
            let result = install_pack(&options)?;
            if install_args.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let dry_str = if result.dry_run { " [dry-run]" } else { "" };
                println!(
                    "📦 Installed pack '{}' ({}){}",
                    result.manifest.name, result.manifest.source_kind, dry_str
                );
                println!("   Install dir: {}", result.install_dir.display());
                println!("   Version: {}", result.manifest.version);
                if let Some(desc) = &result.manifest.description {
                    println!("   Description: {}", desc);
                }
                println!("   Notes: {}", result.manifest.notes_count);
                println!("   Guard rules: {}", result.manifest.guard_rules_count);
                if !result.skill_paths.is_empty() {
                    println!("   Mounted skills: {}", result.skill_paths.len());
                    for sp in &result.skill_paths {
                        println!("     - {}", sp.display());
                    }
                }
            }
            Ok(())
        }
        PackSubcommand::Uninstall(uninstall_args) => {
            let vault_path = detect_vault_path(uninstall_args.vault);
            let result = uninstall_pack(
                &vault_path,
                &uninstall_args.name,
                uninstall_args.home.as_deref(),
            )?;
            if uninstall_args.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("🗑️  Uninstalled pack '{}'", result.name);
                println!("   Removed dir: {}", result.removed_dir.display());
                if !result.removed_skills.is_empty() {
                    println!("   Removed skills: {}", result.removed_skills.len());
                    for skill in &result.removed_skills {
                        println!("     - {}", skill);
                    }
                }
            }
            Ok(())
        }
        PackSubcommand::List(list_args) => {
            let vault_path = detect_vault_path(list_args.vault);
            let packs = list_installed_packs(&vault_path)?;
            if list_args.json {
                println!("{}", serde_json::to_string_pretty(&packs)?);
            } else if packs.is_empty() {
                println!("No packs currently installed in {}", vault_path.display());
            } else {
                println!("📦 Installed packs in {}:", vault_path.display());
                for p in &packs {
                    println!(
                        "   - {} (v{}, source: {}, notes: {}, rules: {}, skills: {})",
                        p.name,
                        p.version,
                        p.source_kind,
                        p.notes_count,
                        p.guard_rules_count,
                        p.installed_skills.len()
                    );
                    if let Some(desc) = &p.description {
                        println!("       {}", desc);
                    }
                }
            }
            Ok(())
        }
    }
}

fn main() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            ""
        };
        if payload.contains("Broken pipe") || payload.contains("failed printing to stdout") {
            std::process::exit(0);
        }
        default_hook(info);
    }));

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Loadout(args)) => {
            if let Err(e) = run_loadout(args) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Offload(args)) => {
            if let Err(e) = run_offload(args) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Inspect(args)) => {
            if let Err(e) = run_inspect(args) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Mcp(args)) => {
            if let Err(e) = run_mcp(args) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Sync(args)) => {
            if let Err(e) = run_sync(args) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Search(args)) => {
            if let Err(e) = run_search(args) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Doctor(args)) => {
            if let Err(e) = run_doctor(args) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Init(args)) => {
            if let Err(e) = run_init_cmd(args) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Install(args)) => {
            if let Err(e) = run_install_cmd(args) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Flush(args)) => {
            if let Err(e) = run_flush(args) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Distill(args)) => {
            if let Err(e) = run_distill(args) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Gc(args)) => {
            if let Err(e) = run_gc(args) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Guard(args)) => match args.command {
            GuardSubcommand::Check(check_args) => {
                let is_json = check_args.json;
                let is_stop_hook = check_args.claude_stop_hook;
                let vault_path = detect_vault_path(check_args.vault.clone());
                match run_guard_check(check_args, &vault_path) {
                    Ok(report) => {
                        if is_stop_hook {
                            // Claude Code `Stop` hook: failures already wrote
                            // the report to stderr and exited 2 inside
                            // `run_guard_check`; success stays silent.
                            std::process::exit(0);
                        }
                        if is_json {
                            if let Ok(json_str) = serde_json::to_string_pretty(&report) {
                                println!("{}", json_str);
                            }
                        } else {
                            print!("{}", format_guard_report(&report));
                        }
                        if !report.is_passed {
                            std::process::exit(1);
                        } else {
                            std::process::exit(0);
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Error: {}", e);
                        std::process::exit(2);
                    }
                }
            }
            GuardSubcommand::Install(install_args) => match run_guard_install(install_args) {
                Ok(hook_path) => {
                    println!(
                        "✅ Successfully installed k0maru guard hook: {}",
                        hook_path.display()
                    );
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(2);
                }
            },
        },
        Some(Commands::Ui {
            vault,
            port,
            open,
            embed_model: _,
        }) => {
            let vault_path = detect_vault_path(vault);
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = rt.block_on(k0maru::ui::run_server(vault_path, port, open)) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Pack(args)) => {
            if let Err(e) = run_pack(args) {
                eprintln!("❌ Error: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            // Default when no subcommand is provided
        }
    }
}
