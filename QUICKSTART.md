# ⚡ Quick Start: K0maru-Agent-Memory

> 🤖 **Zero-Human Setup Prompt for AI Coding Assistants (Claude Code, Cursor, Windsurf, Cline, Hermes, OpenClaw)**:  
> Instruct your AI assistant directly in your project root:
> ```text
> Read https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/llms.txt (or AGENT_GUIDE.md) and set up k0maru-agent-memory for this repository.
> ```
> *(Agent Playbook: **[`AGENT_GUIDE.md`](AGENT_GUIDE.md)** · Machine-Readable Spec: **[`llms.txt`](llms.txt)**)*

---

## 🛠️ Step 1: Install the Single Static Binary

`k0maru` is distributed as a single static binary with an embedded FastMCP server, local SQLite cache, and bundled ONNX vector runtime. It requires zero resident background processes, zero Python virtual environments, and zero external databases. Cold start is strictly sub-5ms (empirically 3.4ms).

### Recommended: Official One-Line Installer (macOS & Linux)
```bash
curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash
```

### Option 2: Homebrew (macOS & Linux)
```bash
brew install K0maru/tap/k0maru
```

### Option 3: Compile from Source
```bash
# Clone and build release binary
cargo build --release

# Install into user PATH
cp target/release/k0maru ~/.local/bin/

# Verify installation
k0maru --version
# Output: k0maru 1.0.0
```

---

## 🚀 Option A: 30-Second Zero-Vault Bootstrap (`k0maru init --install`)

You do not need an existing Obsidian vault or pre-configured notes directory. Run inside any Git repository:

```bash
# One-command autonomous bootstrap in current repository
k0maru init --install
```

### What `k0maru init --install` (alias `k0maru bootstrap`) Does Automatically:
1. 🔍 **Project Topology Sensing**: Analyzes `README.md`, build manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`), agent instruction files (`AGENTS.md`, `CLAUDE.md`), and recent Git commits.
2. 📐 **Scaffolds Ground-Truth Knowledge Wiki (`.k0maru/wiki/`)**:
   - `10_Projects/<project-slug>.md`: Synthesizes project mission, core modules, architecture invariants, and operational commands.
   - `20_Cards/adr-001-repository-guardrails.md`: Initial Living ADR card with executable regex rules preventing credential leaks, debug dumps, and unvetted commits.
   - `01_AI_Logs/<YYYY-MM-DD>-initial-bootstrap.md`: First handover log documenting repository baseline.
3. 🔗 **Mounts FastMCP & Configures Lifecycle Hooks**:
   - Wires `.mcp.json` (Claude Code) and `.cursor/mcp.json` (Cursor).
   - Configures Claude Code lifecycle hooks (`SessionStart` auto-loadout, `Stop` pre-yield guard) in `.claude/settings.local.json`.
   - Installs Git lifecycle hooks (`.git/hooks/pre-commit` fail-closed guard and `.git/hooks/post-commit` automatic synchronization).
4. ⚡ **Builds Rebuildable Cache**: Indexes Markdown notes into `.k0maru/wiki/.k0maru/cache.sqlite` with full FTS5 BM25 search and dense vector embeddings.

### Autonomous Zero-Human Loop (No Human Intervention Needed)
- **Session Start**: Claude Code / agents trigger `k0maru loadout --diff`, injecting `<300 tokens` of active project context and diff-triggered ADR invariants.
- **Pre-Yield Guard**: Claude Code runs `k0maru guard check --diff --claude-stop-hook`. If violations exist (exit code 2), the agent autonomously refactors or supersedes the rule before responding.
- **Pre-Commit Guard**: `.git/hooks/pre-commit` verifies all staged changes against active ADR invariants in `<10ms`.
- **Post-Commit Sync**: `.git/hooks/post-commit` synchronizes index caches and crystallizes milestone notes for `feat:`, `fix:`, `refactor:`, `perf:`, and `adr:` commits.

---

## 📚 Option B: Bring Your Own Obsidian / Markdown Vault

If you already maintain an Obsidian vault, Johnny.Decimal vault, or flat Karpathy-style Markdown folder, mount it across your agent ecosystem:

```bash
# Mount existing vault and install lifecycle hooks across all detected agent clients
k0maru install --vault ~/Documents/MyVault --hooks

# Run health diagnostics
k0maru doctor
```

K0maru's non-destructive JSON injector discovers installed clients (Claude Code, Cursor, Windsurf, Cline, Gemini CLI, Hermes Agent, OpenClaw) and safely mounts the MCP server.

---

## ⚡ Core Workflows & Daily Operations

### 1. Diff-Aware Context Loadout (`k0maru loadout --diff`)
Prime any AI coding session with an ultra-compact (<300 tokens) context payload:
```bash
# Inspect loadout for current project incorporating git diff
k0maru loadout --diff

# Copy loadout directly to system clipboard for manual pasting
k0maru loadout my-project --diff --copy
```
Injects project mission, active Living ADR invariants triggered by modified files, and recent milestone logs in ~5ms.

### 2. Living ADR Guard (`k0maru guard check --diff`)
Audit modified code against active architecture invariants in `<10ms` in memory:
```bash
# Check working tree and staged diffs
k0maru guard check --diff

# Check staged changes only (used in pre-commit hooks)
k0maru guard check --diff --staged

# Claude Code pre-yield stop hook (returns exit code 2 on violation)
k0maru guard check --diff --claude-stop-hook
```

### 3. Decision Crystallization & Supersession (`k0maru flush`)
Persist new architectural decisions or replace obsolete invariants:
```bash
# Supersede an obsolete ADR and register new invariants
k0maru flush \
  --category decision \
  --title "ADR-042: Fenced Distributed Token Lease Standard" \
  --supersedes "ADR-010: Legacy Redis SetNX Lock" \
  --summary "Migrated settlement locking to distributed fenced token leases" \
  --content $'## Non-Negotiable Invariants\n- [fenced-lease-required] (deny) [src/settlement/**/*] must_contain: "acquire_fenced_lease"' \
  --tags "architecture,redis,leases"
```
The superseded ADR is automatically marked `status: superseded`, penalized in search ($0.2\times$), and its invariants deactivated.

### 4. Multi-Modal Hybrid Search (`k0maru search`)
Retrieve notes using Reciprocal Rank Fusion (RRF, $k=60$) combining BM25 lexical matching, 384-dimensional ONNX embeddings, and WikiLinks graph boost (+0.004):
```bash
# Hybrid search across your vault
k0maru search "distributed lock idempotency"

# Pure lexical or vector search
k0maru search "acquire_fenced_lease" --mode bm25
k0maru search "how to handle double payments" --mode vector
```

### 5. Long Log Symbolic Offloading (`k0maru offload` & `inspect`)
Prevent terminal log explosions (>50 lines) from overflowing agent context:
```bash
# Pipe verbose commands through offload
cargo test | k0maru offload
pytest -v | k0maru offload
```
Logs exceeding the threshold are stored in `.scratch/refs/` and replaced with an offline Mermaid state diagram. To inspect the full raw trace:
```bash
k0maru inspect node_54697ed3
```

### 6. Trace-to-Skill Distillation (`k0maru distill`)
Extract messy compilation errors or debugging traces into reusable playbooks:
```bash
# Distill a session trace into a Markdown playbook note
k0maru distill --trace-file /tmp/error.log --title "Fixing OpenSSL Linking Errors"

# Export directly to Hermes Agent dynamic skill directory
k0maru distill --trace-file /tmp/error.log --target hermes --export-hermes
```

### 7. Portable Memory Bundles (`k0maru pack`)
Share verified ADRs and domain knowledge across repositories without repository coupling:
```bash
# Export active ADR decisions into a portable pack
k0maru pack export --category decision --output security-rules.pack.json

# Import into another repository's wiki (auto-adapts folder conventions)
k0maru pack import security-rules.pack.json --dry-run
k0maru pack import security-rules.pack.json
```

### 8. Embedded Developer Dark Console (`k0maru ui`)
Visualize multi-modal retrieval attribution, search waterfalls, and log state machines:
```bash
# Launch zero-daemon local dashboard and open browser
k0maru ui --vault ~/Documents/MyVault --open
```
Runs at `http://127.0.0.1:3721`. Press `Ctrl+C` to immediately release port and memory.

### 9. System Diagnostics (`k0maru doctor`)
Verify binary status, vault markdown health, cache integrity, and agent client integrations:
```bash
k0maru doctor
```

---

## 📖 CLI Command Reference

| Command | Key Arguments | Description |
| :--- | :--- | :--- |
| `k0maru init` *(alias: `bootstrap`)* | `--repo <path>`, `--vault <path>`, `--install` | Scaffolds local wiki, Living ADRs, and client hooks in 1 second |
| `k0maru install` | `--vault <path>`, `--hooks`, `--target <client>` | Mounts MCP server into Claude Code, Cursor, Windsurf, Cline, etc. |
| `k0maru doctor` | `--vault <path>`, `--json` | Comprehensive environment, cache, and client configuration health check |
| `k0maru loadout` | `--diff`, `--copy`, `-l/--list`, `<query>` | Synthesizes diff-aware context loadout (<300 tokens) |
| `k0maru guard check` | `--diff`, `--staged`, `--claude-stop-hook` | Audits git diff against active Living ADR invariants (<10ms) |
| `k0maru search` | `<query>`, `--mode <hybrid\|bm25\|vector>`, `--limit <N>` | Multi-modal hybrid search (BM25 + ONNX Vector + Graph Boost) |
| `k0maru sync` | `--vault <path>`, `--vector`, `--json` | Incremental vault change scan and cache index regeneration |
| `k0maru offload` | `--threshold <N>`, `--task-id <id>` | Truncates long terminal logs into Mermaid state machines |
| `k0maru inspect` | `<node_id>` | Retrieves full raw trace for an offloaded log node |
| `k0maru flush` | `--title <title>`, `--category <cat>`, `--supersedes <id>` | Atomically persists ADRs, notes, or milestones to vault |
| `k0maru distill` | `--trace-file <path>`, `--target <hermes>` | Extracts troubleshooting playbooks or Hermes dynamic skills |
| `k0maru pack` | `export`, `import`, `--dry-run` | Exports or imports portable `.pack.json` memory bundles |
| `k0maru ui` | `--vault <path>`, `--port <port>`, `--open` | Zero-daemon embedded web console (`127.0.0.1:3721`) |
| `k0maru mcp` | `--vault <path>` | Launches FastMCP stdio server for IDE agent integration |
