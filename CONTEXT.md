# CONTEXT.md — K0maru-Agent-Memory Domain Model & Glossary

This document defines the ubiquitous domain language, core conceptual entities, and architectural invariants of the `K0maru-Agent-Memory` ecosystem. All contributors, AI coding agents, specifications, tests, and implementations must strictly adhere to these definitions and boundaries.

---

## 📖 Domain Glossary

| Term | Symbol / Type | Definition & System Responsibilities |
| :--- | :--- | :--- |
| **Vault** | `Vault` | The root directory containing plain-text Markdown notes (e.g. `.k0maru/wiki`, Obsidian Vault, Karpathy LLM-Wiki, or repository docs). The vault is the single source of truth; no remote SaaS or proprietary database is permitted to hold persistent state. |
| **Document** | `Document` | The atomic document entity within a vault. Represents an immutable snapshot comprising relative path, content hash (`xxh3`), modification timestamp (`mtime`), parsed YAML Frontmatter, raw body content, and hierarchical classification. |
| **Hierarchy Level** | `HierarchyLevel` | Four-tier categorization model reflecting document lifecycle and epistemic maturity:<br/>• **`L0 (Daily)`**: Ephemeral daily logs, transient scratchpads, daily standups.<br/>• **`L1 (Resource)`**: Static references, cheat sheets, API documentation, environment guides.<br/>• **`L2 (Log)`**: AI interaction transcripts, benchmark outputs, investigation postmortems.<br/>• **`L3 (Evergreen)`**: Architectural decision records (ADRs), core invariants, permanent design principles. |
| **WikiLink** | `WikiLink` | Bidirectional cross-document link syntax, supporting bare targets (`[[TargetNote]]`) and piped aliases (`[[TargetNote\|Display Alias]]`). Forms a directed edge in the knowledge graph. Must never be wrapped in inline backtick code spans. |
| **Adjacency Graph** | `AdjacencyGraph` | In-memory directed graph constructed from WikiLinks. Powers 1-hop and 2-hop topological neighborhood traversal and graph boost scoring during hybrid search. |
| **Disposable Cache** | `DisposableCache` | Transient, single-file SQLite database (`cache.sqlite`) containing parsed document metadata, WikiLinks adjacency tables, FTS5 BM25 inverted indices, and dense vector embeddings. Disposable and rebuildable from raw Markdown in milliseconds without data loss. |
| **Incremental Scanner** | `IncrementalScanner` | High-throughput change detection engine combining `mtime` inspection with `xxh3` hashing. Accurately determines `added`, `modified`, and `deleted` document sets and updates the cache incrementally. |
| **Loadout** | `Loadout` | Compact context bundle synthesized for AI agents (<300 tokens). Aggregates project identity, mission, high-priority Living ADR invariants activated by current Git diff hunks, and recent engineering milestones to prime agents with zero cognitive overhead. |
| **Guard Engine / Rule** | `GuardEngine` / `GuardRule` | Deterministic, sub-10ms AST and token/regex validation engine. Audits unstaged, staged, or range-scoped Git diffs against active Living ADR invariants before commit or yield (`[DENY]` vs `[WARN]`). |
| **Flush Engine** | `FlushEngine` | Atomic write-back engine that persists architectural decisions, verified bug resolutions, and session milestones into the vault as Markdown notes. Detects target vault folder conventions and resolves filename collisions cleanly. |
| **Distill Engine** | `DistillEngine` | Heuristic pattern extractor that distills messy terminal execution traces, compiler errors, and debugging sessions into structured, reusable troubleshooting playbooks or skills. |
| **Hybrid Search Engine** | `HybridSearchEngine` | Multi-modal search engine combining SQLite FTS5 lexical BM25 retrieval, CPU ONNX dense vector embeddings (`sqlite-vec`), and WikiLinks adjacency graph boost (+0.004 to +0.05) via Reciprocal Rank Fusion (RRF, $k=60$). |
| **FastMCP Server** | `FastMCPServer` | Single-binary stdio JSON-RPC server implementing the Model Context Protocol. Exposes five core tools (`get_project_loadout`, `recall_memory`, `guard_check`, `flush_session`, `distill_session_skill`) to AI coding assistants without resident background daemons. |

---

## 🏛️ Core Architectural Invariants

### 1. Truth in Files, Performance in Cache
- The filesystem is the authoritative ground truth. All knowledge, decisions, invariants, and logs live as human-readable plain-text Markdown files.
- The SQLite cache (`cache.sqlite`) is strictly an ephemeral accelerator. If deleted, corrupted, or missing, the system must cleanly and automatically rebuild it from raw Markdown in milliseconds.

### 2. Zero Proprietary Formats
- No vendor lock-in or proprietary storage schemes.
- All documents adhere to standard CommonMark, GitHub Flavored Markdown (GFM), and standard YAML Frontmatter conventions readable by Obsidian, Logseq, VS Code, and standard command-line tools.

### 3. Sub-300 Token Context Loadouts & Smart-Zone Cognitive Budget
- Initial context injection via `k0maru loadout` or `get_project_loadout` MUST strictly respect a `< 300 token` footprint.
- Agents must operate within the cognitive Smart-Zone (15% to 60% of context window capacity). Massive terminal logs and redundant chat transcripts must be offloaded or summarized rather than dumped directly into context.

### 4. Sub-10ms Deterministic Guard & Zero-LLM Invariant Checking
- `k0maru guard check` must execute deterministically using compiled regex, AST substring matches, and file glob boundaries in `< 10ms` in memory (~15–55ms end-to-end including Git diff subprocess execution).
- Guard validation must never depend on non-deterministic, slow, or expensive LLM API calls.

### 5. Zero-Daemon & POSIX Pipe Composability
- The system must never require a resident background daemon, continuous polling process, or open network port for normal operation.
- All CLI commands must compose cleanly with standard Unix pipes (`stdin` / `stdout` / `stderr`), and the MCP server must communicate strictly over standard input/output streams (`stdio`).
