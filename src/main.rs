use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand};

use k0maru::adapters::{GenericWikiAdapter, ObsidianAdapter};
use k0maru::convention::{FlushEngine, FlushRequest};
use k0maru::core::traits::VaultAdapter;
use k0maru::distill::{DistillEngine, DistillOptions};
use k0maru::doctor::{format_report, run_diagnostics};
use k0maru::install::{format_install_report, run_install, InstallOptions, InstallTarget};
use k0maru::loadout::{copy_to_clipboard, LoadoutBuilder};
use k0maru::offload::{inspect_node, OffloadEngine};
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::{HybridSearchEngine, SearchMode, SqliteStorage};
use k0maru::vector::default_embedding_engine;

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

    /// List all active projects in the vault
    #[arg(short, long)]
    pub list: bool,
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
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search query string
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
}

#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Path to vault root directory (defaults to current dir or detects nearest vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Output structured JSON instead of human-readable report
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Path to vault root directory (defaults to current dir or detects nearest vault)
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Target client to configure: all, claude, cursor, gemini, windsurf, cline
    #[arg(short, long, default_value = "all")]
    pub target: String,

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

    /// Category: decision, log, concept, etc.
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

    /// Preview the generated skill note without writing to disk
    #[arg(long)]
    pub dry_run: bool,

    /// Output structured JSON result
    #[arg(long)]
    pub json: bool,
}

fn detect_vault_path(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(p) = explicit {
        return p;
    }
    if let Ok(mut current) = std::env::current_dir() {
        loop {
            if current.join(".k0maru").exists()
                || current.join(".obsidian").exists()
                || current.join("10_Projects").is_dir()
                || current.join("20_Cards").is_dir()
            {
                return current;
            }
            if !current.pop() {
                break;
            }
        }
    }
    PathBuf::from(".")
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
    let _ = write!(handle, "{}", msg);
}

fn safe_println(msg: &str) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(handle, "{}", msg);
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

    if args.list || args.query.is_none() {
        let projects = builder.list_projects()?;
        if args.json {
            let json_str = serde_json::to_string_pretty(&projects)?;
            println!("{}", json_str);
            if args.copy {
                copy_and_notify(&json_str);
            }
        } else {
            println!("📂 当前项目列表：");
            println!("--------------------------------------------------");
            for p in &projects {
                println!("  • {}  [{}] ({})", p.name, p.status, p.path);
            }
            println!("--------------------------------------------------");
            println!("💡 用法: k0maru loadout <项目关键词>");
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

    if let Some(query) = args.query {
        match builder.build(&query)? {
            Some(loadout) => {
                let output = if args.json {
                    serde_json::to_string_pretty(&loadout)?
                } else {
                    loadout.markdown
                };
                println!("{}", output);
                if args.copy {
                    copy_and_notify(&output);
                }
            }
            None => {
                eprintln!(
                    "❌ 未找到匹配 '{}' 的工程项目。可使用 --list 查看所有项目。",
                    query
                );
                std::process::exit(1);
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
    let refs_dir = detect_refs_dir(args.refs_dir);
    let mut engine = OffloadEngine::new(refs_dir).with_threshold(args.threshold);
    if let Some(task_id) = args.task_id {
        engine = engine.with_task_id(task_id);
    }

    let stdin = std::io::stdin();
    let reader = stdin.lock();
    let result = engine.process_stream(reader)?;

    if result.truncated {
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
    let mut server = k0maru::mcp::McpServer::new(&vault_path);
    let model_cache = if vault_path.join(".k0maru").join("models").exists() {
        Some(vault_path.join(".k0maru").join("models"))
    } else {
        None
    };
    if let Ok(embedder) = default_embedding_engine(model_cache) {
        server = server.with_embedder(embedder);
    }
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
        Some(default_embedding_engine(model_cache)?)
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
    let mode = match args.mode.to_lowercase().as_str() {
        "hybrid" => SearchMode::Hybrid,
        "bm25" => SearchMode::Bm25,
        "vector" => SearchMode::Vector,
        other => {
            return Err(format!(
                "Invalid search mode '{}'. Supported modes: hybrid, bm25, vector",
                other
            )
            .into());
        }
    };

    let vault_path = detect_vault_path(args.vault.clone());
    if !vault_path.exists() {
        return Err(format!("Vault path does not exist: {}", vault_path.display()).into());
    }

    let cache_dir = vault_path.join(".k0maru");
    let cache_path = cache_dir.join("cache.sqlite");

    let storage = SqliteStorage::open(&cache_path)?;
    let model_cache = if vault_path.join(".k0maru").join("models").exists() {
        Some(vault_path.join(".k0maru").join("models"))
    } else {
        None
    };
    let embedder = default_embedding_engine(model_cache).ok();
    let engine = HybridSearchEngine::new(&storage, embedder);

    let results = engine.search(&args.query, mode, args.limit)?;

    if args.json {
        let json_str = serde_json::to_string_pretty(&results)?;
        println!("{}", json_str);
    } else {
        if results.is_empty() {
            println!("🔍 No results found matching query: '{}'", args.query);
            return Ok(());
        }

        println!(
            "🔍 Search Results for '{}' ({} results, mode: {}):",
            args.query,
            results.len(),
            args.mode
        );
        println!("--------------------------------------------------");
        for (i, r) in results.iter().enumerate() {
            let rank = i + 1;
            println!("#{:<2} [{:.4}] {} ({})", rank, r.score, r.title, r.path);
            if !r.snippet.is_empty() {
                println!("    {}", r.snippet);
            }
        }
        println!("--------------------------------------------------");
    }

    Ok(())
}

fn run_search(args: SearchArgs) -> Result<(), Box<dyn std::error::Error>> {
    handle_search(args)
}

fn run_doctor(args: DoctorArgs) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = detect_vault_path(args.vault);
    let report = run_diagnostics(&vault_path, None);

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

fn run_install_cmd(args: InstallArgs) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = detect_vault_path(args.vault);
    let target: InstallTarget = args.target.parse()?;

    let options = InstallOptions {
        vault_path,
        target,
        dry_run: args.dry_run,
        home_override: None,
    };

    let report = run_install(options).map_err(|e| e as Box<dyn std::error::Error>)?;

    if args.json {
        let json_str = serde_json::to_string_pretty(&report)?;
        println!("{}", json_str);
    } else {
        println!("{}", format_install_report(&report));
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
    }

    Ok(())
}

fn run_distill(args: DistillArgs) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = detect_vault_path(args.vault);
    let refs_dir = detect_refs_dir(args.refs_dir);

    let engine = DistillEngine::new(&vault_path, &refs_dir);
    let opts = DistillOptions {
        title: args.title,
        context_hint: args.context,
        category: Some(args.category),
        tags: args.tags,
        related_notes: args.related,
        dry_run: args.dry_run,
    };

    let result = if let Some(ref nid) = args.node {
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

    if args.json {
        let json_str = serde_json::to_string_pretty(&result)?;
        println!("{}", json_str);
    } else if result.flush_result.dry_run {
        println!("🔍 [Dry Run] Distilled skill preview (no file written):");
        println!("Title: {}", result.skill.title);
        println!("Path: {}", result.flush_result.file_path.display());
        println!("Relative: {}", result.flush_result.relative_path.display());
        println!("Category: {}", result.flush_result.category);
        println!("--------------------------------------------------");
        println!("{}", result.preview_markdown);
        println!("--------------------------------------------------");
    } else if !result.flush_result.created {
        println!(
            "ℹ️  Skill content identical to existing note; no write needed: {}",
            result.flush_result.relative_path.display()
        );
    } else {
        println!(
            "✓ Crystallized skill note successfully into: {}",
            result.flush_result.relative_path.display()
        );
        println!("Title: {}", result.skill.title);
        println!("Path: {}", result.flush_result.file_path.display());
        println!("Category: {}", result.flush_result.category);
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Loadout(args)) => {
            if let Err(e) = run_loadout(args) {
                eprintln!("❌ 错误: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Offload(args)) => {
            if let Err(e) = run_offload(args) {
                eprintln!("❌ 错误: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Inspect(args)) => {
            if let Err(e) = run_inspect(args) {
                eprintln!("❌ 错误: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Mcp(args)) => {
            if let Err(e) = run_mcp(args) {
                eprintln!("❌ 错误: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Sync(args)) => {
            if let Err(e) = run_sync(args) {
                eprintln!("❌ 错误: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Search(args)) => {
            if let Err(e) = run_search(args) {
                eprintln!("❌ 错误: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Doctor(args)) => {
            if let Err(e) = run_doctor(args) {
                eprintln!("❌ 错误: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Install(args)) => {
            if let Err(e) = run_install_cmd(args) {
                eprintln!("❌ 错误: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Flush(args)) => {
            if let Err(e) = run_flush(args) {
                eprintln!("❌ 错误: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Distill(args)) => {
            if let Err(e) = run_distill(args) {
                eprintln!("❌ 错误: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Ui { vault, port, open }) => {
            let vault_path = detect_vault_path(vault);
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("❌ 错误: {}", e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = rt.block_on(k0maru::ui::run_server(vault_path, port, open)) {
                eprintln!("❌ 错误: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            // Default when no subcommand is provided
        }
    }
}
