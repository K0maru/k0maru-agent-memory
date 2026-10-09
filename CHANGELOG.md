# Changelog / 更新日志

All notable changes to `K0maru-Agent-Memory` will be documented in this file.
本项目的所有重要版本更新均在此文件中记录（采用中英双语对照）。

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.8.0] - 2026-10-09

### 🚀 What's Changed (English)

#### Key Highlights & New Capabilities
- **Empirical A100 Open-Source Model Benchmarks**:
  - Measured empirical local inference performance on an **NVIDIA A100-SXM4-80GB GPU** across 5 industrial defect scenarios:
    - **Qwen2.5-Coder-32B**: Pass@1 jumped from 60.0% to **100.0% (+40%)**, latency accelerated **5.90x** (17.23s -> 2.92s);
    - **DeepSeek-R1-32B**: Reasoning reflection time cut by **-48.2%** (27.14s -> 14.07s, 1.93x faster), prompt tokens reduced by **-20.2%** (817 -> 652);
    - **DeepSeek-Coder-V2 (16B MoE)**: Delivered sub-second **1.05s** turnaround (**11.86x faster**);
    - **Qwen3.8-27B**: Maintained a flawless **100% Pass@1** baseline with **23.2% Token Reduction Ratio (TRR)** and halved turnaround on complex Rust compiler diagnostics.
- **Autonomous Agent Ecosystem Adaptation (Hermes Agent & OpenClaw)**:
  - Simulated an enterprise-grade LLM-Wiki repository (`payment-gateway`, L3 idempotency constraints, incident postmortems, and vault routing rules);
  - Drove **`Qwen3.8-27B`** through an autonomous Hermes / OpenClaw function calling loop with **100% verification across all 4 core invariants**:
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

### 🚀 详细更新日志（简体中文）

#### 核心亮点与新特性
- **云端 A100 开源大模型实测看板**:
  - 在 **NVIDIA A100-SXM4-80GB GPU** 上对 5 组工业级真实缺陷排查场景进行了严谨的本地端到端实测验证：
    - **Qwen2.5-Coder-32B**: 诊断成功率 Pass@1 从 60.0% 大幅跃升至 **100.0%（提升 +40%）**，推理延迟缩短 **5.90 倍**（从 17.23s 骤降至 2.92s）；
    - **DeepSeek-R1-32B**: 深度思考与反思推理耗时削减 **-48.2%**（从 27.14s 降至 14.07s，提速 1.93 倍），Prompt Token 消耗精简 **-20.2%**（817 -> 652）；
    - **DeepSeek-Coder-V2 (16B MoE)**: 达成亚秒级 **1.05s** 极速首轮修复决策（**提速 11.86 倍**）；
    - **Qwen3.8-27B**: 保持 **100% Pass@1** 极高基准准确率，Prompt Token 压缩率达到 **23.2%**，在复杂的 Rust 编译器报错长链路诊断中实现代码定位耗时减半。
- **智能体生态真实闭环适配（Hermes Agent 与 OpenClaw）**:
  - 完整模拟了企业级 LLM-Wiki 知识库（涵盖 `payment-gateway` 支付网关、L3 幂等性约束规则、线上故障复盘日志与架构路由规范）；
  - 驱动 **`Qwen3.8-27B`** 自主运行在真实 Hermes / OpenClaw 工具调用循环中，**100% 验证通过了四大核心不变量**：
    1. 飞行前架构记忆嗅探（先发制人执行 `get_project_loadout` 及 2 次针对性 `recall_memory`）；
    2. 生成完全符合业务规范的 Go 语言代码（包含 Redis 分布式锁 `SET NX EX 30` 与 `STATUS_PENDING` 防重状态机守卫）；
    3. 经验自愈与决策结晶（通过 `flush_session` 自动落盘归档至 `20_Cards/adr-pay-012-...md`）；
    4. 瞬时自愈检索（再次命中新结晶笔记仅需 **0.88 ms**，RRF 得分达 **12.05**，高居 Rank #1 榜首）。
- **经验提炼与 Hermes 动态技能协议（Trace-to-Skill）**:
  - 新增 `k0maru distill` 命令行工具以及 FastMCP 原生工具 `distill_session_skill`；
  - 原生对齐 Nous Research Hermes Agent 的动态技能协议格式（通过 `--target hermes` 生成标准规范），支持通过 `--export-hermes` 一键注入到 `~/.hermes/skills/` 目录。
- **智能体生态一键安装与诊断扩展**:
  - `k0maru install` 和 `k0maru doctor` 正式扩展支持 **Hermes Agent** 与 **OpenClaw**，现已完整覆盖 Claude Code、Cursor、Gemini CLI、Windsurf 及 Cline / Roo Code 全主流生态。
- **优质双语文档与主文档重构**:
  - 全面重构主页 `README.md` 与中文镜像 `README_zh.md`，将架构优势、差异化对比矩阵与极速起步章节置顶；
  - 撰写了详尽的对比评测技术白皮书（`docs/COMPARISON_REPORT.md` 与 `docs/COMPARISON_REPORT_zh.md`），并提供发表级可视化对比图表。

#### 质量与测试指标
- **195 / 195 单元与集成测试全部通过**，通过率 100%。
- `cargo clippy --all-targets -- -D warnings` 零警告。
- `cargo fmt --check` 代码格式规范验证通过。

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
