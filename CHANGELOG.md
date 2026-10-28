# Changelog

All notable changes to `K0maru-Agent-Memory` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.0.0] - 2026-10-27

### 🚀 What's Changed

#### Key Highlights & Production Release Contract
- **Zero-Config Cold-Start Bootstrap (`k0maru init` / `bootstrap`)**:
  - Instant `<repo>/.k0maru/wiki` generation from `README.md`, `AGENTS.md`, build manifests (`Cargo.toml`, `package.json`, `pyproject.toml`), and Git history;
  - Automatic `.gitignore` hygiene ensuring `.k0maru/cache.sqlite` and ephemeral artifacts are cleanly excluded while keeping Markdown assets versioned.
- **Proactive Lifecycle Hooks (`k0maru install --hooks` & `k0maru guard install`)**:
  - Official Claude Code `hooks.SessionStart` 3-layer schema (`matcher` + `hooks`) ensuring immediate `<300-token` project loadout injection at session launch;
  - Fail-closed Git pre-commit guard (`--diff --staged`) halting regressions before commit;
  - Safe, non-destructive JSONC merging with full comment and trailing-comma preservation.
- **Deterministic Memory Decay & Supersession (`k0maru flush --supersedes`)**:
  - Zero-LLM deterministic lifecycle state machine (`Active`, `Superseded`, `Deprecated`, `Archived`);
  - 16-hop transitive `superseded_by` resolution in `LoadoutBuilder` and `HybridSearchEngine`;
  - Complete belief-entrenchment reversal via $0.2\times$ BM25 penalty, $0.0$ graph boost, and `[SUPERSEDED by [[...]]]` header tags;
  - Automatic active `[Guard: ...]` invariant injection into `<300-token` dynamic loadouts.
- **Red-Team Hardened Living ADR Guard (`v1.0.0`)**:
  - 28-vector adversarial hardening covering diff-header injection attacks, staged-index vs. working-tree `MustContain` verification, and `HEAD` baseline tamper protection;
  - Multi-project slug isolation preventing cross-project rule leakage;
  - Balanced-bracket and word-boundary rule extraction (`forbid`, `require`, `regex`) with strict CWD independence.

---

## [0.11.0] - 2026-10-13

### 🚀 What's Changed

#### Key Highlights & New Capabilities
- **Living ADR Guard & Architectural Invariant Sentinel**:
  - Implemented `k0maru guard` CLI command and FastMCP `guard_check` tool to continuously audit code modifications and Git diffs against active ADRs and architectural invariants;
  - Zero-overhead regex and literal diff inspection preventing architectural erosion, cross-layer leakage, and raw SQL injections before commit.
- **Pluggable Embeddings Architecture & Dynamic Dimension Storage**:
  - Modular vector backend supporting dynamic dimension migration without wiping existing databases;
  - Auto-detection and adaptation to various embedding models.
- **First-Class Project Catalog & Scoped Push-Down Resolver**:
  - Enhanced `get_project_loadout` with scoped push-down resolver and multi-tier loadout aggregation;
  - Project alias normalization and adversarial project resolution support.
- **CI/CD Multi-Platform Distribution Pipeline Self-Healing**:
  - Added automated GitHub Release creation (`taiki-e/create-gh-release-action`) across all supported platforms;
  - Switched Linux aarch64 runner to native `ubuntu-24.04-arm` and migrated macOS runners to `macos-latest`;
  - Packaged both `k0maru` and `k0maru-memory` binary alias in release archives.

---

## [0.8.1] - 2026-10-09

### 🚀 What's Changed

#### Key Highlights & New Capabilities
- **Publication-Grade VitePress Documentation Engine**:
  - Launched official bilingual documentation website powered by VitePress with a tailored Developer Dark Cockpit theme (`#0F172A` background, `#22C55E` emerald accent, `#1B2336` cards, `JetBrains Mono` code blocks);
  - Implemented 100% symmetrical chapter parity between English (`docs/en/`) and Chinese (`docs/zh/`) with instant language switching;
  - Built-in zero-SaaS, offline-first Minisearch local search indexing both English and Chinese content in <2ms;
  - Comprehensive documentation covering: Product Philosophy & BYOM, 10-dimensional Competitor Matrices, Step-by-Step Installation & Quickstart, 4-Layer Decoupled Architecture & L0-L3 Memory Hierarchy, 8 Key Implementation Deep Dives, Ecosystem FastMCP Integration (Claude Code, Cursor, Windsurf, Hermes Agent, OpenClaw), Industrial Production Scenarios, Exhaustive CLI & MCP Schemas Reference, and Empirical A100 Benchmarks;
  - Automated GitHub Pages deployment pipeline via `.github/workflows/deploy-docs.yml`.
- **Editorial & Style Standards Compliance**:
  - English documentation strictly complies with the **Google Developer Documentation Style Guide** (second-person "you", active voice, present tense, descriptive headings, parallel list items);
  - Chinese documentation strictly complies with **阮一峰《中文技术文档写作规范》** and **《中文文案排版指北》** (Pangu spacing between Chinese and English/numbers, full-width punctuation, standard proper noun casing).

#### Quality & Test Metrics
- VitePress static build verified in 1.42s with 0 broken links and 0 errors.
- 232 / 232 unit and integration tests passing with 100% green rate.
- Zero warnings under `cargo clippy --all-targets -- -D warnings`.
- Codebase formatted cleanly under `cargo fmt --check`.

---

## [0.8.0] - 2026-10-09

### 🚀 What's Changed

#### Key Highlights & New Capabilities
- **Empirical A100 Open-Source Model Benchmarks**:
  - Measured empirical local inference performance on an **NVIDIA A100-SXM4-80GB GPU** across 5 industrial defect scenarios:
    - **Qwen2.5-Coder-32B**: Pass@1 jumped from 60.0% to **100.0% (+40%)**, latency accelerated **5.90x** (17.23s -> 2.92s);
    - **DeepSeek-R1-32B**: Reasoning reflection time cut by **-48.2%** (27.14s -> 14.07s, 1.93x faster), prompt tokens reduced by **-20.2%** (817 -> 652);
    - **DeepSeek-Coder-V2 (16B MoE)**: Delivered sub-second **1.05s** turnaround (**11.86x faster**);
    - **Qwen3.8-27B**: Maintained a flawless **100% Pass@1** baseline with **23.2% Token Reduction Ratio (TRR)** and halved turnaround on complex Rust compiler diagnostics.
- **Autonomous Agent Ecosystem Adaptation (Hermes Agent & OpenClaw)**:
  - Simulated an enterprise-grade LLM-Wiki repository (`payment-gateway`, L3 idempotency constraints, incident postmortems, and vault routing rules);
  - Driven **`Qwen3.8-27B`** through an autonomous Hermes / OpenClaw function calling loop with **100% verification across all 4 core invariants**:
    1. Pre-flight memory inspection (`get_project_loadout` + 2x `recall_memory`);
    2. Invariant-compliant Go code generation with Redis mutex (`SET NX EX 30`) and `STATUS_PENDING` state machine guard;
    3. Self-healing decision crystallization via `flush_session` into `20_Cards/adr-pay-012-...md`;
    4. Instant self-healing recall in **0.88 ms** with a top score of **12.05** (Rank #1 hit).
- **Trace-to-Skill & Hermes Dynamic Skill Protocol**:
  - Added `k0maru distill` CLI command and FastMCP `distill_session_skill` tool;
  - Supported Nous Research Hermes Agent dynamic skill schema via `--target hermes` and direct injection into `~/.hermes/skills/` via `--export-hermes`.
- **Ecosystem Installer & Diagnostics Expansion**:
  - `k0maru install` and `k0maru doctor` now support **Hermes Agent** and **OpenClaw** alongside Claude Code, Cursor, Gemini CLI, Windsurf, and Cline / Roo Code.
- **Publication-Grade Documentation & Restructured README**:
  - Restructured `README.md` and `README_zh.md` to highlight core advantages, architectural comparison matrices, and quick start upfront;
  - Authored comprehensive comparative evaluation reports (`docs/COMPARISON_REPORT.md` & `docs/COMPARISON_REPORT_zh.md`) with publication-ready visualization figures.

#### Quality & Test Metrics
- **195 / 195 unit and integration tests passing** with 100% green rate.
- Zero warnings under `cargo clippy --all-targets -- -D warnings`.
- Codebase formatted cleanly under `cargo fmt --check`.

---

## [0.7.0] - 2026-10-07

### 🚀 What's Changed
- **Trace-to-Skill Heuristic Extractor**:
  - Added domain models and pattern matchers for Rust, Python, and shell error traces;
  - Introduced `NoteCategory::Skill` with auto-routing to `skills/` or `playbooks/`.
- **Cross-Platform Distribution & Packaging**:
  - Added official one-line `install.sh` installation script with platform detection and SHA-256 verification;
  - Added Homebrew tap formula in `packaging/homebrew/`;
  - Added multi-platform GitHub Actions release workflow building for macOS, Linux, and Windows.

---

## [0.6.0] - 2026-10-04

### 🚀 What's Changed
- **Adaptive Vault Convention Sniffer (`ConventionSniffer`)**:
  - Auto-detects directory conventions, naming styles (`date_slug` vs `slug_only`), templates, and YAML Frontmatter.
- **Crystallization Flush Engine (`FlushEngine`)**:
  - Provides safe write-backs with collision avoidance (`-v2.md`) and instant incremental cache synchronization;
  - Added `k0maru flush` CLI command and FastMCP `flush_session` tool.

---

## [0.5.0] - 2026-10-01

### 🚀 What's Changed
- **One-Click Agent Ecosystem Installer (`k0maru install`)**:
  - Safe, non-destructive JSON merge injecting `k0maru-memory` MCP config into Claude Code, Cursor, Gemini CLI, Windsurf, and Cline;
  - Added `--dry-run` configuration preview.
- **System Health & Diagnostic Engine (`k0maru doctor`)**:
  - Probes binary, PATH, vault markdown topology, SQLite cache size, FTS5 index, and client configs.

---

## [0.4.2] - 2026-10-01

### 🚀 What's Changed
- **Developer Dark Cockpit WebUI Dashboard**:
  - Embedded zero-daemon Axum web server (`k0maru ui`);
  - Search debugger with RRF score waterfall and Graph Boost indicators;
  - Offline SVG Mermaid diagram viewer and syntax-highlighted log slice inspector;
  - Bilingual frontend internationalization switcher (English / 简体中文).
