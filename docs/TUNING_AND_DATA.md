# Tuning Guide & Data Provenance

This document outlines the **data provenance, privacy guarantees, and developer customization levers** in `K0maru-Agent-Memory`.

---

## 🔒 1. Data Privacy & Provenance Guarantees

K0maru is engineered under the **Cleanroom & Zero-Telemetry Principle**:

1. **Zero Data Ingestion / Exfiltration**:
   - K0maru never collects, stores, uploads, or telemeters any part of your Markdown notes, tasks, or query inputs.
   - All parsing, FTS5 lexical indexing, and ONNX vector embeddings occur **100% locally on your machine**.
   - No network ports are opened during standard operation; communication relies entirely on Unix stdio pipes.

2. **Open-Source & Public Benchmark Data Provenance**:
   All benchmark metrics (Token Reduction Ratio, cold-start latency, memory curves) and distillation pattern matchers are derived exclusively from **public, reproducible open-source corpora**:
   - **Compiler & Build Logs**: Extracted from public Rust (`cargo build`), TypeScript (`tsc`), and Go compilation runs across public repositories.
   - **Test & Trace Logs**: Public test failure dumps from [pytest](https://github.com/pytest-dev/pytest) and [Jest](https://github.com/jestjs/jest) test runners.
   - **Agent Trajectories**: Public execution logs from [SWE-bench](https://www.swebench.com/) and open GitHub Actions CI logs.
   - **Information Retrieval Benchmarks**: Standard retrieval evaluation suites from [BEIR](https://github.com/beir-cellar/beir) (MS MARCO, SciFact, NFCorpus) and [CoIR](https://github.com/project-miracle/coir) (Code Information Retrieval).
   - **Digital Garden Markdown**: Open-source documentation and public Quartz/Obsidian gardens (e.g., [The Rust Programming Language Book](https://github.com/rust-lang/book), MDN Web Docs).

> **Invariant**: No private user vaults or confidential development traces were ever used in training, quantizing, or benchmarking K0maru.

---

## 🛠️ 2. Developer Customization & Tuning Guide

Every developer has a distinct thinking flow, repository layout, and machine spec. Here is how you can tune K0maru to fit your exact setup.

### 2.1 Vault Layout & Category Routing

K0maru includes an **Adaptive Convention Sniffer** (`ConventionSniffer`) that automatically detects your vault style. If you want explicit control, create `.k0maru/rules.md` (or add to `AGENTS.md` / `CLAUDE.md` / `CONVENTIONS.md`) in your vault root:

```markdown
# K0maru Vault Rules

- decisions: docs/adr
- logs: 01_AI_Logs
- concepts: 20_Cards
- skills: playbooks
```

| Key | Supported Aliases | Default Fallback |
| :--- | :--- | :--- |
| `decisions` | `decision`, `adr` | Matches existing `decisions/`, `adr/`, or vault root |
| `logs` | `log`, `journal`, `daily`, `records` | Matches existing `logs/`, `journal/`, or vault root |
| `concepts` | `concept`, `cards`, `evergreen` | Matches existing `concepts/`, `cards/`, or vault root |
| `skills` | `skill`, `playbooks`, `recipes`, `troubleshooting` | Matches existing `skills/`, `playbooks/`, or vault root |

#### Custom Note Templates
Place Markdown templates in `templates/`, `_templates/`, or `.obsidian/templates/`. K0maru will automatically interpolate template variables:
- `{{title}}`: Note title
- `{{summary}}`: Executive summary
- `{{content}}`: Note body
- `{{date}}`: Date (YYYY-MM-DD)
- `{{tags}}`: YAML frontmatter tag list

---

### 2.2 Search & Hybrid Retrieval Tuning

K0maru provides three retrieval modes via `k0maru search <query> --mode <MODE>`:

```bash
# 1. Hybrid Mode (Default): Combines BM25 + Vector Dense + Graph Boost
k0maru search "sqlite lock timeout" --mode hybrid --limit 5

# 2. BM25 Lexical Mode: Pure token exact match (fastest, ultra-low memory)
k0maru search "fn handle_call_tool" --mode bm25 --limit 10

# 3. Dense Vector Mode: Pure semantic similarity
k0maru search "how to handle cross-session context" --mode vector --limit 5
```

#### How Reciprocal Rank Fusion (RRF) Works:
$$\text{RRF}(d) = \frac{0.5}{60 + \text{rank}_{\text{BM25}}(d)} + \frac{0.5}{60 + \text{rank}_{\text{Vector}}(d)} + \text{GraphBoost}(d)$$

- **When to choose BM25**: When searching for exact symbol names, error codes (`E0308`), specific file paths, or CLI flags. Requires **0 MB** ONNX model memory and completes in **<1 ms**.
- **When to choose Vector**: When searching conceptual topics or when your phrasing differs from the exact words in the notes.
- **Graph Boost (+0.05)**: Automatically promotes notes that have bidirectional WikiLinks connections to other top candidate notes.

---

### 2.3 Resource & Hardware Tuning (Lightweight vs Dense)

#### Ultra-Lightweight / Low-Memory Mode (VPS or CI Runners)
If running on a minimal VPS (<1GB RAM) or CI container:
- Run with `--mode bm25`: avoids initializing ONNX runtime entirely, using **<12MB RAM**.
- Or compile from source without the `fastembed` feature:
  ```bash
  cargo build --release --no-default-features
  ```
  This strips the ONNX embedding engine, producing a **~2.8MB** binary that performs instant BM25 and Graph retrieval.

#### Standard Embedded Vector Mode
- Default model: `all-MiniLM-L6-v2` (384 dimensions, ~80MB ONNX weight).
- Model weights are cached locally at `~/.k0maru/models/`.
- Pre-compute all embeddings ahead of time:
  ```bash
  k0maru sync --vector
  ```
  Subsequent queries will query the cached `sqlite-vec` virtual table in **<5 ms**.

---

### 2.4 Context Offload & Log Threshold Tuning

The log offloader intercepts stdout/stderr streams to prevent terminal dumps from exhausting agent context windows:

```bash
# Default threshold: 50 lines
cargo test 2>&1 | k0maru offload

# Tune threshold for shorter context models (e.g., 32k window)
cargo test 2>&1 | k0maru offload --threshold 25

# Tune threshold for long context models (e.g., 1M+ window)
cargo test 2>&1 | k0maru offload --threshold 100
```

When an offload occurs, inspect specific execution slices by node ID:
```bash
# View summary Mermaid diagram and error slice
k0maru inspect <node-id>
```

---

### 2.5 Developer Cockpit & UI Preferences

Launch the developer cockpit:
```bash
k0maru ui --vault ~/my-vault --port 3721 --open
```

- **Language Preference**: K0maru auto-detects system language and persists manual switches (`English` / `简体中文`) in browser `localStorage`.
- **Port Conflict Resolution**: Specify `--port <custom-port>` if 3721 is in use.
