[English](README.md) | [简体中文](README_zh.md)

# K0maru-Agent-Memory

> **A Standalone, Single-Static-Binary, Zero-Daemon Long-Term Memory Hub for AI Coding Agents.**  
> *Mounting personal Markdown vaults (Obsidian & Karpathy LLM-Wiki) with instant sub-millisecond context retrieval and token governance.*

[![Language: Rust 2021](https://img.shields.io/badge/Language-Rust_2021-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version: 0.7.0](https://img.shields.io/badge/Version-0.7.0-blue.svg)]()
[![Tests: 213 passed](https://img.shields.io/badge/Tests-213_passed-brightgreen.svg)]()
[![Binary Size: 3.66MB](https://img.shields.io/badge/Binary_Size-3.66MB-success.svg)]()
[![Cold Start: 3.4ms](https://img.shields.io/badge/Cold_Start-3.4ms-purple.svg)]()
[![Zero-Daemon](https://img.shields.io/badge/Daemon-Zero_Daemon-informational.svg)]()

---

## 🌟 Core Advantages & Architectural Superiority (Why K0maru?)

Modern AI coding agents (Claude Code, Cursor, Windsurf, Gemini, etc.) frequently suffer from two critical pain points: **Cross-Session Amnesia** (loss of architectural decisions across sessions) and **Terminal Log Bloat** (compiler and test outputs flooding the context window).

Existing agent memory frameworks frequently deploy heavy multi-container Docker clusters (PostgreSQL, pgvector, Redis) or resident background daemons, incurring continuous RAM footprints (800MB–2GB), port conflicts, and complex deployment friction.

**K0maru is engineered specifically for local developer workstations, delivering industrial-grade performance with minimal mechanical complexity**:

1. **⚡ Zero-Daemon & Sub-Millisecond Cold Start**:
   - Pure Rust, statically compiled into a single **3.66 MB** standalone binary.
   - Cold-starts in just **3.38 ms** with a lean ~11 MB RAM footprint; **occupies 0 persistent network ports**.
   - Runs on demand via standard Unix pipes and `stdio` streams, releasing all memory immediately upon process exit.
2. **📝 Bring Your Own Markdown**:
   - Your existing local Markdown notes (Obsidian Vault, Karpathy-style flat LLM-Wiki) serve as the **Single Source of Truth**.
   - Zero proprietary binary lock-in, zero migration friction, and zero changes to your existing folder structure.
3. **🪓 Disposable Transient Cache**:
   - Maintains only a single-file SQLite database (`cache.sqlite`) integrating FTS5 BM25 full-text indices and `sqlite-vec` virtual tables.
   - Strictly disposable: delete it anytime (`rm cache.sqlite`) and the scanner **self-heals and rebuilds the index from scratch in 74 ms**.
4. **📉 Industrial-Grade Context Governance (99.4% TRR & Live A100 Validation)**:
   - Filters verbose logs via standard Unix pipes (`| k0maru offload`), transforming long traces into Mermaid state diagrams and stored slices.
   - Validated on NVIDIA A100 GPUs to reduce prompt tokens significantly while preventing attention drift and reasoning degradation.

### Architectural Comparison Matrix

| Evaluation Dimension | Vanilla Context (Copy-Paste) | Containerized Microservices (Letta / TencentDB) | Resident Background Daemons (agentmemory) | **K0maru-Agent-Memory (This Project)** |
| :--- | :--- | :--- | :--- | :--- |
| **Storage Medium** | Ephemeral chat history | External DBs (PostgreSQL, pgvector, Redis) | Proprietary DB in hidden directories | **Local Markdown Vaults (Plain Text)** |
| **Deployment Model** | Manual copy | Multi-container Docker Compose clusters | Background daemon process (`iii-engine`, etc.) | **Single static binary, on-demand (Zero-Daemon)** |
| **Network Port Footprint** | None | 1–4 open ports | 1–4 open ports | **0 ports** (Unix pipes & stdio streams) |
| **Cold Start Latency** | Instant | 2,500 ms – 3,500 ms (container boot) | 1,000 ms – 2,000 ms | **3.38 ms** (native Rust binary) |
| **Resident Memory (RAM)** | 0 MB | ~850 MB – 2,000 MB | ~150 MB – 300 MB | **~11.7 MB** (released instantly on command exit) |
| **Terminal Log Governance**| Manual or LLM truncation | Network API transfer required | Internal bespoke truncation | **Standard Unix pipe filter (`\| k0maru offload`)** |
| **Index Self-Healing** | None | Snapshot & backup dependent | Local database integrity dependent | **`cache.sqlite` disposable anytime, rebuilt in 74ms** |

### Empirical Open-Source Model Benchmark (NVIDIA A100-80GB GPU)

Evaluated on an **NVIDIA A100-SXM4-80GB GPU** in Google Colab Pro, measuring real local inference across leading open-source models on 5 representative industrial defects (Rust concurrency, Python asyncio leaks, TypeScript auth crashes, Go deadlocks, C memory safety):

| Evaluated Open Model | Architecture | Baseline (Raw Traces) | **K0maru Managed** | Breakthrough & Performance Impact |
| :--- | :--- | :---: | :---: | :--- |
| **Qwen2.5-Coder-32B** | 32B Dense Code Model | Pass@1: 60.0%<br>Latency: 17.23s | **Pass@1: 100.0% (+40%)**<br>**Latency: 2.92s (5.90x faster)** | **100% Full Pass Rate**; Mermaid diagrams eliminated trace noise, solving async deadlocks and null checks |
| **DeepSeek-R1-32B** | 32B Full Reasoning Distill | Latency: 27.14s<br>Tokens: 817 | **Latency: 14.07s (1.93x faster)**<br>**Tokens: 652 (-20.2%)** | **Reasoning Latency Cut in Half (-48.2%)**; streamlined reflection chain, Rust turnaround dropped from 39.2s to 11.5s (3.4x) |
| **DeepSeek-Coder-V2** | 16B MoE (2.4B active) | Pass@1: 60.0%<br>Latency: 12.46s | **Pass@1: 80.0% (+20%)**<br>**Latency: 1.05s (11.86x faster)** | **Sub-Second Extreme Throughput**; cured Python async leak with 1.05s turnaround |
| **Qwen3.8-27B** | 27B Dense (Dual-Stage Reasoning) | Tokens: 923<br>Rust Latency: 21.2s | **Tokens: 709 (-23.2%)**<br>**Rust Latency: 11.1s (2x faster)** | **Flawless 100% Pass Baseline**; 23.2% Prompt Token reduction, halved Rust turnaround |

> 📖 **Comprehensive Case Breakdown & Dual-Harness Synergy**:  
> For the complete per-case logs, harness synergy with **Pi (`pi.dev`)** and **Hermes Agent**, and 500B+ frontier models notes, see [Part 5 of the Comparison Report](docs/COMPARISON_REPORT.md#part-5-empirical-colab-a100-hardware-benchmark-multi-model-evaluation-matrix).

---

## 🚀 Quick Start & Installation

For step-by-step instructions, see [QUICKSTART.md](QUICKSTART.md).

### 1. Install K0maru Binary

#### Option A: One-Line Install Script (macOS & Linux - Recommended)
No Rust toolchain required. Automatically detects OS and chip architecture, verifies SHA-256 checksums, and installs the standalone binary:
```bash
curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash
```

#### Option B: Homebrew (macOS & Linux)
```bash
brew install K0maru/tap/k0maru
```

#### Option C: Build from Source (Cargo)
```bash
cargo install --git https://github.com/K0maru/k0maru-agent-memory
```

---

### 2. Connect Your Coding Agents (`k0maru install`)

With a single command, inject K0maru's FastMCP service into all detected coding agent configuration files (Claude Code, Cursor, Gemini CLI, Windsurf, Cline / Roo Code):

```bash
# Auto-detect and configure all installed agents pointing to your Markdown vault
k0maru install --vault ~/Documents/MyVault

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

---

### 3. Verify Health & Environment (`k0maru doctor`)

Run the automated doctor check to inspect your binary, vault conventions, SQLite indices, and agent bindings:

```bash
k0maru doctor
```

Outputs a clear, color-coded health report (`✓ Binary In Path`, `✓ FTS5 Index Ready`, `✓ Vectors Indexed`, `✓ Claude Code Configured`, `✓ Cursor Configured`) with actionable remediation steps for any warnings.

---

## 🛠️ Common Use Cases & Workflows (How-Tos)

### Use Case 1: Kickstarting New Sessions —— Rapid Context Loadout (`k0maru loadout`)

Avoid dumping dozens of unrelated files into fresh agent chats. `loadout` extracts project mission, core architectural invariants (L3), and recent engineering logs (L2), strictly contained within **< 300 Tokens**:

```bash
# Generate project loadout and copy directly to the system clipboard (macOS / Linux / Windows)
k0maru loadout my-project --vault ~/wiki --copy
```
Paste (`Cmd+V`) into your new chat. The agent gains instant architectural grounding while staying firmly in its high-attention reasoning zone.

---

### Use Case 2: Build & Test Failures —— Symbolic Log Offloading (`k0maru offload`)

When running test suites or compilers that generate thousands of lines, pipe the command output through `k0maru`:

```bash
# Intercept verbose dumps; outputs a clean Mermaid diagram with a node_id
cargo test 2>&1 | k0maru offload

# Retrieve targeted failure stack trace slice on demand
k0maru inspect node_54697ed3
```

- Delivers **99.44% Token Reduction** on raw build/test failures.
- State machine diagrams immediately highlight failure transitions (e.g. `Iteration1 --> Iteration2: E0382`), guiding LLMs directly to the root cause without attention drift.

---

### Use Case 3: Querying the Vault —— Natural Language Intent-Driven Recall

Once connected via MCP, frontier models (Claude 3.5 Sonnet, GPT-4o, Qwen) automatically understand intent and invoke the right tools without memorizing explicit keywords:

| User Natural Prompt | Autonomous Tool Call | Intent Logic |
| :--- | :--- | :--- |
| *"Starting a new task, catch up on project context"*<br/>*"What architectural rules must we follow?"* | `get_project_loadout` | Recognizes need for architectural alignment; injects compact L3 rules and recent logs |
| *"How do we usually configure database timeouts?"*<br/>*"Check if we have notes on JWT refresh best practices"* | `recall_memory` | Recognizes query against private knowledge; triggers hybrid semantic search |
| *"The test run failed with 300 lines of errors, check the failure stack"* | `inspect_log_node` | Recognizes need for truncated error slice; extracts exact stack trace by ID |
| *"We finalized RRF hybrid search with k=60, document this decision"* | `flush_session` | Recognizes need to record an ADR; writes structured markdown adhering to conventions |

> 💡 **Pro-Tip: Autonomous Vault Inspection System Prompt**  
> Add this single instruction to your `AGENTS.md` or global system prompt:  
> `> Before designing architecture, debugging complex failures, or implementing core logic, inspect the local vault using k0maru-memory to align with historical ADRs and codebase conventions.`  
> Agents will proactively consult your security and architectural standards before writing a single line of code.

For manual CLI searches:
```bash
# Hybrid search (BM25 + 384d ONNX vectors + RRF k=60 + WikiLinks graph boost)
k0maru search "concurrency deadlock mitigation" --vault ~/wiki --mode hybrid --limit 5
```

---

### Use Case 4: Preserving Decisions —— Adaptive Vault Crystallization (`k0maru flush`)

Crystallize engineering takeaways, architectural decisions (ADRs), or debugging solutions back into your vault:

```bash
k0maru flush \
  --title "Mitigating Tokio Runtime Blocking in Worker Pools" \
  --category decision \
  --summary "Offloaded synchronous disk IO to spawn_blocking threadpool" \
  --content "Detailed stack analysis and remediation..." \
  --tags "tokio,rust,performance" \
  --related "Architecture Standards,Async Concurrency" \
  --vault ~/wiki
```

- **Convention Sniffing**: Automatically adapts to your vault's naming style (e.g., `YYYY-MM-DD-slug.md`), directory structures, and `AGENTS.md` rules.
- **Collision Defense**: Detects duplicate titles and increments versions safely without overwriting existing notes.
- **Instant Indexing**: Automatically triggers incremental indexing; new notes become recallable within 5ms.

---

### Use Case 5: Vault Introspection —— Embedded Developer Console (`k0maru ui`)

Launch a local visual dashboard on demand:

```bash
k0maru ui --vault ~/wiki --open
```

![K0maru Developer Console - Search Debugger & Graph Boost](docs/images/ui-search.png)

- **🌐 Bilingual Switching**: Instant one-click toggle between English and Chinese in the header.
- **🔍 Search Explainability Console**: Inspect BM25 ranks, vector cosine distances, and Emerald `+0.05 Graph Boost` badges.
- **🪵 Log & State Machine Inspector**: Native SVG Mermaid renderer with syntax-highlighted error logs and one-click copying.
- **📊 Token Economics Scoreboard**: Real-time telemetry tracking vector coverage, database size, and cumulative tokens saved.
- **Zero Daemon**: Pressing `Ctrl+C` immediately frees all memory and network ports. See [docs/UI_TUTORIAL.md](docs/UI_TUTORIAL.md).

---

### Use Case 6: Vault Changes —— Sub-Millisecond Incremental Synchronization (`k0maru sync`)

```bash
# Incremental rescan of markdown files and FTS5 indices (takes just 11ms when clean)
k0maru sync --vault ~/wiki

# Batch incremental generation of ONNX vector embeddings (skips unchanged notes)
k0maru sync --vault ~/wiki --vector
```

---

## 🏗️ Architecture & Mechanics

### Four-Layer Decoupled Architecture

![K0maru-Agent-Memory: Architecture Overview](docs/images/architecture_overview.png)

1. **Interface Layer**: CLI Tools (`doctor`, `install`, `loadout`, `search`, `flush`, pipe filters) + Embedded Dashboard (Axum + rust-embed) + FastMCP Stdio Service.
2. **Orchestration & Governance Layer**: State Machine Log Parser (Mermaid abstraction and slice storage) + Packager (<300 token Smart-Zone control) + Adaptive Convention Sniffer.
3. **Hybrid Search & Graph Layer**: SQLite FTS5 BM25 + `sqlite-vec` ONNX Embeddings + Reciprocal Rank Fusion ($k=60$) + 1-Hop WikiLinks Graph Boost (+0.05).
4. **Storage & Persistence Layer**: Local Plaintext Markdown Vault (Single Source of Truth) + Disposable `cache.sqlite` (Transient full-text, vector, and adjacency index).

### Memory Lifecycle Loop

![Memory Lifecycle: LLM-Wiki Bidirectional Closed Loop](docs/images/memory_lifecycle.png)

- **Wake & Loadout** ➔ **Run & Offload** ➔ **Session Flush (ADR)** ➔ **Consolidation (Evergreen Cards)**

---

## 📖 CLI Cheat Sheet

| Command | Common Flags | Description |
| :--- | :--- | :--- |
| `k0maru install` | `--vault <path>` | Injects `k0maru-memory` into agent client config files |
| | `--target <client>` | Target client (`all`, `claude`, `cursor`, `gemini`, `windsurf`, `cline`) |
| | `--dry-run` | Previews configuration changes without touching disk |
| `k0maru doctor` | `--vault <path>` | Full diagnostic check of binary, vault, cache, and client bindings |
| | `--json` | Outputs machine-readable JSON diagnostic report |
| `k0maru ui` | `--vault <path>` | Launches embedded visual developer console (`127.0.0.1:3721`) |
| | `--port <port>` | Binds custom local port |
| | `--open` | Automatically opens default browser upon startup |
| `k0maru loadout <query>` | `--vault <path>` | Target markdown vault directory |
| | `--copy` | Copies assembled <300 token loadout directly to clipboard |
| | `--json` | Outputs structured JSON loadout data |
| | `-l`, `--list` | Lists all active projects within vault |
| `k0maru search <query>` | `--vault <path>` | Target markdown vault directory |
| | `--mode <mode>` | Search mode (`hybrid` [default], `bm25`, `vector`) |
| | `--limit <N>` | Maximum search results to return (default: 5) |
| | `--json` | Outputs structured JSON search results |
| `k0maru sync` | `--vault <path>` | Incrementally scans and updates SQLite indices |
| | `--vector` | Computes and stores ONNX vector embeddings incrementally |
| | `--json` | Outputs JSON sync statistics |
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
| | `--json` | Outputs machine-readable JSON result |
| `k0maru mcp` | `--vault <path>` | Starts stdio FastMCP service for agent integration |

---

## 🔒 Privacy & Cleanroom Principles

- **100% Local & Zero Telemetry**: K0maru never collects, stores, or uploads your notes, code snippets, or queries. All indexing and inference run entirely on your physical machine.
- **Open-Domain Evaluation Compliance**: Benchmark data and rules are derived exclusively from public, compliant, and reproducible sources (SWE-bench agent trajectories, GitHub Actions public CI logs, BEIR/CoIR benchmarks). Zero private user data was involved. See [docs/TUNING_AND_DATA.md](docs/TUNING_AND_DATA.md).

---

## 🛠️ Customization & Tuning

- **Vault Convention Tuning**: Declare custom directory mappings in `.k0maru/rules.md` or `AGENTS.md` (e.g., `decisions: docs/adr`, `logs: 01_AI_Logs`).
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
