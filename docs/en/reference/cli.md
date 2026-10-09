# CLI Command & Flag Reference

K0maru adheres to the Unix command-line philosophy, providing a comprehensive set of subcommands with structured JSON output capabilities.

---

## Command Quick Reference

| Subcommand | Purpose | Example Invocation |
| :--- | :--- | :--- |
| [`loadout`](#k0maru-loadout) | Generate a sub-300-token project bootstrap context | `k0maru loadout my-proj --copy` |
| [`search`](#k0maru-search) | Execute hybrid retrieval (BM25 + vector + graph boost) | `k0maru search "mutex deadlocks" --limit 5` |
| [`offload`](#k0maru-offload) | Pipe filter long logs into a Mermaid state diagram | `cargo test 2>&1 \| k0maru offload` |
| [`inspect`](#k0maru-inspect) | Retrieve raw offloaded stack traces by Node ID | `k0maru inspect node_54697ed3` |
| [`sync`](#k0maru-sync) | Synchronize vault metadata, FTS index, and vectors | `k0maru sync --vector` |
| [`flush`](#k0maru-flush) | Crystallize decisions or incident postmortems into notes | `k0maru flush --title "New ADR" --tags "db"` |
| [`distill`](#k0maru-distill) | Synthesize troubleshooting playbooks from traces | `k0maru distill --node node_xxx --export-hermes`|
| [`install`](#k0maru-install) | Mount FastMCP configuration into AI coding clients | `k0maru install --vault ~/my-vault` |
| [`doctor`](#k0maru-doctor) | Diagnose binary, vault, and cache health | `k0maru doctor` |
| [`ui`](#k0maru-ui) | Launch the local dark cockpit developer console | `k0maru ui --open` |
| [`mcp`](#k0maru-mcp) | Start the FastMCP JSON-RPC server over stdio | `k0maru mcp --vault ~/my-vault` |

---

## Detailed Subcommand Reference

### `k0maru loadout`

Extracts project vision, technology stacks, and L3 architectural invariants into a compact Markdown summary constrained to `< 300 tokens`.

```bash
k0maru loadout [QUERY] [OPTIONS]
```

#### Options & Flags
- `[QUERY]`: Target project name or search keyword (e.g., `payment-gateway`). If omitted without `--list`, matches the first active project.
- `-v, --vault <PATH>`: Path to the target Markdown vault root. Defaults to auto-detecting the current directory.
- `-c, --copy`: Copies formatted Markdown directly to the system clipboard (compatible with macOS, Linux, and Windows).
- `-l, --list`: Lists all active projects discovered in the vault.
- `--json`: Outputs structured JSON data.

#### `--json` Output Schema Example
```json
{
  "project_name": "payment-gateway",
  "vault_path": "/Users/k0maru3/workspace/SecondBrain",
  "token_count": 218,
  "markdown": "# Context Loadout: Payment Gateway Core\n- Stack: Go 1.22, Redis 7...",
  "l3_invariants": [
    "ADR-001: Payment Callback Idempotency Guard"
  ],
  "recent_logs": []
}
```

---

### `k0maru search`

Executes multi-channel hybrid search across knowledge base notes.

```bash
k0maru search <QUERY> [OPTIONS]
```

#### Options & Flags
- `<QUERY>`: Required search query or natural language question.
- `-v, --vault <PATH>`: Path to the Markdown vault root.
- `-m, --mode <MODE>`: Retrieval mode:
  - `hybrid` (default): BM25 lexical + ONNX vector + 1-hop graph boost RRF fusion.
  - `bm25`: Pure SQLite FTS5 inverted search (fastest, requires zero ONNX memory).
  - `vector`: Pure semantic dense vector search.
- `-l, --limit <N>`: Maximum number of candidate results to return (default: `5`).
- `--json`: Outputs results as a JSON array of matching records.

#### `--json` Output Schema Example
```json
[
  {
    "path": "20_Cards/adr-001-idempotency.md",
    "title": "ADR-001: Payment Callback Idempotency Guard",
    "score": 0.0325,
    "bm25_rank": 1,
    "vector_rank": 1,
    "graph_boost": 0.05,
    "snippet": "Services must enforce two invariants: 1. Acquire Redis distributed mutex..."
  }
]
```

---

### `k0maru offload`

Standard Unix pipe filter that intercepts terminal streams exceeding a line threshold, persisting raw logs to disk and emitting a 15-line Mermaid state diagram.

```bash
<COMMAND> | k0maru offload [OPTIONS]
```

#### Options & Flags
- `-t, --threshold <N>`: Line count threshold to trigger offloading (default: `50` lines).
- `--task-id <ID>`: Optional task identifier prefix for persistent log slicing.
- `-r, --refs-dir <PATH>`: Directory where raw logs are saved (default: `.k0maru/refs/` or `.scratch/refs/`).

---

### `k0maru inspect`

Retrieves the untruncated raw log slice associated with an offloaded Node ID.

```bash
k0maru inspect <NODE_ID> [OPTIONS]
```

#### Options & Flags
- `<NODE_ID>`: Required log node identifier (e.g., `node_54697ed3`).
- `-r, --refs-dir <PATH>`: Directory containing stored log slices.

---

### `k0maru sync`

Scans vault notes incrementally and synchronizes metadata, FTS5 indexes, and vector embeddings into `.k0maru/cache.sqlite`.

```bash
k0maru sync [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to the target Markdown vault root.
- `-f, --force`: Forces a full re-parse of all notes, bypassing `mtime` and hash checks.
- `--vector`: Computes and caches vector embeddings incrementally.
- `--json`: Outputs synchronization statistics in JSON format.

---

### `k0maru flush`

Crystallizes architectural decisions, incident postmortems, or task summaries into Markdown notes adhering to vault conventions.

```bash
k0maru flush [OPTIONS]
```

#### Options & Flags
- `--title <TITLE>`: Required note title.
- `--category <CAT>`: Note category (`decision`, `log`, `concept`, default: `log`).
- `--summary <TEXT>`: Optional one-line summary or executive conclusion.
- `--content <TEXT>`: Optional Markdown note body (also accepts input from stdin pipes).
- `--tags <T1,T2>`: Comma-separated tag list.
- `--related <R1,R2>`: Comma-separated related note titles (formatted as `[[WikiLinks]]`).
- `--dry-run`: Previews note paths and frontmatter without writing to disk.
- `-v, --vault <PATH>`: Path to the target Markdown vault root.
- `--json`: Outputs structured write results.

---

### `k0maru distill`

Synthesizes a standardized, reusable troubleshooting playbook from terminal crash logs or offloaded slices.

```bash
k0maru distill [OPTIONS]
```

#### Options & Flags
- `--node <NODE_ID>`: Offloaded node ID to distill.
- `-f, --file <PATH>`: Path to a raw log file to distill.
- `-t, --title <TITLE>`: Explicit title for the generated playbook.
- `-c, --context <HINT>`: Additional contextual explanation.
- `--category <CAT>`: Destination category (default: `skill`).
- `--export-hermes`: Exports the resulting skill card to `~/.hermes/skills/`.
- `--target <TARGET>`: Target schema format (`default` or `hermes`).
- `--dry-run`: Previews distillation without writing to disk.
- `--json`: Outputs structured JSON results.

---

### `k0maru install`

Injects FastMCP configuration entries into installed AI coding clients safely and non-destructively.

```bash
k0maru install [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to the target Markdown vault.
- `-t, --target <CLIENT>`: Specific client to configure (`all`, `claude`, `cursor`, `gemini`, `windsurf`, `cline`, `hermes`, `openclaw`, default: `all`).
- `--home <PATH>`: Overrides user home directory for isolated testing.
- `--dry-run`: Previews configuration diffs without writing to disk.
- `--json`: Outputs installation results in JSON format.

---

### `k0maru doctor`

Performs end-to-end health checks on the binary, system environment, local vault, and client configurations.

```bash
k0maru doctor [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to the target Markdown vault.
- `--home <PATH>`: Overrides user home directory.
- `--json`: Outputs diagnostic status flags and details in JSON format.

---

### `k0maru ui`

Launches the embedded local developer console for real-time retrieval debugging.

```bash
k0maru ui [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to the target Markdown vault.
- `-p, --port <PORT>`: Port to bind (default: `3721`).
- `--open`: Automatically opens the dashboard in your default browser.

---

### `k0maru mcp`

Starts the FastMCP JSON-RPC 2.0 server over standard I/O (`stdio`).

```bash
k0maru mcp [OPTIONS]
```

#### Options & Flags
- `-v, --vault <PATH>`: Path to the target Markdown vault.

---

## 🚦 Environment Variables & Exit Codes

### Environment Variables
- `K0MARU_VAULT`: Specifies the default vault root directory when `--vault` is omitted.
- `HERMES_HOME`: Overrides the default discovery directory for Nous Research Hermes Agent.
- `OPENCLAW_HOME`: Overrides the default discovery directory for OpenClaw Agent.

### Exit Codes
- `0`: Execution succeeded (Success).
- `1`: General error (invalid arguments, missing target path).
- `2`: Vault convention conflict or file system I/O error.
- `130`: Terminated by developer via `Ctrl+C` (SIGINT).
