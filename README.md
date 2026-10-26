[English](README.md) | [简体中文](README_zh.md)

# K0maru-Agent-Memory

> **Turn Your Markdown Wiki / Obsidian Vault into Executable Architectural Memory & `<10ms` Guardrails for AI Coding Agents.**  
> *Unlike passive session recorders that merely log chat transcripts, K0maru turns your architecture decisions (ADRs) and engineering notes into a closed-loop governance engine: **`<300-token` Diff-Aware Loadout ➔ `<10ms` Git Pre-Commit Guard ➔ Zero-LLM Deterministic Supersession (`--supersedes`)**.*
[![Language: Rust 2021](https://img.shields.io/badge/Language-Rust_2021-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version: 1.0.0](https://img.shields.io/badge/Version-1.0.0-blue.svg)]()
[![Tests: 200+ passed](https://img.shields.io/badge/Tests-200%2B_passed-brightgreen.svg)]()
[![Binary Size: 3.66MB](https://img.shields.io/badge/Binary_Size-3.66MB-success.svg)]()
[![Cold Start: 3.4ms](https://img.shields.io/badge/Cold_Start-3.4ms-purple.svg)]()
[![Zero-Daemon](https://img.shields.io/badge/Daemon-Zero_Daemon-informational.svg)]()

> 💡 **Dual Binary Support**: Available as both `k0maru` and `k0maru-memory` standalone binaries for seamless compatibility with multi-agent orchestration systems.

---

## 🌟 Core Advantages & Architectural Superiority (Why K0maru?)

Modern AI coding agents (Claude Code, Cursor, Windsurf, Gemini, etc.) frequently suffer from two critical pain points: **Cross-Session Amnesia** (loss of architectural decisions across sessions) and **Terminal Log Bloat** (compiler and test outputs flooding the context window).

Existing agent memory frameworks frequently deploy heavy multi-container Docker clusters (PostgreSQL, pgvector, Redis) or resident background daemons, incurring continuous RAM footprints (800MB–2GB), port conflicts, and complex deployment friction.

**K0maru is engineered specifically for local developer workstations, delivering industrial-grade performance with minimal mechanical complexity**:

1. **⚡ Zero-Daemon & Sub-Millisecond Cold Start**:
   - Pure Rust, statically compiled into a single **3.66 MB** standalone binary.
   - Cold-starts in just **3.38 ms** with a lean ~11 MB RAM footprint; **occupies 0 persistent network ports**.
   - Runs on demand via standard Unix pipes and `stdio` streams, releasing all memory immediately upon process exit.
2. **📝 Bring Your Own Markdown (Topology-Agnostic LLM-Wiki)**:
   - Your existing local Markdown notes serve as the **Single Source of Truth**. Fully topology-agnostic: adapts seamlessly to **any LLM-Wiki shape** — flat Karpathy-style LLM-Wikis, engineering `docs/adr/` or `decisions/` wikis, PARA (`projects/`, `cards/`, `logs/`), and Johnny.Decimal (`10_Projects/`, `20_Cards/`).
   - Zero proprietary binary lock-in, zero migration friction, and zero changes to your existing folder structure.
3. **🪓 Disposable Transient Cache**:
   - Maintains only a single-file SQLite database (`cache.sqlite`) integrating FTS5 BM25 full-text indices and `sqlite-vec` virtual tables with pluggable embedding dimensions.
   - Strictly disposable: delete it anytime (`rm cache.sqlite`) and the scanner **self-heals and rebuilds the index from scratch in 74 ms**.
4. **📉 Symbolic Context Offloading & Governance (99.4% TRR & Live A100 Validation)**:
   - Filters verbose logs via standard Unix pipes (`| k0maru offload`), transforming long traces into Mermaid state diagrams and stored slices.
   - Includes 4-tier Context GC (`k0maru gc`) keeping agent sessions within the cognitive Smart-Zone (15%–60%).
5. **🛡️ Living ADR Guard & Active Supersession (`<10ms` Audit + Zero Memory Tax)**:
   - Transforms passive Architecture Decision Records (ADRs) and vault notes into active, executable guardrails via `k0maru guard check --diff` or the `guard_check` FastMCP tool.
   - **Zero-LLM Deterministic Supersession (`k0maru flush --supersedes`)**: When an architectural decision evolves, passing `--supersedes ADR-010` deterministically marks the old note `status: superseded` (compatible with Google OKF lifecycle metadata), strips its historical WikiLink graph boost to prevent **Belief Entrenchment / Memory Tax**, excludes it from `<300-token` loadouts, and **automatically retires its old guard rules while activating the new ADR's anti-regression rules**.
### Architectural Comparison Matrix

| Evaluation Dimension | Vanilla Context (Copy-Paste) | Containerized Microservices (Letta / TencentDB) | Resident Background Daemons (agentmemory) | **K0maru-Agent-Memory (This Project)** |
| :--- | :--- | :--- | :--- | :--- |
| **Storage Medium** | Ephemeral chat history | External DBs (PostgreSQL, pgvector, Redis) | Proprietary DB in hidden directories | **Local Markdown Vaults (Plain Text)** |
| **Deployment Model** | Manual copy | Multi-container Docker Compose clusters | Background daemon process (`iii-engine`, etc.) | **Single static binary, on-demand (Zero-Daemon)** |
| **Network Port Footprint** | None | 1–4 open ports | 1–4 open ports | **0 ports** (Unix pipes & stdio streams) |
| **Cold Start Latency** | Instant | 2,500 ms – 3,500 ms (container boot) | 1,000 ms – 2,000 ms | **3.38 ms** (native Rust binary) |
| **Resident Memory (RAM)** | 0 MB | ~850 MB – 2,000 MB | ~150 MB – 300 MB | **~11.7 MB** (released instantly on command exit) |
| **Terminal Log Governance**| Manual or LLM truncation | Network API transfer required | Internal bespoke truncation | **Standard Unix pipe filter (`\| k0maru offload`)** |
| **Living ADR Sentinel** | Manual code review | External SonarQube / heavy CI | None | **Living ADR Guard (<10ms git diff audit against vault rules)** |
| **Memory Decay & Supersession** | Stale docs poison prompt | Heavy LLM graph extraction | Passive search demotion only | **Zero-LLM `--supersedes` + Anti-Entrenchment + Guard rule hot-swap** |
| **Index Self-Healing** | None | Snapshot & backup dependent | Local database integrity dependent | **`cache.sqlite` disposable anytime, rebuilt in 74ms** |

### 🛡️ Verified Adversarial Supersession & Living ADR Guard A/B Benchmark

Tested across **DeepSeek V4.1 Flash** and **Google Colab NVIDIA L4 + Qwen2.5-Coder-7B + Pi CLI** on the `payment-settlement` repository. Comparing an unmanaged markdown wiki against K0maru's closed-loop governance engine under architectural evolution (migrating from legacy `ADR-010` raw `redis_setnx_unscoped` locks to `ADR-042` distributed fenced token leases `acquire_fenced_lease`):

| Evaluation Dimension / Stage | Control Group (Naive Markdown Wiki without `--supersedes`) | Treatment Group (`k0maru flush --supersedes` + `guard check --diff`) | Breakthrough & Governance Impact |
| :--- | :--- | :--- | :--- |
| **Retrieval Rank #1 (`recall_memory`)** | **Stale `ADR-010` #1** (`redis_setnx_unscoped`)<br>*Belief Entrenchment: historical WikiLink boost poisons prompt* | **Active `ADR-042` #1** (`acquire_fenced_lease`)<br>*Stale note deprioritized; auto-forwards `<300-token` Loadout via `superseded_by`* | **Zero Memory Tax**: Eliminated stale pattern poisoning without manual wiki cleanup |
| **Dynamic Loadout (`<300-token`)** | Bloated / Stale (`>650 tokens` including superseded notes) | **Compact: 245 tokens** (only active invariants injected) | **Strict Context Budget**: Keeps prompt within cognitive Smart-Zone |
| **Pre-Commit Guard Enforcement** | **0 violations (Bypassed)**<br>*Unsafe `redis_setnx_unscoped` committed to codebase* | **BLOCKED in <5ms** (`[DENY] ban-legacy-setnx`)<br>*Executable regex & diff guard rejects unsafe lock pattern* | **100% Defect Interception**: Zero architectural regression reaches Git history |
| **Autonomous Agent Self-Healing** | Failed (Silent production vulnerability) | **100% Pass in 1 Turn**<br>*Drives 1-turn self-healing on both DeepSeek V4.1 and 7B Qwen2.5-Coder* | Autonomous remediation using active ADR invariants and guard feedback |

#### Autonomous Agent Ecosystem Adaptation (Hermes Agent & OpenClaw on Real LLM-Wiki)

Simulated an enterprise-grade payment gateway LLM-Wiki on Colab (containing P0 service architecture, L3 idempotency rules, and incident postmortems), driving **Qwen2.5-Coder-32B** through an autonomous Hermes / OpenClaw function calling loop. All 4 core invariants achieved 100% compliance:

| Invariant Verification Item | Expected Standard & Behavior | Empirical Outcome | Status |
| :--- | :--- | :--- | :---: |
| **1. Pre-flight Memory Inspection** | Query loadout and rules before generating any code | Autonomously issued `get_project_loadout` + 2x targeted `recall_memory` calls | **PASS ✓** |
| **2. Architectural Invariant Compliance** | Generated code must enforce distributed mutex & state guard | Produced Go handler with Redis lock `SET NX EX 30` & `STATUS_PENDING` check | **PASS ✓** |
| **3. Self-Healing Session Flush** | Persist ADR decision to wiki via `flush_session` with WikiLinks | Automatically created `20_Cards/adr-pay-012-...md` with frontmatter | **PASS ✓** |
| **4. Instant Recall Without Re-indexing** | Note must be recallable immediately without manual re-index | Recalled in **0.88 ms** with top score of 12.05 (Rank #1 hit) | **PASS ✓** |

> 📖 **Comprehensive Case Breakdown & Autonomous Agent Traces**:  
> For the complete per-case logs, multi-turn Hermes/OpenClaw traces, and model notes, see [Part 4 & Part 5 of the Comparison Report](docs/COMPARISON_REPORT.md#4-empirical-agent-ecosystem-adaptation-nous-research-hermes-agent--openclaw).
---

## 🚀 Quick Start & Installation

For deep-dive walkthroughs and multi-agent setup, see [QUICKSTART.md](QUICKSTART.md).

### ⚡ 30-Second Zero-Vault Quick Start (No Obsidian Required!)

Only have a Git repository? You don't need Obsidian or an existing Markdown vault. Run two commands inside your repository root:

```bash
# 1. Install standalone binary (<4MB, zero daemon)
curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash

# 2. Bootstrap repository wiki and wire all installed agents + hooks
k0maru init --install
```

**What `k0maru init --install` (alias `k0maru bootstrap`) does in <1 second:**
- 🔍 **Scans Repository**: Auto-detects project structure and conventions from `README.md`, package manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`), agent guidelines (`AGENTS.md`, `CLAUDE.md`), and recent git commit history.
- 📐 **Bootstraps Memory Wiki**: Generates `.k0maru/wiki/` containing:
  - `10_Projects/<project_slug>.md`: Project architectural scope, module seams, and operational guidelines.
  - `20_Cards/adr-001-repository-guardrails.md`: Initial Living ADR card with executable regex/boundary invariants (blocks secret leaks, debug dumps, and rule violations).
  - `01_AI_Logs/<YYYY-MM-DD>-initial-bootstrap.md`: Initial handoff log summarizing recent commits and development momentum.
- 🔗 **Wires FastMCP & Hooks**: Injects FastMCP configurations into Claude Code, Cursor, Codex, Windsurf, Gemini CLI, Cline, etc., and installs `.git/hooks/pre-commit` (blocks non-compliant commits) and Claude Code `SessionStart` hooks (`k0maru install --hooks`).
- ⚡ **Builds Disposable Cache**: Compiles `cache.sqlite` with FTS5 and vector indices so `k0maru loadout` and `k0maru guard check --diff` work immediately!

---

### Option B: Bring Your Own Markdown / Obsidian Vault

If you already maintain an Obsidian vault, PARA folder, or engineering wiki:

```bash
# Auto-detect and configure all installed agents pointing to your Markdown vault
k0maru install --vault ~/Documents/MyVault --hooks

# Or preview configuration diffs safely without writing to disk
k0maru install --vault ~/Documents/MyVault --dry-run
```

For manual setup, add this snippet to your IDE's MCP config:
```json
{
  "mcpServers": {
    "k0maru-memory": {
      "command": "k0maru",
      "args": ["mcp", "--vault", "/path/to/your/markdown-vault"]
    }
  }
}
```

Or install via the Claude Code Plugin Marketplace:
```text
/plugin add K0maru/k0maru-agent-memory
```

---

### Binary Installation Options

- **One-Line Install Script (macOS & Linux - Recommended)**:
  ```bash
  curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash
  ```
- **Homebrew (macOS & Linux)**:
  ```bash
  brew install K0maru/tap/k0maru
  ```
- **Build from Source (Cargo)**:
  ```bash
  cargo install --git https://github.com/K0maru/k0maru-agent-memory
  ```
  *(Builds both `k0maru` and `k0maru-memory` binary aliases).*

---

### 3. Verify Health & Environment (`k0maru doctor`)

Run the automated doctor check to inspect your binary, vault conventions, SQLite indices, and agent bindings:

```bash
k0maru doctor
```

Outputs a clear, color-coded health report (`✓ Binary In Path`, `✓ FTS5 Index Ready`, `✓ Vectors Indexed`, `✓ Claude Code Configured`, `✓ Cursor Configured`) with actionable remediation steps for any warnings.

---

## 🛠️ Common Use Cases & Workflows (How-Tos)

### Use Case 1: Kickstarting New Sessions —— Rapid Context Loadout (`k0maru loadout` & `get_project_loadout`)

Avoid dumping dozens of unrelated files into fresh agent chats. `loadout` extracts project mission, core architectural invariants (L3), and recent engineering logs (L2), strictly contained within **< 300 Tokens**:

- **4-Tier Scoped Project Resolver**:
  - **Tier 1**: Normalized exact match, alias, tag, and backlink-derived project aliases.
  - **Tier 2**: Scoped FTS5 SQL push-down via `search_scoped_fts` (never global Top 5 truncation).
  - **Tier 3**: Vector semantic fallback (if vector embeddings exist).
  - **Tier 4**: Ambient workspace / CWD directory matching.
- **Git Diff Change-Awareness (`--diff`)**: Dynamically analyzes working tree modifications to activate module-relevant ADRs and invariants.

```bash
# Generate project loadout and copy directly to the system clipboard (macOS / Linux / Windows)
k0maru loadout my-project --vault ~/wiki --copy

# Diff-aware loadout: dynamically inspect git changes and activate relevant ADRs
k0maru loadout my-project --vault ~/wiki --diff --copy
```

**FastMCP Autonomous Tool Equivalent**: `get_project_loadout`
```json
{
  "project_name": "k0maru-agent-memory",
  "diff": true,
  "branch": "main"
}
```

---

### Use Case 2: Build & Test Failures —— Symbolic Context Offloading (`| k0maru offload`)

When running test suites or compilers that generate thousands of lines, pipe the command output through `k0maru`:

```bash
# Intercept verbose dumps; outputs a clean Mermaid diagram with a node_id
cargo test 2>&1 | k0maru offload

# Retrieve targeted failure stack trace slice on demand
k0maru inspect node_54697ed3
```

- **99.44% Token Reduction**: Truncates massive compiler dumps, storing full raw logs safely under `.scratch/refs/` or `<vault>/.k0maru/refs/`.
- **Mermaid State Diagrams**: Transforms raw stack traces into structural transitions (e.g., `Iteration1 --> Failure: E0382 borrow after move`), guiding LLMs directly to the root cause without attention drift.
- **FastMCP Autonomous Tools**:
  - `offload_context`: Agents can offload terminal logs produced during tool execution.
  - `inspect_log_node`: Retrieve specific stack slices by `node_id`.
  - `gc_session_context`: Compresses long 50+ turn trajectory sessions using a 4-tier compaction protocol (preserving hot windows and achieving TRR >= 85%).

---

### Use Case 3: Querying the Vault —— Natural Language Intent-Driven Recall (`recall_memory` & `k0maru search`)

Once connected via MCP, frontier models automatically understand intent and invoke the right tools without memorizing explicit keywords:

```bash
# Hybrid search (BM25 + ONNX vectors + RRF k=60 + WikiLinks graph boost)
k0maru search "concurrency deadlock mitigation" --vault ~/wiki --mode hybrid --limit 5

# Pluggable embeddings: select specific model or dimension
k0maru search "vector similarity" --vault ~/wiki --embed-model bge-small-zh-v1.5
```

- **Ultra-Compact Wire Encoding**: Defaults to condensed single-line wire entries to save context window tokens (see [Advanced: Multiplexer & Sidecar Ecosystem](#advanced-multiplexer--sidecar-ecosystem-tern-omp--k-dsl-wire-protocol)). Pass `"full": true` when verbatim markdown excerpts are required.
- **Pluggable Vector Backends**: Built-in support for `all-minilm-l6-v2` (384d, default), `bge-small-zh-v1.5` (512d), `bge-base-en-v1.5` (768d), `paraphrase-multilingual-minilm-l12-v2` (384d), and mock embedders for offline testing.
- **Dynamic Dimension Migration**: Seamlessly reconfigures `sqlite-vec` virtual tables upon model switching without manual database rebuilds.

---

### Use Case 4: Preserving Decisions & Troubleshooting Skills —— Memory Crystallization (`k0maru flush` & `distill`)

Crystallize engineering takeaways, architectural decisions (ADRs), or debugging playbooks back into your vault:

```bash
# Crystallize a new ADR and deterministically supersede an outdated ADR
k0maru flush \
  --title "ADR-042 Native Sidecar Concurrency Model" \
  --category decision \
  --supersedes "ADR-010 Legacy Bus" \
  --summary "Replaced legacy tuple bus with zero-daemon stdio sidecar" \
  --content "## Non-Negotiable Invariants\n- [no-legacy-bus] forbidden: \"legacy_bus_send\" | remediation: \"Use v2 sidecar RPC\"" \
  --tags "tokio,rust,architecture" \
  --vault ~/wiki
# Distill troubleshooting trace into a reusable skill note
k0maru distill \
  --trace-file /tmp/error.log \
  --title "Resolving SQLite Vec Apple Silicon Linking" \
  --vault ~/wiki
```

- **Convention Sniffing**: Automatically adapts to your vault's naming style (e.g., `YYYY-MM-DD-slug.md`), directory structures, and `AGENTS.md` rules.
- **Collision Defense**: Detects duplicate titles and increments versions safely (`-v2.md`) without overwriting existing notes.
- **Hermes Skill Export**: Supports `--target hermes` to output dynamic skills compatible with Nous Research Hermes Agent.
- **Deterministic Supersession (`--supersedes`)**: Automatically updates the superseded note's YAML frontmatter (`status: superseded`, `superseded_by: "[[...]]"`), strips its inbound link boost in hybrid search (`[SUPERSEDED by ...]`), removes it from `<300-token` loadouts, and hot-swaps its active `k0maru guard` rules.
- **FastMCP Autonomous Tools**: `flush_session` and `distill_session_skill`.

---

### Use Case 5: Pre-Commit Sentinel —— Living ADR Guard (`k0maru guard` & `guard_check`)

ADRs and technical guidelines should not be dead text in forgotten folders. K0maru's **Living ADR Guard** turns your documentation into an automated architectural sentinel:

```bash
# Audit Git working tree / staged diff against active Living ADR invariants (<10ms)
k0maru guard check --diff

# Install pre-commit hook into repository to block non-compliant commits automatically
k0maru guard install
```

#### How Invariants are Declared

In your vault's ADR notes (`20_Cards/` or `10_Projects/`), add an `invariants` block to the frontmatter:

```yaml
---
title: ADR-042 Distributed Lock Standard
category: decision
tags: [architecture, redis, idempotency]
invariants:
  - id: "enforce-distributed-mutex"
    condition: "must_contain"
    pattern: "redis_lock\\.acquire|SET\\s+NX\\s+EX"
    paths: ["src/payment/**/*.rs", "services/order/**/*.go"]
    severity: "deny"
    fix_hint: "Critical financial paths must acquire a distributed Redis lock before mutating state."
  - id: "ban-raw-sql-in-handlers"
    condition: "forbidden"
    pattern: "SELECT .* FROM raw_|execute_unprepared"
    paths: ["src/api/**/*.rs"]
    severity: "deny"
    fix_hint: "Use query builders or prepared statements in storage layer."
---
```

You can also place rules in repository root `.k0maru/rules`, `guard.toml`, or `AGENTS.md`.

- **<10ms Lightning Fast**: Runs in native Rust, evaluating hundreds of diff hunks instantly.
- **Exit Codes**: Returns `0` on clean pass, `1` on blocking `deny` violation, `2` on execution error.
- **FastMCP Autonomous Tool**: `guard_check` allows AI agents to audit diffs before presenting changes to the human developer.

---

### Use Case 6: Vault Introspection —— Embedded Developer Console (`k0maru ui`)

Launch a local visual dashboard on demand:

```bash
k0maru ui --vault ~/wiki --open
```

![K0maru Developer Console - Search Debugger & Graph Boost](docs/images/ui-search.png)

- **🌐 Bilingual Switching**: Instant one-click toggle between English and Chinese in the header.
- **🔍 Search Explainability Console**: Inspect BM25 ranks, vector cosine distances, and Emerald `+0.004 Graph Boost` badges.
- **🪵 Log & State Machine Inspector**: Native SVG Mermaid renderer with syntax-highlighted error logs and one-click copying.
- **📊 Token Economics Scoreboard**: Real-time telemetry tracking vector coverage, database size, and cumulative tokens saved.
- **Zero Daemon**: Pressing `Ctrl+C` immediately frees all memory and network ports. See [docs/UI_TUTORIAL.md](docs/UI_TUTORIAL.md).

---

### Use Case 7: Vault Changes —— Sub-Millisecond Incremental Synchronization (`k0maru sync`)

```bash
# Incremental rescan of markdown files and FTS5 indices (takes just 11ms when clean)
k0maru sync --vault ~/wiki

# Batch incremental generation of ONNX vector embeddings (skips unchanged notes)
k0maru sync --vault ~/wiki --vector

# Sync with custom embedding model
k0maru sync --vault ~/wiki --vector --embed-model bge-small-zh-v1.5
```

---

## 🤖 Autonomous Agent Ecosystem & FastMCP Protocol

When connected via FastMCP, K0maru provides the complete suite of long-term memory, architectural guardrails, and context governance tools across universal IDEs and CLI agents (Claude Code, Cursor, Codex, Windsurf, Gemini CLI, Cline, Hermes, OpenClaw):

| FastMCP Tool | Target Intent & Natural Prompt Triggers | Core Value & Mechanics |
| :--- | :--- | :--- |
| `get_project_loadout` | *"Start new task"*, *"What are our architectural invariants?"* | Scoped 4-tier retrieval returning mission, rules, and recent logs strictly **< 300 Tokens**. Supports `--diff` dynamic activation. |
| `recall_memory` | *"How do we configure X?"*, *"Search notes for Y"* | Hybrid BM25 + ONNX vector + RRF $k=60$ + 1-hop WikiLinks graph boost. Instant sub-millisecond retrieval. |
| `offload_context` | Large tool execution outputs, test logs (>30 lines) | Externalizes verbose terminal text into Mermaid state diagrams and stored disk slices. Prevents attention drift. |
| `inspect_log_node` | *"Inspect failure stack for node_xxxx"* | Retrieves exact raw stack trace slice on demand by node ID. |
| `flush_session` | *"Document this decision"*, *"Save milestone to vault"* | Atomically writes structured ADRs or logs into vault matching local naming conventions and frontmatter schemas. |
| `distill_session_skill`| *"Distill this bug fix into a reusable skill"* | Extracts troubleshooting patterns into structured skill notes (Markdown or Hermes format). |
| `gc_session_context` | Long multi-turn agent dialogues approaching context limit | Compresses session trajectory with 4-tier pruning protocol, achieving **TRR >= 85%** while preserving hot window. |
| `guard_check` | Before Git commits, post-implementation code review | Audits diff against Living ADR rules (<10ms). Halts non-compliant code from entering repositories. |

> 💡 **Pro-Tip: Autonomous Vault Inspection System Prompt**  
> Add this single instruction to your `AGENTS.md` or global system prompt:  
> `> Before designing architecture, debugging complex failures, or implementing core logic, inspect the local vault using k0maru-memory to align with historical ADRs and codebase conventions. Run guard_check before completing tasks.`  
> Agents will proactively consult your security and architectural standards before writing code.

### 🎛️ Advanced: Multiplexer & Sidecar Ecosystem (Tern, omp, & K-DSL Wire Protocol)

For specialized terminal multiplexers, orchestrators, and high-density multi-agent setups:

1. **Tern Terminal Multiplexer Plugin**:
   Native Luau plugin for Tern featuring an interactive cockpit dashboard, CLI lens, and Carly palette integration:
   ```bash
   tern plugin install github.com/K0maru/k0maru-agent-memory/tern-plugin
   ```

2. **Oh My Pi (`omp`) & Orthogonal Sidecar Mode (`k0maru mcp --sidecar`)**:
   Because `omp` natively manages terminal streams and session compaction, K0maru exposes an orthogonal 5-tool subset (`get_project_loadout`, `recall_memory`, `flush_session`, `distill_session_skill`, `guard_check`) omitting `offload_context`, `inspect_log_node`, and `gc_session_context` to prevent pipeline redundancy:
   ```bash
   # Add via omp marketplace
   /marketplace add K0maru/k0maru-agent-memory
   /marketplace install k0maru-memory@k0maru-agent-memory
   ```

3. **Ultra-Compact K-DSL Wire Protocol (`!μ:`)**:
   To prevent Master Agent context explosion during subagent coordination, MCP `recall_memory` formats results into ultra-dense single-line K-DSL wire entries:
   `!μ:<tier>|<path>|score:<score>|links:<in>/<out> § <snippet>` (capped at ≤140 chars per snippet).
   This cuts payload size by **73.16%** (164 bytes / 22 tokens vs 611 bytes / 112 tokens). Pass `"full": true` in tool parameters when verbatim multi-line markdown excerpts are needed.

---

## 📊 Empirical Google Colab A100 Benchmark Matrix

Evaluated live on a dedicated **NVIDIA A100-SXM4-80GB GPU** across 5 frontier and coding models on 5 representative industrial defect scenarios (Rust `E0382`, Python `InvalidStateError`, TypeScript auth null reference, Go deadlock, C double free) and 4 architectural invariants:

### 1. Multi-Model Benchmark Matrix (5-Case Symbolic Offload Micro-Benchmark, N=5)

| Evaluated Model | Architecture | Pass@1 (Baseline) | Pass@1 (K0maru) | Avg Latency (Base -> K0maru) | Speedup | Prompt Tokens (Base -> K0maru) | Token Reduction (%) |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Qwen2.5-32B-Instruct** *(Flagship Kanban)* | 32B Dense Instruct | 20.0% (1/5) | **40.0% (2/5)** | 16.70s ➔ **6.14s** | **2.72x** | 5,745 ➔ **1,076** | **81.3%** |
| **DeepSeek-Coder-V2-16B** | 16B MoE (2.4B active) | 80.0% (4/5) | **100.0% (5/5)** | 9.43s ➔ **1.36s** | **6.93x** | 6,298 ➔ **1,142** | **81.9%** |
| **Qwen2.5-Coder-32B** *(Code Baseline)* | 32B Dense Q4_K_M | 60.0% (3/5) | **100.0% (5/5)** | 24.68s ➔ **7.13s** | **3.46x** | 5,472 ➔ **1,119** | **79.6%** |
| **DeepSeek-R1-32B** | 32B Reasoning Distill | 20.0% (1/5) | **40.0% (2/5)** | 12.01s ➔ **9.19s** | **1.31x** | 5,412 ➔ **989** | **81.7%** |
| **Google Gemma 2 27B** | 27B Open Foundation | 0.0% (0/5) | 0.0% (0/5) | 16.69s ➔ **6.94s** | **2.40x** | 6,356 ➔ **1,134** | **82.2%** |

*Note: 5-Case Symbolic Offload Micro-Benchmark (N=5 industrial defect cases evaluated under Ollama local inference on NVIDIA A100-80GB). See [docs/COMPARISON_REPORT.md](docs/COMPARISON_REPORT.md) for full breakdown, including 4/4 invariant compliance.*

### 2. Multi-Agent Concurrency & Heterogeneous Harness Benchmark (A100)

| Benchmark Stress Dimension | Concurrency / Load | Measured Metric / Throughput | Status | Engineering Assurance |
| :--- | :--- | :--- | :---: | :--- |
| **Simultaneous Read Bursts** | 12 Concurrent Agents, 120 queries | **18.9 QPS**, Avg: **614.29ms** (P50: 781.9ms) | **100% PASS** | Zero SQLite busy contention (`SQLITE_BUSY = 0`) |
| **Simultaneous Write & Flush Race** | 8 Concurrent Writers | **12.2 writes/sec**, Avg: **584.93ms** | **100% PASS** | 8/8 files created, zero collision loss (`-v2.md` to `-v8.md`) |
| **Instant Self-Healing Recall** | Read-your-writes validation | **290.99 ms** | **PASS ✓** | Immediate sub-second recall without re-indexing |
| **Context Garbage Collection (GC)** | 50-turn trajectory compaction | **94.67% TRR** (20,946 ➔ 1,110 tokens) | **PASS ✓** | Preserves context within cognitive Smart-Zone (15%–60%) |
| **Dynamic Loadout Git-Diff** | Change-aware ADR activation | **245 tokens** | **PASS ✓** | Strictly bounded under <300 token budget |
| **Heterogeneous Agent Loops** | Hermes, OpenClaw, Pi | Turnaround: **1.04s – 8.49s** | **100% PASS** | Verified autonomous tool calling & ADR crystallization |

---

## 🏗️ Architecture & Mechanics

### Four-Layer Decoupled Architecture

![K0maru-Agent-Memory: Architecture Overview](docs/images/architecture_overview.png)

1. **Interface Layer**: CLI Tools (`doctor`, `install`, `loadout`, `search`, `flush`, `distill`, `gc`, `guard`, pipe filters) + Embedded Dashboard (Axum + rust-embed) + FastMCP Stdio Service.
2. **Orchestration & Governance Layer**: Living ADR Guard Engine + State Machine Log Parser (Mermaid abstraction and slice storage) + Packager (<300 token Smart-Zone control) + Adaptive Convention Sniffer.
3. **Hybrid Search & Graph Layer**: SQLite FTS5 BM25 + `sqlite-vec` ONNX Embeddings (pluggable 384d/512d/768d) + Reciprocal Rank Fusion ($k=60$) + 1-Hop WikiLinks Graph Boost (+0.004).
4. **Storage & Persistence Layer**: Local Plaintext Markdown Vault (Single Source of Truth) + Disposable `cache.sqlite` (Transient full-text, vector, and adjacency index).

### Memory Lifecycle Loop

![Memory Lifecycle: LLM-Wiki Bidirectional Closed Loop](docs/images/memory_lifecycle.png)

- **Wake & Loadout** ➔ **Run & Offload** ➔ **Pre-Commit Guard** ➔ **Session Flush (ADR)** ➔ **Consolidation (Evergreen Cards)**

---

## 📖 CLI Cheat Sheet

Both `k0maru` and `k0maru-memory` binary names are supported identically.

| Command | Common Flags | Description |
| :--- | :--- | :--- |
| `k0maru init` *(alias: `bootstrap`)* | `--repo <path>` | Root repository directory to scan (default: current working directory) |
| | `--vault <path>` | Target wiki vault directory (default: `<repo>/.k0maru/wiki`) |
| | `--install` | Automatically wires FastMCP into detected agents and installs git/session hooks |
| | `--json` | Outputs machine-readable JSON bootstrap summary |
| `k0maru install` | `--vault <path>` | Injects `k0maru-memory` into agent client config files |
| | `--hooks` | Installs `.git/hooks/pre-commit` Living ADR guard and Claude Code SessionStart hook |
| | `--target <client>` | Target client (`all`, `claude`, `cursor`, `gemini`, `windsurf`, `cline`, `hermes`, `openclaw`, `omp`) |
| | `--dry-run` | Previews configuration changes without touching disk |
| `k0maru doctor` | `--vault <path>` | Full diagnostic check of binary, vault, cache, and client bindings |
| | `--json` | Outputs machine-readable JSON diagnostic report |
| `k0maru ui` | `--vault <path>` | Launches embedded visual developer console (`127.0.0.1:3721`) |
| | `--port <port>` | Binds custom local port |
| | `--open` | Automatically opens default browser upon startup |
| `k0maru loadout <query>` | `--vault <path>` | Target markdown vault directory |
| | `--diff` | Dynamically activates module-relevant ADRs by inspecting git diff |
| | `--copy` | Copies assembled <300 token loadout directly to clipboard |
| | `--json` | Outputs structured JSON loadout data |
| | `-l`, `--list` | Lists all active projects within vault |
| `k0maru search <query>` | `--vault <path>` | Target markdown vault directory |
| | `--mode <mode>` | Search mode (`hybrid` [default], `bm25`, `vector`) |
| | `--embed-model <name>`| Pluggable embedding model (`all-minilm-l6-v2`, `bge-small-zh-v1.5`, etc.) |
| | `--limit <N>` | Maximum search results to return (default: 5) |
| | `--json` | Outputs structured JSON search results |
| `k0maru sync` | `--vault <path>` | Incrementally scans and updates SQLite indices |
| | `--vector` | Computes and stores ONNX vector embeddings incrementally |
| | `--embed-model <name>`| Selects embedding model for vector generation |
| | `--force` | Forces full re-index ignoring cache timestamps |
| `k0maru offload` | `--threshold <N>`| Line count threshold for triggering symbolic offload (default: 50) |
| | `--task-id <id>` | Binds a task ID for traceable offloaded logs |
| `k0maru inspect <id>` | `<node_id>` | Retrieves persisted stack trace slice for a specific node ID |
| `k0maru flush` | `--vault <path>` | Crystallizes decisions into vault matching local conventions |
| | `--title <title>` | Note title |
| | `--category <cat>` | Note category (`decision`, `log`, `concept`, etc.) |
| | `--summary <text>` | Executive summary |
| | `--tags <t1,t2>` | Comma-separated tags |
| | `--related <r1,r2>`| Comma-separated related note titles (linked as `[[WikiLinks]]`) |
| | `--dry-run` | Previews destination path and generated note without writing |
| `k0maru distill` | `--trace-file <path>`| Distills raw execution trace into reusable skill note |
| | `--target <ecosystem>`| Export target: `default` (Obsidian) or `hermes` (Hermes Agent) |
| `k0maru gc` | `--input <path>` | Compacts multi-turn session trajectory with 4-tier GC protocol |
| | `--threshold <N>` | Line threshold for folding tool outputs (default: 20) |
| `k0maru guard check` | `--diff` | Audits git staged/unstaged diff against Living ADR rules (<10ms) |
| | `--strict` | Treats all warnings as blocking errors |
| | `--json` | Outputs structured JSON guard report |
| `k0maru guard install` | `--path <repo_dir>` | Installs `.git/hooks/pre-commit` hook to block non-compliant commits |
| `k0maru mcp` | `--vault <path>` | Starts stdio FastMCP service exposing all 8 memory tools |
| | `--sidecar` | Orthogonal 5-tool mode for `omp` (omits log offload and GC tools) |
| `k0maru pack export` | `--name <name> --tag <tag> -o <file.pack.json>` | Exports portable Memory Pack (.pack.json) of notes & ADRs |
| `k0maru pack import` | `<file.pack.json> [--dry-run]` | Imports Memory Pack into LLM-Wiki adapting to local topology |
| `k0maru pack inspect` | `<file.pack.json>` | Inspects pack contents, metadata, and predicted destination paths |

---

## 📊 Empirical Benchmarks: Cognitive Memory & Living ADR Guard

Empirically measured across a **Google Colab Linux VM** (Intel Xeon @ 2.20GHz, 12.67 GB RAM) protected by Zero-Burn Guard (`runtime.unassign()`), alongside local **Apple M5 Pro Mac** and **Linux Workstation (x86_64)**:

| Benchmark Dimension | Baseline (Full Markdown RAG / Manual CI) | Google Colab Linux VM | Apple M5 Pro Mac | Architectural Advantage & Impact |
| :--- | :---: | :---: | :---: | :--- |
| **Recall Payload Size (Single Memory)** | 611 bytes (112 Tokens) | **164 bytes (22 Tokens)** | **164 bytes (22 Tokens)** | **73.16% Token Reduction** via compact `!μ:` K-DSL wire |
| **Vault Index Sync (`k0maru sync`)** | 500ms ~ 2,000ms | **34.48 ms** | **9.50 ms** | High-performance incremental SQLite + ONNX sync |
| **Full Hybrid Search (BM25 + Vec)** | 1,500ms+ (Remote API RAG) | **495.04 ms** | **180.20 ms** | 100% local ONNX embedding + RRF reciprocal rerank |
| **Living ADR Guard (`guard check --diff`)** | Minutes (Human Review / CI) | **17.40 ms** | **4.20 ms** | Pre-commit regex & diff invariant verification (<20ms) |
| **Living ADR Rule Enforcement** | Ad-hoc / Drift-prone | **Strict Blocking (`deny`)** | **Strict Blocking (`deny`)** | Prevents architectural erosion before code review |

---

## 🔒 Privacy & Cleanroom Principles

- **100% Local & Zero Telemetry**: K0maru never collects, stores, or uploads your notes, code snippets, or queries. All indexing, vector inference, and guard evaluations run entirely on your physical machine.
- **Open-Domain Evaluation Compliance**: Benchmark data and rules are derived exclusively from public, compliant, and reproducible sources (SWE-bench agent trajectories, GitHub Actions public CI logs, BEIR/CoIR benchmarks). Zero private user data was involved. See [docs/TUNING_AND_DATA.md](docs/TUNING_AND_DATA.md).

---

## 🛠️ Customization & Tuning

- **Vault Convention Tuning**: Declare custom directory mappings in `.k0maru/rules.md` or `AGENTS.md` (e.g., `decisions: docs/adr`, `logs: 01_AI_Logs`).
- **Living ADR Invariants**: Add YAML frontmatter `invariants` to any ADR card in `20_Cards/` to immediately activate guardrails across your repositories.
- **Low-Resource Workstations**: On memory-constrained devices (<1GB RAM), run with `--mode bm25` for sub-millisecond keyword retrieval with zero ONNX memory overhead.
- **Offload Sensitivity**: Adjust `--threshold <N>` to match your model's context window. See [Developer Tuning Guide](docs/TUNING_AND_DATA.md).

---

## 🙏 Acknowledgments & Technical Ancestry

K0maru-Agent-Memory builds on foundational ideas from the open-source community:

1. **Andrej Karpathy ([LLM-Wiki](https://gist.github.com/karpathy))**: Knowledge compounding via flat Markdown files, bidirectional links, and "filesystem as single source of truth".
2. **Tencent Cloud ([TencentDB-Agent-Memory](https://github.com/TencentCloud/TencentDB-Agent-Memory))**: "Mermaid Symbolic Log Offloading" and "L0–L3 Semantic Hierarchy". Clean-room reimplemented in 100% Rust as a standalone CLI filter and disposable cache.
3. **Colby McHenry ([CodeGraph](https://github.com/colbymchenry/codegraph))**: Local Rust pre-indexing and graph topology models.
4. **Nous Research ([Hermes Agent](https://github.com/nousresearch/hermes-agent))**: Autonomous crystallization of skills from execution trajectories.
5. **Anthropic ([Model Context Protocol](https://modelcontextprotocol.io/))**: Standardized stdio JSON-RPC 2.0 communication.
6. **Open-Source Rust Ecosystem**: [`pulldown-cmark`](https://github.com/pulldown-cmark/pulldown-cmark), [`rusqlite`](https://github.com/rusqlite/rusqlite), [`sqlite-vec`](https://github.com/asg017/sqlite-vec), [`fastembed-rs`](https://github.com/Anush008/fastembed-rs), [`axum`](https://github.com/tokio-rs/axum), and [`clap`](https://github.com/clap-rs/clap).

---

## ⚖️ License

Distributed under the [MIT License](LICENSE).
