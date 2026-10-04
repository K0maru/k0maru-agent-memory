# 30 — Flush Engine, CLI & FastMCP Integration

**Type:** task  
**Status:** open  
**Blocked by:** 29  

## Context
Implement the safe write-back engine (`FlushEngine`), integrating it with both the CLI (`k0maru flush`) and the FastMCP stdio server (`flush_session` tool). The engine writes notes into the target vault according to detected conventions, avoids collision/data loss, and automatically triggers incremental cache indexing.

## Objectives & Deliverables
1. **Flush Engine (`src/convention/flush.rs`)**:
   - `FlushEngine::flush(&self, request: FlushRequest) -> Result<FlushResult, FlushError>`
   - Anti-collision logic: if target filename already exists, appends `-v2` / `-2` or timestamp suffix, preventing silent overwrite.
   - Dry-run mode (`dry_run == true`): resolves destination path, renders final Markdown content, and returns `FlushResult` without writing to disk.
   - Auto-sync integration: upon successful write, invokes `IncrementalScanner` and vector sync, making the newly crystallized note immediately searchable.
2. **CLI Command (`src/main.rs`)**:
   - Add `Flush(FlushArgs)` variant to `Commands`:
     ```rust
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
     ```
   - Support reading content from `stdin` if `--content` is omitted and stdin is not a tty.
3. **FastMCP Integration (`src/mcp/server.rs`)**:
   - Register `flush_session` in `tools/list`:
     - Parameters: `title`, `summary`, `content`, `category`, `tags`, `related_notes`.
   - Implement handler calling `FlushEngine::flush`:
     - Scope strictly bounded by `self.vault_path`.
     - Returns formatted confirmation with relative path and WikiLinks count.
4. **Testing (`tests/test_flush_engine.rs` & `tests/test_mcp_flush.rs`)**:
   - Test dry-run: verifies zero file created on disk.
   - Test live write: verifies file created with proper Frontmatter and body.
   - Test anti-collision: verifies creating identical title produces second file without overwriting.
   - Test auto-sync: verifies that after flush, `recall_memory` or `SqliteStorage::search_fts` finds the new note.
   - Test FastMCP `flush_session` end-to-end via JSON-RPC.
5. **Acceptance Criteria**:
   - 100% test pass rate across entire suite.
   - `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean.
