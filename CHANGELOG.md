# Changelog / 更新日志

All notable changes to `K0maru-Agent-Memory` will be documented in this file.
本项目的所有重要版本更新均在此文件中记录（采用中英双语对照）。

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.8.0] - 2026-10-09

### What's Changed / 更新日志

#### 🚀 Key Highlights & New Capabilities / 核心亮点与新特性
- **Empirical A100 Open-Source Model Benchmarks / 云端 A100 开源大模型实测看板**:
  - Measured empirical local inference performance on an **NVIDIA A100-SXM4-80GB GPU** across 5 industrial defect scenarios:
    - **Qwen2.5-Coder-32B**: Pass@1 jumped from 60.0% to **100.0% (+40%)**, latency accelerated **5.90x** (17.23s -> 2.92s);
    - **DeepSeek-R1-32B**: Reasoning reflection time cut by **-48.2%** (27.14s -> 14.07s, 1.93x faster), prompt tokens reduced by **-20.2%** (817 -> 652);
    - **DeepSeek-Coder-V2 (16B MoE)**: Delivered sub-second **1.05s** turnaround (**11.86x faster**);
    - **Qwen3.8-27B**: Maintained a flawless **100% Pass@1** baseline with **23.2% Token Reduction Ratio (TRR)** and halved turnaround on complex Rust compiler diagnostics.
- **Autonomous Agent Ecosystem Adaptation (Hermes Agent & OpenClaw) / 智能体生态真实闭环适配**:
  - Simulated an enterprise-grade LLM-Wiki repository (`payment-gateway`, L3 idempotency constraints, incident postmortems, and vault routing rules);
  - Drove **`Qwen3.8-27B`** through an autonomous Hermes / OpenClaw function calling loop with **100% verification across all 4 core invariants**:
    1. Pre-flight memory inspection (`get_project_loadout` + 2x `recall_memory`);
    2. Invariant-compliant Go code generation with Redis mutex (`SET NX EX 30`) and `STATUS_PENDING` state machine guard;
    3. Self-healing decision crystallization via `flush_session` into `20_Cards/adr-pay-012-...md`;
    4. Instant self-healing recall in **0.88 ms** with a top score of **12.05** (Rank #1 hit).
- **Trace-to-Skill & Hermes Dynamic Skill Protocol / 经验提炼与动态技能对齐**:
  - Added `k0maru distill` CLI command and FastMCP `distill_session_skill` tool;
  - Supported Nous Research Hermes Agent dynamic skill schema via `--target hermes` and direct injection into `~/.hermes/skills/` via `--export-hermes`.
- **Ecosystem Installer & Diagnostics Expansion / 生态安装与诊断扩展**:
  - `k0maru install` and `k0maru doctor` now support **Hermes Agent** and **OpenClaw** alongside Claude Code, Cursor, Gemini CLI, Windsurf, and Cline / Roo Code.
- **Publication-Grade Documentation & Restructured README / 优质双语文档与主文档重构**:
  - Restructured `README.md` and `README_zh.md` to highlight core advantages, architectural comparison matrices, and quick start upfront;
  - Authored comprehensive comparative evaluation reports (`docs/COMPARISON_REPORT.md` & `docs/COMPARISON_REPORT_zh.md`) with publication-ready visualization figures.

#### 🧪 Quality & Test Metrics / 质量与测试指标
- **195 / 195 unit and integration tests passing** with 100% green rate.
- Zero warnings under `cargo clippy --all-targets -- -D warnings`.
- Codebase formatted cleanly under `cargo fmt --check`.

---

## [0.7.0] - 2026-10-07

### What's Changed / 更新日志
- **Trace-to-Skill Heuristic Extractor / 故障排障痕迹智能提炼**:
  - Added domain models and pattern matchers for Rust, Python, and shell error traces;
  - Introduced `NoteCategory::Skill` with auto-routing to `skills/` or `playbooks/`.
- **Cross-Platform Distribution & Packaging / 跨平台分发与安装套件**:
  - Added official one-line `install.sh` installation script with platform detection and SHA-256 verification;
  - Added Homebrew tap formula in `packaging/homebrew/`;
  - Added multi-platform GitHub Actions release workflow building for macOS, Linux, and Windows.

---

## [0.6.0] - 2026-10-04

### What's Changed / 更新日志
- **Adaptive Vault Convention Sniffer (`ConventionSniffer`) / 异构知识库拓扑自适应嗅探**:
  - Auto-detects directory conventions, naming styles (`date_slug` vs `slug_only`), templates, and YAML Frontmatter.
- **Crystallization Flush Engine (`FlushEngine`) / 经验结晶与原子落盘引擎**:
  - Provides safe write-backs with collision avoidance (`-v2.md`) and instant incremental cache synchronization;
  - Added `k0maru flush` CLI command and FastMCP `flush_session` tool.

---

## [0.5.0] - 2026-10-01

### What's Changed / 更新日志
- **One-Click Agent Ecosystem Installer (`k0maru install`) / 一键智能体生态挂载器**:
  - Safe, non-destructive JSON merge injecting `k0maru-memory` MCP config into Claude Code, Cursor, Gemini CLI, Windsurf, and Cline;
  - Added `--dry-run` configuration preview.
- **System Health & Diagnostic Engine (`k0maru doctor`) / 系统环境全方位诊断套件**:
  - Probes binary, PATH, vault markdown topology, SQLite cache size, FTS5 index, and client configs.

---

## [0.4.2] - 2026-10-01

### What's Changed / 更新日志
- **Developer Dark Cockpit WebUI Dashboard / 内嵌极客暗黑调试控制台**:
  - Embedded zero-daemon Axum web server (`k0maru ui`);
  - Search debugger with RRF score waterfall and Graph Boost indicators;
  - Offline SVG Mermaid diagram viewer and syntax-highlighted log slice inspector;
  - Bilingual frontend internationalization switcher (English / 简体中文).
