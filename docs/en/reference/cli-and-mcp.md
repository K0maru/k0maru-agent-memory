# CLI & FastMCP Complete Reference Manual

K0maru adheres to the Unix command-line philosophy and modern agent orchestration standards. It compiles into a single, static Rust binary (`k0maru` and `k0maru-memory`) that requires zero resident daemons and zero open network ports.

This manual provides the comprehensive specification for all **CLI subcommands**, the **FastMCP JSON-RPC 2.0 protocol**, **Memory Packs (`k0maru pack`)**, **omp sidecar mode (`--sidecar`)**, and **ecosystem marketplace integrations**.

---

## 🚀 CLI Quick Reference

| Subcommand | Purpose | Primary Options | Example Invocation |
| :--- | :--- | :--- | :--- |
| [`loadout`](#k0maru-loadout) | Generate sub-300-token project context loadout | `[QUERY]`, `--diff`, `--copy`, `--json` | `k0maru loadout payment-api --diff` |
| [`search`](#k0maru-search) | Execute hybrid retrieval (BM25 + vector + graph) | `<QUERY>`, `--mode`, `--limit`, `--json` | `k0maru search "mutex deadlock" --limit 5` |
| [`offload`](#k0maru-offload) | Pipe filter long logs into Mermaid state diagrams | `--threshold`, `--task-id` | `cargo test 2>&1 \| k0maru offload` |
| [`inspect`](#k0maru-inspect) | Retrieve raw offloaded stack traces by Node ID | `<node_id>` | `k0maru inspect node_54697ed3` |
| [`sync`](#k0maru-sync) | Synchronize vault metadata, FTS index, and vectors | `--vector`, `--embed-model`, `--force` | `k0maru sync --vector` |
| [`flush`](#k0maru-flush) | Crystallize decisions or postmortems into notes | `--title`, `--category`, `--summary`, `--tags` | `k0maru flush --title "Redis Mutex" --category decision` |
| [`distill`](#k0maru-distill) | Synthesize troubleshooting playbooks from traces | `--trace-file`, `--node`, `--target` | `k0maru distill --node node_xxx --target hermes` |
| [`gc`](#k0maru-gc) | 4-tier session trajectory context garbage collection| `--input`, `--threshold` | `k0maru gc --input session.json` |
| [`guard check`](#k0maru-guard-check) | Audit git diff against Living ADR rules (<10ms) | `--diff`, `--strict`, `--json` | `k0maru guard check --diff` |
| [`guard install`](#k0maru-guard-install) | Install Git pre-commit hook to block violations | `--path` | `k0maru guard install` |
| [`install`](#k0maru-install) | Mount FastMCP configuration into AI coding clients | `--vault`, `--target`, `--dry-run` | `k0maru install --target omp --vault ~/Vault` |
| [`doctor`](#k0maru-doctor) | Diagnose binary, vault, and cache health | `--vault`, `--json` | `k0maru doctor` |
| [`ui`](#k0maru-ui) | Launch embedded developer cockpit console | `--port`, `--open` | `k0maru ui --port 3721 --open` |
| [`pack`](#k0maru-pack) | Export, import, or inspect portable Memory Packs | `export`, `import`, `inspect` | `k0maru pack export --name redis-rules -o pack.json` |
| [`mcp`](#k0maru-mcp) | Start FastMCP stdio server (full or sidecar mode) | `--vault`, `--sidecar`, `--embed-model` | `k0maru mcp --sidecar` |

---

## 📖 Detailed CLI Subcommand Reference

### `k0maru loadout`

Extracts project vision, technology stacks, and L3 architectural invariants into a compact Markdown summary strictly constrained to `< 300 tokens`.

```bash
k0maru loadout [QUERY] [OPTIONS]
```

#### Options & Flags
- `[QUERY]`: Target project name or keyword. Defaults to the first active project discovered in the vault if omitted.
- `-v, --vault <PATH>`: Path to the target Markdown vault root (auto-detected if omitted).
- `--diff`: Dynamically inspects git diff in the working directory to activate module-relevant ADRs.
- `-c, --copy`: Copies the formatted loadout directly to the system clipboard.
- `-l, --list`: Lists all active projects discovered in the vault.
- `--json`: Outputs structured JSON data.
- `--json-stream`: Outputs line-delimited JSON stream (NDJSON).

---

### `k0maru search`

Executes multi-channel hybrid search combining SQLite FTS5 (BM25 lexical), `sqlite-vec` (ONNX vector embeddings), Reciprocal Rank Fusion ($k=60$), and 1-hop WikiLinks graph boost (+0.05).

```bash
k0maru search <QUERY> [OPTIONS]
```

#### Options & Flags
- `<QUERY>`: Search phrase or keywords.
- `-v, --vault <PATH>`: Path to the target Markdown vault root.
- `-m, --mode <MODE>`: Search mode (`hybrid` [default], `bm25`, `vector`).
- `--embed-model <MODEL>`: ONNX embedding model (`all-minilm-l6-v2` [default], `bge-small-zh-v1.5`, etc.).
- `-l, --limit <N>`: Maximum number of results to return (default: 5).
- `--json`: Outputs structured JSON array of search hits.

---

### `k0maru offload`

A Unix pipe filter that intercepts large compiler or test logs from stdin, externalizing long traces into disk slices and emitting a compact symbolic Mermaid state diagram to stdout.

```bash
<COMMAND> 2>&1 | k0maru offload [OPTIONS]
```

#### Options & Flags
- `--threshold <N>`: Minimum line count required before triggering offload (default: 50 lines).
- `--task-id <ID>`: Associates a task identifier for downstream session tracing.

---

### `k0maru inspect`

Retrieves the raw, untruncated stack trace slice previously offloaded by `k0maru offload` using its assigned Node ID.

```bash
k0maru inspect <NODE_ID>
```

#### Arguments
- `<NODE_ID>`: Node identifier emitted in the Mermaid diagram (e.g. `node_54697ed3`).

---

### `k0maru sync`

Performs incremental or full synchronization of vault notes, YAML frontmatter, WikiLinks topology, and vector embeddings into the disposable `cache.sqlite`.

```bash
k0maru sync [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to the target Markdown vault root.
- `--vector`: Computes and caches vector embeddings incrementally for new or modified notes.
- `--embed-model <MODEL>`: Selects embedding model for vector generation.
- `--force`: Disregards file modification timestamps and executes a clean rebuild.

---

### `k0maru flush`

Crystallizes architectural decisions, verified bug fixes, or task milestones into permanent Markdown notes, automatically adapting to the target vault's directory structure and frontmatter conventions.

```bash
k0maru flush [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to the target Markdown vault root.
- `-t, --title <TITLE>`: Title of the note to create.
- `-c, --category <CATEGORY>`: Category name (`decision`, `log`, `concept`, `rule`).
- `-s, --summary <TEXT>`: Concise summary or conclusion.
- `--tags <TAGS>`: Comma-separated tags (e.g. `redis,idempotency`).
- `--related <TITLES>`: Comma-separated related note titles (formatted as `[[WikiLinks]]`).
- `--dry-run`: Previews the target path and generated Markdown without touching disk.
- `--json`: Outputs structured JSON result.
- `--stdin`: Reads note body content directly from standard input.

---

### `k0maru distill`

Synthesizes troubleshooting playbooks from execution traces or offloaded crash logs, extracting symptoms, root causes, and verification steps.

```bash
k0maru distill [OPTIONS]
```

#### Options & Flags
- `--trace-file <PATH>`: Path to raw terminal trace or log file.
- `--node <NODE_ID>`: Offloaded node ID to distill directly from cache storage.
- `-t, --target <TARGET>`: Output format (`default` for Markdown vault, or `hermes` for Nous Research Hermes Agent skill schema).
- `--dry-run`: Previews synthesized skill card without writing to disk.
- `--json`: Outputs structured JSON representation.

---

### `k0maru gc`

Executes long-trajectory Context Garbage Collection (Context GC) using a 4-tier compaction protocol, achieving $\ge 85\%$ Token Reduction Rate (TRR) while preserving the active cognitive hot window.

```bash
k0maru gc [OPTIONS]
```

#### Options & Flags
- `-i, --input <PATH>`: Path to the multi-turn session transcript JSON file.
- `--threshold <N>`: Line count threshold for folding verbose tool outputs (default: 20 lines).
- `--json`: Outputs compacted conversation JSON to stdout.

---

### `k0maru guard`

Enforces Living ADR architectural rules and invariants across staged and unstaged Git diffs in $<10\text{ ms}$.

#### `k0maru guard check`

```bash
k0maru guard check [OPTIONS]
```

- `--diff`: Audits current Git diff against active Living ADR invariants discovered in the vault.
- `--strict`: Elevates all non-blocking warnings (`warn`) to blocking failures (`deny`).
- `--json`: Outputs structured JSON guard report with violated invariant IDs and file locations.

#### `k0maru guard install`

```bash
k0maru guard install [OPTIONS]
```

- `-p, --path <REPO_DIR>`: Target git repository path. Installs `.git/hooks/pre-commit` to intercept non-compliant code automatically before commits.

---

### `k0maru install`

Safely injects K0maru FastMCP client configurations into installed AI coding agents using non-destructive JSON merges.

```bash
k0maru install [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to the target Markdown vault.
- `-t, --target <CLIENT>`: Specific client to configure:
  - `all` (default: auto-detects and configures all installed clients)
  - `claude` (Claude Code)
  - `cursor` (Cursor IDE)
  - `gemini` (Gemini CLI)
  - `windsurf` (Windsurf)
  - `cline` (Cline / Roo Code)
  - `hermes` (Nous Research Hermes Agent)
  - `openclaw` (OpenClaw Agent)
  - `omp` (Oh My Pi — automatically configures `--sidecar` mode)
- `--home <PATH>`: Overrides user home directory for isolated testing or custom paths.
- `--dry-run`: Previews configuration diffs without writing to disk.
- `--json`: Outputs structured installation report in JSON format.

```bash
# Configure Oh My Pi (omp) in sidecar mode
k0maru install --target omp --vault ~/Documents/MyVault
```

---

### `k0maru doctor`

Performs end-to-end diagnostic health checks on binary location, vault layout, cache integrity, ONNX vector dimensions, and client MCP configurations.

```bash
k0maru doctor [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to the target Markdown vault.
- `--home <PATH>`: Overrides user home directory.
- `--json`: Outputs machine-readable JSON status report.

---

### `k0maru ui`

Spawns the local, embedded developer console for interactive search exploration, graph visualization, and health metrics.

```bash
k0maru ui [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to the target Markdown vault.
- `-p, --port <PORT>`: Port to bind (default: `3721`).
- `--open`: Automatically launches the default web browser upon startup.
- `--embed-model <MODEL>`: Specifies vector embedding model.

---

### `k0maru pack`

Exports, imports, and inspects portable **Memory Packs** (`.pack.json`). Memory Packs decouple knowledge notes and Living ADR invariants from specific directory topologies, enabling cross-vault and cross-agent portability.

```bash
k0maru pack <COMMAND> [OPTIONS]
```

#### Subcommands

#### 1. `k0maru pack export`

Scans the source vault, extracts notes and Living ADR rules, strips folder-bound prefixes from `[[WikiLinks]]` (e.g. `[[20_Cards/Foo|Bar]]` $\to$ `[[Foo|Bar]]`), and bundles them into a portable JSON package.

```bash
k0maru pack export --name <NAME> [OPTIONS]
```

- `--name <NAME>`: Descriptive name for the memory pack (required).
- `-v, --vault <PATH>`: Target source vault path (auto-detected if omitted).
- `--version <SEMVER>`: Memory pack version string (default: `0.1.0`).
- `--description <DESC>`: Optional description of the pack's contents.
- `--tag <TAG>`: Export only notes matching a specific tag (e.g. `topic/redis`).
- `--query <QUERY>`: Export only notes matching a search query.
- `--files <FILE...>`: Specific markdown files to bundle into the pack.
- `-o, --output <FILE>`: Destination output file path (e.g. `redis-rules.pack.json`).
- `--json`: Emits raw JSON to stdout.

**Example**:
```bash
k0maru pack export --name redis-resilience --tag topic/redis -o redis.pack.json
```

#### 2. `k0maru pack import`

Imports a Memory Pack into any LLM-Wiki. K0maru's `FlushEngine` dynamically discovers the target vault's natural category conventions and naming schemes:
- **Flat Karpathy LLM-Wikis**: Notes land in the root directory with kebab-case filenames;
- **Engineering wikis**: Decisions land in `decisions/` or `docs/adr/`;
- **PARA vaults**: Notes land in `cards/` or `projects/`;
- **Johnny.Decimal / SecondBrain**: Notes land in `20_Cards/` or `10_Projects/`.

Imported notes preserve all Living ADR `invariants` in YAML frontmatter, immediately arming `k0maru guard check` in the new environment.

```bash
k0maru pack import <PACK_FILE> [OPTIONS]
```

- `<PACK_FILE>`: Path to the `.pack.json` file (required).
- `-v, --vault <PATH>`: Target vault path (auto-detected if omitted).
- `--dry-run`: Previews imported notes and predicted destination paths without writing to disk.
- `--json`: Outputs structured JSON import results.

**Example**:
```bash
k0maru pack import redis.pack.json --dry-run
k0maru pack import redis.pack.json
```

#### 3. `k0maru pack inspect`

Inspects a Memory Pack's metadata, note count, bundled Living ADR invariants, and predicted destination file paths for a target vault without applying changes.

```bash
k0maru pack inspect <PACK_FILE> [OPTIONS]
```

- `<PACK_FILE>`: Path to the `.pack.json` file (required).
- `-v, --vault <PATH>`: Optional target vault path to compute exact destination paths.
- `--json`: Outputs structured JSON inspection report.

**Example**:
```bash
k0maru pack inspect redis.pack.json
```

---

### `k0maru mcp`

Starts the zero-daemon FastMCP JSON-RPC 2.0 server over standard I/O (`stdio`).

```bash
k0maru mcp [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to the target Markdown vault root directory.
- `--embed-model <MODEL>`: Pluggable ONNX embedding model (`all-minilm-l6-v2`, `bge-small-zh-v1.5`, etc.).
- `--sidecar`: Enables orthogonal sidecar mode for `omp` (`Oh My Pi`), exposing 5 core memory and guard tools while omitting log offloading and GC tools.

---

## 🤖 FastMCP Protocol & Operating Modes

K0maru implements the Anthropic Model Context Protocol (MCP) version `2024-11-05` using JSON-RPC 2.0 over `stdio`. It runs strictly on demand: when an agent starts, it spawns `k0maru mcp`; when the agent shuts down, the process exits cleanly with 0 persistent RAM or open ports.

### Operating Modes Comparison

| Feature | Default Full Mode | Orthogonal Sidecar Mode (`--sidecar`) |
| :--- | :---: | :---: |
| **Target Clients** | Claude Code, Cursor, Windsurf, Hermes, OpenClaw | Oh My Pi (`omp`) |
| **Total Tools** | **8 Tools** | **5 Tools** |
| `get_project_loadout` | ✅ Yes | ✅ Yes |
| `recall_memory` | ✅ Yes | ✅ Yes |
| `offload_context` | ✅ Yes | ❌ Omitted (native to `omp`) |
| `inspect_log_node` | ✅ Yes | ❌ Omitted (native to `omp`) |
| `flush_session` | ✅ Yes | ✅ Yes |
| `distill_session_skill` | ✅ Yes | ✅ Yes |
| `gc_session_context` | ✅ Yes | ❌ Omitted (native to `omp`) |
| `guard_check` | ✅ Yes | ✅ Yes |

> **Why Sidecar Mode for `omp`?**  
> `omp` already includes first-class terminal stream management, session compaction, and log inspection. Running K0maru in `--sidecar` mode avoids duplicate or competing tools while delivering deep domain memory retrieval, decision crystallization, and sub-10ms Living ADR guardrails.

---

## 🛠️ FastMCP Tool Contracts & Schemas

### 1. `get_project_loadout`
*Available in: Full Mode & Sidecar Mode*

Fetches the project vision, core technology stack, and active L3 Living ADR invariants, strictly bounded to `< 300 tokens`. Supports change-aware diff inspection to dynamically activate module-relevant rules.

```json
{
  "name": "get_project_loadout",
  "description": "Fetch high-priority architectural blueprint, core invariants, project vision, and recent ADR decision logs (<300 tokens) from the vault. Supports change-aware diff inspection to dynamically activate module-relevant ADRs.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_name": {
        "type": "string",
        "description": "Optional project name to target. If omitted, matches the nearest or first active project."
      },
      "diff": {
        "type": "string",
        "description": "Optional git diff to dynamically activate module-relevant ADR rules."
      }
    }
  }
}
```

---

### 2. `recall_memory`
*Available in: Full Mode & Sidecar Mode*

Performs hybrid retrieval across the vault using SQLite FTS5 (BM25) and `sqlite-vec` (ONNX vector embeddings) with Reciprocal Rank Fusion ($k=60$) and WikiLinks graph boost. Defaults to ultra-dense single-line K-DSL wire entries (`!μ:<tier>|<path>|...`) to conserve agent context tokens.

```json
{
  "name": "recall_memory",
  "description": "Search and retrieve documentation, technical research reports, project context, past architectural decisions, configurations, setup guides, and notes from the user's local knowledge vault.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "The search keyword, conceptual phrase, or technical question."
      },
      "limit": {
        "type": "integer",
        "description": "Maximum number of notes to retrieve (default: 5)."
      },
      "full": {
        "type": "boolean",
        "description": "When true, returns full multi-line markdown content instead of compact single-line K-DSL wire entries."
      }
    },
    "required": ["query"]
  }
}
```

---

### 3. `offload_context`
*Available in: Full Mode Only*

Externalizes verbose terminal output, compiler stack traces, or test outputs ($>30$ lines) into an on-disk slice, emitting a structural Mermaid state diagram to prevent context window bloat.

```json
{
  "name": "offload_context",
  "description": "Externalize and compress verbose terminal logs, build outputs, test traces, or error dumps (>30 lines) into a compact symbolic Mermaid state diagram on disk.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "content": {
        "type": "string",
        "description": "The raw terminal output or stack trace to compress and store."
      },
      "task_id": {
        "type": "string",
        "description": "Optional task identifier for correlation."
      }
    },
    "required": ["content"]
  }
}
```

---

### 4. `inspect_log_node`
*Available in: Full Mode Only*

Retrieves the exact, uncompressed stack trace slice for an offloaded node by its Node ID.

```json
{
  "name": "inspect_log_node",
  "description": "Retrieve and inspect the full offloaded raw terminal log or compiler trace content by its node ID (e.g. node_1a2b3c4d).",
  "inputSchema": {
    "type": "object",
    "properties": {
      "node_id": {
        "type": "string",
        "description": "The symbolic node identifier from the Mermaid state diagram."
      }
    },
    "required": ["node_id"]
  }
}
```

---

### 5. `flush_session`
*Available in: Full Mode & Sidecar Mode*

Crystallizes architectural decisions, verified fixes, or milestone lessons into the local Markdown vault. The note is formatted with YAML frontmatter, linked via `[[WikiLinks]]`, and synchronized to the SQLite cache in $<5\text{ ms}$.

```json
{
  "name": "flush_session",
  "description": "Crystallize technical decisions, verified bug fixes, architecture decisions (ADRs), or task milestones back into the local Markdown vault as permanent notes.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "title": {
        "type": "string",
        "description": "Title of the note."
      },
      "category": {
        "type": "string",
        "description": "Category convention: decision, log, concept, rule, or custom."
      },
      "summary": {
        "type": "string",
        "description": "Executive summary of the milestone or breakthrough."
      },
      "tags": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Associated tags (e.g. [\"redis\", \"mutex\"])."
      },
      "related": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Titles of related notes to link as [[WikiLinks]]."
      },
      "content": {
        "type": "string",
        "description": "Detailed markdown body content."
      },
      "invariants": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Living ADR invariants to enforce (e.g. [\"deny: SELECT * FROM\"])."
      }
    },
    "required": ["title"]
  }
}
```

---

### 6. `distill_session_skill`
*Available in: Full Mode & Sidecar Mode*

Distills an execution trace or troubleshooting session into a standardized skill card or playbook, supporting export to Markdown or Nous Research Hermes Agent.

```json
{
  "name": "distill_session_skill",
  "description": "Distill an execution trace, compilation error, or debugging session into a reusable troubleshooting playbook/skill note and save it to the vault or export to Hermes Agent.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "trace_content": {
        "type": "string",
        "description": "Raw trace or error dump text."
      },
      "node_id": {
        "type": "string",
        "description": "Node ID of an offloaded log in cache storage."
      },
      "skill_name": {
        "type": "string",
        "description": "Name for the distilled skill."
      },
      "export_hermes": {
        "type": "boolean",
        "description": "Export in Hermes Agent skill schema format."
      }
    }
  }
}
```

---

### 7. `gc_session_context`
*Available in: Full Mode Only*

Applies the 4-tier compaction protocol to long multi-turn agent conversation trajectories, pruning verbose tool traces and collapsing compiler outputs to achieve $\ge 85\%$ TRR.

```json
{
  "name": "gc_session_context",
  "description": "Perform long-trajectory context garbage collection (Context GC) on a multi-turn agent session trajectory, folding verbose tool outputs and repetitive compiler error dumps into symbolic references.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "messages": {
        "type": "array",
        "items": { "type": "object" },
        "description": "List of conversation messages adhering to standard ChatML format."
      },
      "threshold": {
        "type": "integer",
        "description": "Line count threshold for folding tool results (default: 20)."
      }
    },
    "required": ["messages"]
  }
}
```

---

### 8. `guard_check`
*Available in: Full Mode & Sidecar Mode*

Audits a Git diff or code change against active Living ADR invariants discovered across the vault in $<10\text{ ms}$.

```json
{
  "name": "guard_check",
  "description": "Audit git diff or code changes against active Living ADR architectural invariants and project rules (<10ms).",
  "inputSchema": {
    "type": "object",
    "properties": {
      "diff": {
        "type": "string",
        "description": "Git diff string to audit. If omitted, checks uncommitted changes in current repository."
      },
      "project_name": {
        "type": "string",
        "description": "Optional project name to narrow rule scope."
      }
    }
  }
}
```

---

## 🔌 Ecosystem & Plugin Marketplace Integrations

### Tern Native Multiplexer Plugin

K0maru provides a native Luau plugin for the **Tern** terminal multiplexer (`tern-plugin/`):
- **Cockpit Block**: Interactive terminal block rendering binary health, SQLite index status, and Living ADR compliance meters (`r` to refresh);
- **CLI Lens**: Live output lens intercepting `k0maru *` commands and rendering structured tables and cards;
- **Window Palette**: Commands for `k0maru: cockpit`, `k0maru: doctor`, `k0maru: guard-check`, and Carly `recall` export.

```bash
# Install native Tern plugin
tern plugin install github.com/K0maru/k0maru-agent-memory/tern-plugin
```

### Oh My Pi (`omp`) Plugin Marketplace

K0maru is published to the `omp` marketplace catalog. Installing it automatically configures K0maru FastMCP in `--sidecar` mode:

```text
/marketplace add K0maru/k0maru-agent-memory
/marketplace install k0maru-memory@k0maru-agent-memory
```

### Claude Code Plugin Marketplace

```text
/plugin add K0maru/k0maru-agent-memory
```

---

## 🚦 Environment Variables & Exit Codes

### Environment Variables
- `K0MARU_VAULT`: Default Markdown vault root path when `--vault` is omitted.
- `HERMES_HOME`: Discovery path override for Nous Research Hermes Agent.
- `OPENCLAW_HOME`: Discovery path override for OpenClaw Agent.

### Exit Codes
- `0`: Execution succeeded (Clean pass).
- `1`: Architectural invariant violation (`deny`) or operational error.
- `2`: System configuration or environment error.
- `130`: Terminated by developer (`Ctrl+C` / SIGINT).
