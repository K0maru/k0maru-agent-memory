# CLI Command & Flag Reference (v1.0.0)

K0maru adheres to the Unix command-line philosophy and modern agent orchestration standards. It compiles into a single, static Rust binary (`k0maru` and `k0maru-memory`) requiring zero resident daemons and zero background Docker containers.

This manual provides the complete specification for all **14 CLI subcommands** in K0maru v1.0.0, including options, flags, and JSON output schemas.

---

## 🚀 Command Quick Reference

| Subcommand | Purpose | Key Options & Flags | Example Invocation |
| :--- | :--- | :--- | :--- |
| [`init` / `bootstrap`](#k0maru-init--bootstrap) | Bootstrap zero-config LLM-Wiki & Living ADR Guard | `--repo`, `--vault`, `--install`, `--project` | `k0maru init --repo . --install` |
| [`loadout`](#k0maru-loadout) | Generate sub-300-token project context loadout | `[QUERY]`, `--diff`, `--branch`, `--copy`, `--json` | `k0maru loadout payment-api --diff` |
| [`guard check`](#k0maru-guard-check) | Audit Git diff against Living ADR rules (<10ms) | `--diff`, `--claude-stop-hook`, `--strict`, `--json` | `k0maru guard check --diff` |
| [`guard install`](#k0maru-guard-install) | Install Git pre-commit hook to intercept violations | `--path`, `--hook`, `--vault` | `k0maru guard install` |
| [`flush`](#k0maru-flush) | Crystallize decisions or postmortems into notes | `--title`, `--category`, `--supersedes`, `--tags` | `k0maru flush --title "New ADR" --supersedes ADR-001` |
| [`search`](#k0maru-search) | Hybrid retrieval (BM25 + vector + graph) with auto-sync | `<QUERY>`, `--mode`, `--limit`, `--embed-model` | `k0maru search "mutex deadlock" --limit 5` |
| [`install`](#k0maru-install) | Configure FastMCP clients & lifecycle hooks | `--vault`, `--target`, `--hooks`, `--sidecar` | `k0maru install --target all --hooks` |
| [`pack`](#k0maru-pack) | Export, import, or inspect portable Memory Packs | `export`, `import`, `inspect` | `k0maru pack export --name redis-rules -o pack.json` |
| [`sync`](#k0maru-sync) | Synchronize vault metadata, FTS index, and vectors | `--vector`, `--embed-model`, `--force`, `--json` | `k0maru sync --vector` |
| [`offload`](#k0maru-offload) | Pipe filter long logs into Mermaid state diagrams | `--threshold`, `--task-id`, `--refs-dir` | `cargo test 2>&1 \| k0maru offload` |
| [`inspect`](#k0maru-inspect) | Retrieve raw offloaded stack traces by Node ID | `<node_id>`, `--refs-dir` | `k0maru inspect node_54697ed3` |
| [`distill`](#k0maru-distill) | Synthesize troubleshooting playbooks from traces | `--node`, `--file`, `--target`, `--export-hermes` | `k0maru distill --node node_xxx --target hermes` |
| [`gc`](#k0maru-gc) | 4-tier session trajectory Context Garbage Collection | `[FILE]`, `--threshold`, `--hot-window`, `--json` | `k0maru gc session.json --threshold 20` |
| [`doctor`](#k0maru-doctor) | Diagnose binary, vault, cache, and MCP health | `--vault`, `--home`, `--json` | `k0maru doctor` |
| [`ui`](#k0maru-ui) | Launch embedded developer cockpit console | `--port`, `--open`, `--embed-model` | `k0maru ui --port 3721 --open` |
| [`mcp`](#k0maru-mcp) | Start FastMCP stdio server (full or sidecar mode) | `--vault`, `--sidecar`, `--embed-model` | `k0maru mcp --sidecar` |

---

## 📖 Detailed Subcommand Reference

### `k0maru init` / `bootstrap`

Bootstraps a zero-config repository LLM-Wiki and Living ADR sentinel in `<repo>/.k0maru/wiki` from any Git repository. Can optionally install agent MCP client configurations and proactive lifecycle hooks in a single pass.

```bash
k0maru init [OPTIONS]
# or using the visible alias
k0maru bootstrap [OPTIONS]
```

#### Options & Flags
- `-r, --repo <PATH>`: Target Git repository path (defaults to current working directory).
- `-v, --vault <PATH>`: Custom destination knowledge vault path (defaults to `<repo>/.k0maru/wiki`).
- `-p, --project <NAME>`: Custom project name or slug.
- `-i, --install`: Automatically configures agent MCP clients and installs proactive lifecycle hooks (`.claude/settings.local.json` SessionStart hook and Git pre-commit guard hook).
- `--home <PATH>`: Home directory override for inspecting and writing client configurations.
- `--json`: Emits structured JSON installation and bootstrap report.

#### Example Invocation
```bash
# Initialize repo and configure all agent hooks in one shot
k0maru init --repo . --install
```

---

### `k0maru loadout`

Extracts project vision, technology stacks, and L3 architectural invariants into a compact Markdown summary strictly constrained to `< 300 tokens` to prevent attention drift.

```bash
k0maru loadout [QUERY] [OPTIONS]
```

#### Options & Flags
- `[QUERY]`: Target project name or search keyword (e.g., `payment-gateway`). Defaults to auto-detecting the nearest active project if omitted.
- `-v, --vault <PATH>`: Path to target Markdown vault root (auto-detected if omitted).
- `-d, --diff`: Inspects Git working tree / index changes and dynamically activates module-relevant ADRs based on modified files.
- `-b, --branch <BRANCH>`: Target Git base branch for comparison (e.g., `main`, `master`, `dev`).
- `-c, --copy`: Copies formatted Markdown directly to the system clipboard.
- `-l, --list`: Lists all active projects discovered in the vault.
- `--json`: Outputs structured JSON data.
- `--json-stream`: Outputs line-delimited JSON stream (NDJSON).

#### `--json` Output Schema Example
```json
{
  "project_name": "payment-gateway",
  "vault_path": "/workspace/my-project/.k0maru/wiki",
  "token_count": 218,
  "markdown": "# Context Loadout: Payment Gateway Core\n- Stack: Go 1.22, Redis 7...",
  "l3_invariants": [
    "ADR-001: Payment Callback Idempotency Guard"
  ],
  "recent_logs": []
}
```

---

### `k0maru guard`

Living ADR guard and architectural invariant sentinel. Audits code changes against architectural rules in $<10\text{ ms}$.

#### `k0maru guard check`

```bash
k0maru guard check [OPTIONS] [FILES...]
```

- `-d, --diff`: Audits current Git working tree and index diff against active Living ADR invariants discovered in the vault.
- `--staged`: Only audits staged changes (`git diff --cached`); used by Git pre-commit hooks.
- `-p, --project <PROJECT>`: Filters ADR invariants for a specific project scope.
- `-b, --branch <BRANCH>`: Target Git base branch for comparison (e.g., `main`, `dev`).
- `--claude-stop-hook`: Claude Code `Stop` hook mode: audits the working tree, emits violations to `stderr` with exit code `2`, and stays completely silent (exit `0`) when all rules pass.
- `--strict`: Treats all advisory warnings (`warn`) as blocking failures (`deny`).
- `--json`: Emits structured JSON audit report.
- `[FILES...]`: Optional explicit file paths to audit directly instead of diffs.

#### `k0maru guard install`

```bash
k0maru guard install [OPTIONS]
```

- `--hook <TYPE>`: Target Git hook type (default: `pre-commit`).
- `-p, --path <REPO_DIR>`: Target Git repository path (default: current directory).
- `-v, --vault <PATH>`: Target Markdown vault path to bind the hook to.

#### Example Invocation
```bash
# Audit working tree diff before committing
k0maru guard check --diff

# Run as Claude Code Stop hook
k0maru guard check --diff --claude-stop-hook
```

---

### `k0maru flush`

Crystallizes architectural decisions, verified bug fixes, or task milestones into permanent Markdown notes adhering to vault conventions, with automatic frontmatter and `[[WikiLinks]]` generation.

```bash
k0maru flush [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Target vault path (auto-detected if omitted).
- `-t, --title <TITLE>`: Title of the decision or log note (required).
- `-s, --summary <TEXT>`: Concise executive summary or conclusion.
- `-c, --content <TEXT>`: Markdown note body (accepts input from stdin pipes if omitted).
- `--category <CAT>`: Destination category (`decision`, `log`, `concept` [alias: `card`], `skill`, `generic`, default: `log`).
- `--tags <TAGS>`: Comma-separated tag list (e.g. `redis,idempotency`).
- `--related <TITLES>`: Comma-separated related note titles to link via `[[WikiLinks]]`.
- `--supersedes <NOTES>`: Comma-separated notes or ADRs that this note supersedes. Automatically updates superseded notes' frontmatter with `status: superseded` to resolve LLM belief entrenchment.
- `--dry-run`: Previews note paths and frontmatter without writing to disk.
- `--json`: Outputs structured JSON write confirmation.

#### Example Invocation
```bash
# Record an ADR that supersedes an earlier decision
k0maru flush \
  --title "ADR-012: Distributed Lock with Redlock" \
  --category decision \
  --summary "Migrate single-instance Redis mutex to Redlock cluster" \
  --supersedes "ADR-001-Redis-Mutex" \
  --tags "redis,distributed-lock"
```

---

### `k0maru search`

Executes multi-channel hybrid search combining SQLite FTS5 (BM25 lexical), `sqlite-vec` (ONNX vector embeddings), Reciprocal Rank Fusion ($k=60$), and 1-hop WikiLinks graph boost (+0.05).

**Auto-Sync Behavior**: Before executing a search query, `k0maru search` automatically runs an incremental scan (`IncrementalScanner`) over modified Markdown files, self-healing `cache.sqlite` and ensuring newly created notes are immediately searchable with zero manual sync steps.

```bash
k0maru search <QUERY> [OPTIONS]
```

#### Options & Flags
- `<QUERY>`: Search phrase, technical question, or `-` to read query from stdin.
- `-v, --vault <PATH>`: Path to target Markdown vault root.
- `-m, --mode <MODE>`: Search mode (`hybrid` [default], `bm25`, `vector`).
- `-l, --limit <N>`: Maximum candidate results to return (default: `5`).
- `--embed-model <MODEL>`: Embedding model (`all-minilm-l6-v2` [default], `bge-small-zh-v1.5`, etc.).
- `--json`: Outputs structured JSON array of search hits.
- `--json-stream`: Outputs line-delimited JSON stream (NDJSON).

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
- `--hooks`: Also installs proactive lifecycle hooks (`.claude/settings.local.json` SessionStart hook injecting `k0maru loadout --diff`, and Git pre-commit guard check).
- `--home <PATH>`: Overrides user home directory for isolated testing or custom paths.
- `--dry-run`: Previews configuration diffs without writing to disk.
- `--json`: Outputs structured installation report in JSON format.

---

### `k0maru pack`

Exports, imports, and inspects portable **Memory Packs** (`.pack.json`). Memory Packs decouple knowledge notes and Living ADR invariants from specific directory topologies, enabling cross-vault and cross-agent portability.

```bash
k0maru pack <COMMAND> [OPTIONS]
```

#### Subcommands

#### 1. `k0maru pack export`
Bundles notes and Living ADR invariants into a portable `.pack.json` package, stripping folder-bound prefixes from `[[WikiLinks]]`.

```bash
k0maru pack export --name <NAME> [OPTIONS]
```
- `--name <NAME>`: Descriptive name for the memory pack (required).
- `-v, --vault <PATH>`: Target source vault path.
- `--version <SEMVER>`: Memory pack version string (default: `0.1.0`).
- `--description <DESC>`: Optional description of contents.
- `--tag <TAG>`: Export only notes matching a tag (e.g. `topic/redis`).
- `--query <QUERY>`: Export only notes matching a search query.
- `--files <FILE...>`: Specific Markdown files to bundle.
- `-o, --output <FILE>`: Destination output file path.
- `--json`: Emits raw JSON to stdout.

#### 2. `k0maru pack import`
Imports a Memory Pack into any LLM-Wiki, automatically adapting to target directory conventions.

```bash
k0maru pack import <PACK_FILE> [OPTIONS]
```
- `<PACK_FILE>`: Path to `.pack.json` file (required).
- `-v, --vault <PATH>`: Target vault path.
- `--dry-run`: Previews imported notes and target paths without writing to disk.
- `--json`: Outputs structured JSON import results.

#### 3. `k0maru pack inspect`
Inspects a Memory Pack's metadata, note count, and bundled Living ADR invariants without writing to disk.

```bash
k0maru pack inspect <PACK_FILE> [OPTIONS]
```
- `<PACK_FILE>`: Path to `.pack.json` file (required).
- `-v, --vault <PATH>`: Target vault path to compute exact destination paths.
- `--json`: Outputs structured JSON inspection report.

---

### `k0maru sync`

Performs incremental or full synchronization of vault notes, YAML frontmatter, WikiLinks topology, and vector embeddings into disposable `.k0maru/cache.sqlite`.

```bash
k0maru sync [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to target Markdown vault root.
- `-f, --force`: Forces full re-parse of all notes, bypassing `mtime` and hash checks.
- `--vector`: Computes and caches vector embeddings incrementally for new or modified notes.
- `--embed-model <MODEL>`: Selects embedding model for vector generation.
- `--json`: Outputs synchronization statistics in JSON format.

---

### `k0maru offload`

A Unix pipe filter that intercepts terminal streams exceeding a line threshold, persisting raw logs to disk and emitting a compact symbolic Mermaid state diagram to stdout (99.4% token reduction).

```bash
<COMMAND> 2>&1 | k0maru offload [OPTIONS]
```

#### Options & Flags
- `-t, --threshold <N>`: Minimum line count required before triggering offload (default: `50` lines).
- `--task-id <ID>`: Associates a task identifier for downstream session tracing.
- `-r, --refs-dir <PATH>`: Directory where raw logs are saved (default: `.scratch/refs` or `<vault>/.k0maru/refs`).
- `--json-stream`: Outputs line-delimited JSON stream (NDJSON).

---

### `k0maru inspect`

Retrieves the raw, untruncated stack trace slice previously offloaded by `k0maru offload` using its assigned Node ID.

```bash
k0maru inspect <NODE_ID> [OPTIONS]
```

#### Options & Flags
- `<NODE_ID>`: Node identifier emitted in the Mermaid diagram (e.g. `node_54697ed3`).
- `-r, --refs-dir <PATH>`: Directory containing stored log slices.

---

### `k0maru distill`

Synthesizes troubleshooting playbooks from execution traces or offloaded crash logs, extracting symptoms, root causes, and verification steps.

```bash
k0maru distill [OPTIONS]
```

#### Options & Flags
- `--node <NODE_ID>`: Offloaded node ID to distill directly from cache storage.
- `-f, --file <PATH>`: Path to raw terminal trace or log file.
- `-t, --title <TITLE>`: Explicit title for the generated playbook.
- `-c, --context <HINT>`: Additional contextual explanation.
- `--category <CAT>`: Destination category (default: `skill`).
- `--tags <TAGS>`: Comma-separated tags.
- `--related <TITLES>`: Comma-separated related note titles.
- `-r, --refs-dir <PATH>`: Directory where offloaded references are stored.
- `--target <TARGET>`: Output format (`default`, `hermes`, or `omp`).
- `--export-hermes`: Directly exports resulting skill card to `~/.hermes/skills/`.
- `--export-omp`: Directly exports resulting skill to `~/.omp/agent/skills/<slug>/SKILL.md`.
- `--dry-run`: Previews synthesized skill card without writing to disk.
- `--json`: Outputs structured JSON representation.

---

### `k0maru gc`

Executes long-trajectory Context Garbage Collection (Context GC) using a 4-tier compaction protocol, achieving $\ge 85\%$ Token Reduction Rate (TRR) while preserving the active cognitive hot window.

```bash
k0maru gc [FILE] [OPTIONS]
```

#### Options & Flags
- `[FILE]`: Path to multi-turn session transcript JSON file (defaults to standard input if omitted).
- `-v, --vault <PATH>`: Path to vault root.
- `-t, --threshold <N>`: Line count threshold for folding verbose tool outputs (default: `20` lines).
- `-k, --hot-window <N>`: Number of recent turns preserved untouched in hot window (default: `5`).
- `--context-window <N>`: Context window size in tokens for Smart-Zone evaluation (default: `128000`).
- `--json`: Outputs compacted conversation JSON to stdout.
- `--json-stream`: Outputs line-delimited JSON stream (NDJSON).

---

### `k0maru doctor`

Performs end-to-end diagnostic health checks on binary location, vault layout, cache integrity, ONNX vector dimensions, and client MCP configurations.

```bash
k0maru doctor [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to target Markdown vault.
- `--home <PATH>`: Overrides user home directory for config inspection.
- `--json`: Outputs machine-readable JSON status report.

---

### `k0maru ui`

Spawns the local embedded developer console for interactive search exploration, graph visualization, and health metrics.

```bash
k0maru ui [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to target Markdown vault.
- `-p, --port <PORT>`: Port to bind (default: `3721`).
- `--open`: Automatically launches default web browser upon startup.
- `--embed-model <MODEL>`: Specifies vector embedding model.

---

### `k0maru mcp`

Starts the zero-daemon FastMCP JSON-RPC 2.0 server over standard I/O (`stdio`).

```bash
k0maru mcp [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to target Markdown vault root directory.
- `--embed-model <MODEL>`: Pluggable ONNX embedding model (`all-minilm-l6-v2`, `bge-small-zh-v1.5`, etc.).
- `--sidecar`: Enables orthogonal sidecar mode for `omp` (`Oh My Pi`), exposing 5 core memory and guard tools while omitting log offloading and GC tools.

---

## 🚦 Environment Variables & Exit Codes

### Environment Variables
- `K0MARU_VAULT`: Default Markdown vault root path when `--vault` is omitted.
- `CLAUDE_PROJECT_DIR`: Automatically discovered by Claude Code to mount repository `.k0maru/wiki`.
- `HERMES_HOME`: Discovery path override for Nous Research Hermes Agent.
- `OPENCLAW_HOME`: Discovery path override for OpenClaw Agent.

### Exit Codes
- `0`: Execution succeeded (Clean pass).
- `1`: Architectural invariant violation (`deny`) or operational error.
- `2`: Hook failure or syntax error (used by `--claude-stop-hook` to signal violations to Claude Code).
- `130`: Terminated by developer (`Ctrl+C` / SIGINT).
