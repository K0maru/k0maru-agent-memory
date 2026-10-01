use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand};

use k0maru::adapters::{GenericWikiAdapter, ObsidianAdapter};
use k0maru::core::traits::VaultAdapter;
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
        None => {
            // Default when no subcommand is provided
        }
    }
}
