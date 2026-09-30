# Spec: K0maru-Agent-Memory (Lightweight Standalone Memory Engine in Rust)

Status: ready-for-agent

## Problem Statement

Modern AI coding agents (such as Claude Code, Cursor, Windsurf, and Antigravity) suffer from three critical bottlenecks:
1. **Context Window Saturation & Amnesia**: Agents lose architectural memory between fresh sessions and forget key project constraints, requiring human developers to repetitively paste background context into every new session.
2. **Terminal Log Bloat & Cognitive Drift**: Running tests, linters, or build scripts frequently outputs hundreds of lines of traces. When dumped raw into the agent's context, this noise eats 30%–60% of the token budget and triggers hallucinations or instruction drift.
3. **Proprietary Lock-in & Microservice Bloat**: Existing agent memory solutions force developers into proprietary database formats, Docker microservice clusters, or heavy background daemons that hijack 4+ network ports and require complex runtime environments (Python venvs, Node runtime, C++ compilers).

Thousands of developers already maintain rich, structured knowledge in **Obsidian Vaults (like SecondBrain)** and **Karpathy-style LLM-Wikis**. There is currently no standalone, high-performance, single-static-binary, zero-daemon CLI engine that can natively mount these existing Markdown vaults and serve as a transparent, sub-5ms memory hub for AI agents.

## Solution

**K0maru-Agent-Memory** is an independent, cleanroom, zero-daemon memory engine written in **Rust**, producing a single standalone static binary (`k0maru`) built on two non-negotiable principles:
- **"Bring Your Own Wiki"**: Plain Markdown files with YAML Frontmatter and `[[WikiLinks]]` are the sole and immutable Source of Truth. No proprietary file formats, no mandatory directory reshaping.
- **"Truth in Files, Performance in Cache"**: SQLite functions purely as an ephemeral, disposable single-file cache (`cache.sqlite`). It can be wiped at any moment and fully rebuilt from raw Markdown in under 100 milliseconds.

The system provides four core decoupled layers:
1. **Storage & Wiki Adapters (`k0maru::adapters`)**: High-fidelity CommonMark AST extraction of YAML Frontmatter, bare `[[Card]]` and aliased `[[Card|Alias]]` links, `#tags`, and L0–L3 semantic hierarchies, natively prioritizing Karpathy LLM-Wiki and Obsidian SecondBrain layouts.
2. **Incremental Parse & Cache (`k0maru::scanner & storage`)**: Nanosecond-level `mtime + hash` dirty checking to synchronize only changed files into a single-file SQLite database with bundled FTS5 BM25 search.
3. **Hybrid Retrieval & Graph Routing (`k0maru::retrieval & graph`)**: Progressive dual-engine retrieval combining FTS5 full-text keyword ranking with 1-hop and 2-hop topological neighbor traversal over the WikiLinks adjacency graph.
4. **Agent-First CLI & MCP Protocol (`k0maru::cli & mcp`)**: 
   - `k0maru loadout <project>`: Assembles a high-density, `< 300 Token` YAML/Markdown context package for instant task warmup, supporting `--copy` and `--json`;
   - `k0maru offload`: Unix pipe filter that truncates long command logs (>50 lines) into `.scratch/refs/<task_id>_<node_id>.log` and outputs a symbolic Mermaid state machine;
   - `k0maru inspect <node_id>`: Pinpoint inspection of externalized error chunks;
   - `k0maru sync` & `k0maru doctor`: Machine-readable (`--json`) commands for automated Agent self-maintenance;
   - `k0maru mcp`: A lean, zero-daemon stdio FastMCP server implementing minimal, high-value tools adhering strictly to Occam's razor.

## User Stories

1. As an AI developer, I want to download or compile a single static binary `k0maru` without installing Python runtimes or virtual environments, so that I can use it anywhere instantly.
2. As an AI coding developer, I want to run `k0maru init --vault /path/to/notes` to bind my existing Markdown directory without altering any existing file suffixes or folder layouts.
3. As a knowledge worker, I want my notes to remain human-readable Markdown with standard `[[WikiLinks]]` and YAML metadata, so that my knowledge is never locked into a black-box database.
4. As an engineer with limited laptop resources, I want the memory system to start in <5ms, occupy <15MB of RAM, and operate with zero persistent background daemons and zero open network ports.
5. As an AI coding agent starting a work session, I want to invoke `k0maru loadout <project>` to receive an immediate `< 300 Token` context pack containing project vision, L3 active rules, and recent L2 logs, so that I have complete context without wasting token quota.
6. As an engineer switching between chat interfaces, I want to run `k0maru loadout <project> --copy` to automatically place the formatted loadout into my OS clipboard for one-click pasting.
7. As an AI coding agent executing a verbose test suite (`cargo test | k0maru offload`), I want outputs exceeding 50 lines truncated and saved to `.scratch/refs/<task_id>_<node_id>.log`, so that my prompt context only sees a compact Mermaid state transition diagram.
8. As an AI coding agent seeing a failed node in the Mermaid diagram, I want to run `k0maru inspect <node_id>` to view the exact error trace snippet, so that I can diagnose failures without rerunning the command or reading thousands of lines of output.
9. As an AI agent maintaining the workspace, I want to run `k0maru sync --json` and `k0maru doctor --json` to receive structured machine-readable health metrics without regex parsing terminal text.
10. As a developer actively editing notes, I want `k0maru sync` to inspect `mtime` and file hashes so that only added, modified, or deleted files are re-parsed, completing incremental syncs across 10,000 files in under 50ms.
11. As a developer dealing with a corrupted or deleted `cache.sqlite`, I want the engine to automatically re-index the entire vault from scratch in milliseconds without data loss.
12. As an AI coding agent in Claude Code, Cursor, or Windsurf, I want a standard FastMCP server over `stdio` (`k0maru mcp`), so that I can query memory via standard MCP tool calls without custom IDE extensions.
13. As an Obsidian SecondBrain user, I want the engine to natively recognize standard folders (`10_Projects` as projects, `20_Cards` as L3 Evergreen cards, `01_AI_Logs` as L2 logs, `00_Daily` as L0 ephemeral notes), so that my vault structure is immediately understood.
14. As a Karpathy-style flat LLM-Wiki user, I want the engine to parse flat Markdown pages and relative links, so that I can mount single-directory wiki repositories without restructuring them.
15. As an AI agent searching for relevant background, I want to perform full-text BM25 search across indexed cards and logs via FTS5, so that the most relevant knowledge cards are ranked highest.
16. As an AI agent exploring an architectural concept, I want to query 1-hop and 2-hop topological neighbors through WikiLinks, so that I understand all directly and indirectly connected cards.
17. As a developer writing tests, I want all automated test suites to run against isolated `tempfile::tempdir` fixtures, so that tests never touch or mutate my real personal SecondBrain vault.
18. As an architect adhering to Occam's razor, I want the MCP tool surface to remain minimal and deep, avoiding unnecessary entity bloat or redundant endpoints.

## Implementation Decisions

### 1. Technology Stack & Crates
- **Language**: Rust (2021 edition), compiled with `rustc >= 1.75`.
- **CLI Framework**: `clap` with `derive` and `cargo` features for clean subcommands, flags, and help generation.
- **Markdown Parsing**: `pulldown-cmark` (CommonMark event-based AST parser) + custom zero-allocation link extractor.
- **Embedded Database**: `rusqlite` with `bundled` feature enabled (compiles SQLite C-amalgamation with native FTS5 extension, 100% self-contained with 0 shared library linking issues).
- **Serialization**: `serde`, `serde_json`, `serde_yaml`.
- **Filesystem Traversal**: `walkdir` for high-throughput disk recursion.
- **Clipboard Integration**: `arboard` for cross-platform zero-dependency clipboard access.
- **Hashing**: `xxhash-rust` or `blake3` for instantaneous file content fingerprints.

### 2. Module Boundaries and Layer Seams
- **`src/core/`**:
  - `models.rs`: Domain entities (`Document`, `WikiLink`, `HierarchyLevel`, `SyncStats`, `OffloadNode`).
  - `traits.rs`: Traits defining the seams (`VaultAdapter`, `CacheStorage`).
- **`src/parser/`**:
  - `markdown.rs`: Extracts YAML Frontmatter, filters code blocks, extracts `WikiLink` (`[[Target]]`, `[[Target|Alias]]`), extracts `#tag`s.
- **`src/adapters/`**:
  - `obsidian.rs`: Implements `VaultAdapter` for SecondBrain folder structure.
  - `generic.rs`: Implements `VaultAdapter` for flat Karpathy LLM-Wiki.
- **`src/storage/`**:
  - `sqlite.rs`: Implements `CacheStorage` managing SQLite schema, tables (`documents`, `links`, `tags`), and FTS5 virtual table `documents_fts`.
- **`src/scanner/`**:
  - `engine.rs`: Incremental file change detector comparing disk state to SQLite metadata.
- **`src/loadout/`**:
  - `builder.rs`: Compiles project vision, top 5 L3 cards, and top 3 L2 logs into <300 token Markdown/YAML.
- **`src/offload/`**:
  - `engine.rs`: Streaming stdin log chunker, externalizes logs >50 lines to `.scratch/refs/`, generates Mermaid state diagrams.
- **`src/cli/`**:
  - `main.rs`, `args.rs`: CLI subcommands with `--json`, `--copy`, `--threshold`, `--force`.
- **`src/mcp/`**:
  - `server.rs`: Stdio JSON-RPC 2.0 MCP protocol listener.

### 3. Log Offloading Location
- Saved to `.scratch/refs/<task_id>_<node_id>.log` relative to current project root, or `<vault_root>/.k0maru/refs/` when operating inside a vault.

### 4. Minimal MCP Interface Surface (Occam's Razor)
FastMCP exposes exactly four deep, atomic tools:
1. `get_project_loadout(project_name: String) -> String`
2. `recall_memory(query: String, limit: usize) -> String`
3. `offload_context(raw_text: String, task_name: String) -> String`
4. `inspect_log_node(node_id: String) -> String`

## Testing Decisions

### 1. Seams and Test Philosophy
- **High-Seam Behavioral Testing**: Tests exercise the public Rust traits (`VaultAdapter`, `CacheStorage`, `IncrementalScanner`, `LoadoutBuilder`, `OffloadEngine`) and the `k0maru` CLI binary using `assert_cmd`.
- **Total Vault Isolation**: Real vaults are strictly read-only. Automated tests use `tempfile::tempdir` fixtures to spin up mock vaults.

### 2. Test Suites to Build
- `tests/test_domain_models.rs`: Serialization, deserialization, and hierarchy classification.
- `tests/test_parser.rs`: Frontmatter extraction, bare and aliased WikiLinks, hierarchical tags, code block link exclusion.
- `tests/test_adapters.rs`: `ObsidianAdapter` folder mapping and `GenericWikiAdapter` flat file traversal against mock vaults.
- `tests/test_sqlite_storage.rs`: SQLite schema initialization, CRUD, FTS5 BM25 search, link queries, wipe-and-rebuild resilience.
- `tests/test_scanner.rs`: Incremental dirty-checking state transitions (`added`, `modified`, `deleted`, `unchanged`).
- `tests/test_loadout.rs`: Token budget (<300 tokens) and content extraction.
- `tests/test_offload.rs`: Stream pipe threshold (>50 lines), disk persistence, Mermaid generation.
- `tests/test_cli.rs`: CLI commands (`init`, `sync`, `loadout`, `offload`, `inspect`, `doctor`).

## Out of Scope
- Background daemon processes or inotify file watchers.
- Cloud database syncing, authentication, multi-tenant web servers, Docker containers.
- Modifying or writing into the user's daily journals (`00_Daily/`).
- External dynamic C library dependencies.
