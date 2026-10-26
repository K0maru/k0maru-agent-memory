# Changelog / 更新日志

All notable changes to `K0maru-Agent-Memory` will be documented in this file.
本项目的所有重要版本更新均在此文件中记录（采用中英双语对照）。

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.0.0] - 2026-10-27

### 🚀 What's Changed (English)

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

### 🚀 详细更新日志（简体中文）

#### 核心亮点与正式版发布契约
- **零配置极速冷启动引导（`k0maru init` / `bootstrap`）**:
  - 瞬时从 `README.md`、`AGENTS.md`、项目构建清单（`Cargo.toml`、`package.json`、`pyproject.toml`）与 Git 提交历史全自动生成 `<repo>/.k0maru/wiki` 架构知识库；
  - 自动化 `.gitignore` 目录卫生治理，确保 `.k0maru/cache.sqlite` 与临时索引被干净忽略，而纯文本 Markdown 架构资产纳入版本控制。
- **主动生命周期钩子（`k0maru install --hooks` 与 `k0maru guard install`）**:
  - 全面支持官方 Claude Code `hooks.SessionStart` 三层规范（`matcher` + `hooks`），实现编码会话启动时 `<300-token` 架构上下文秒级自动注入；
  - Fail-closed 安全闭锁式 Git pre-commit 守卫（`--diff --staged`），在代码暂存提交前毫秒级阻断架构退化；
  - 采用无损 JSONC 语法树合并算法，完整保留用户原有配置中的注释与尾随逗号。
- **确定性记忆衰减与新旧决策取代（`k0maru flush --supersedes`）**:
  - 零 LLM 确定性状态机生命周期模型（`Active`、`Superseded`、`Deprecated`、`Archived`）；
  - 在 `LoadoutBuilder` 与 `HybridSearchEngine` 中支持深达 16 跳的可传递 `superseded_by` 溯源解析；
  - 彻底破除历史双链“信念固化 (Belief Entrenchment)”偏见，施加 $0.2\times$ 权重惩罚、归零图谱加成，并自动插入 `[SUPERSEDED by [[...]]]` 警示；
  - 自动将新一代活跃 ADR 的 `[Guard: ...]` 守卫不变量前置注入 `<300-token` 动态装配中。
- **红蓝对抗加固型活体架构守卫（Living ADR Guard v1.0.0）**:
  - 覆盖 28 种对抗性边界向量深度防御，包括 Diff 标头注入伪造、暂存区 Index 对比工作区 Tree 的 `MustContain` 严格一致性校验、以及 `HEAD` 基线防篡改防护；
  - 多项目 Slug 严格隔离，杜绝跨子项目规则串扰；
  - 支持括号平衡匹配与单词边界解析的规则抽取（`forbid`、`require`、`regex`），且完全工作目录无关（CWD-independent）。

---

## [0.11.0] - 2026-10-13

### 🚀 What's Changed (English)

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

### 🚀 详细更新日志（简体中文）

#### 核心亮点与新特性
- **活体架构守卫哨兵（Living ADR Guard）**:
  - 新增 `k0maru guard` 命令行与 FastMCP `guard_check` 核心工具，在 Git Commit 与 PR 审查前自动对照活跃 ADR 与项目不变量进行增量 Diff 审计；
  - 零开销正则与字面量增量 Diff 规则检查，彻底阻断跨层调用泄漏与架构退化。
- **可插拔向量嵌入架构与动态维度迁移**:
  - 实现向量存储引擎的多维度动态平滑迁移，支持在切换不同嵌入模型时无需整库重建；
  - 提升局部向量索引稳定性。
- **一流项目目录与下推解析器**:
  - 强化 `get_project_loadout`，支持项目标识归一化、别名映射与多级 Loadout 聚合装配。
- **全平台 CI/CD 发版流水线自愈与 Runner 矩阵升级**:
  - 引入自动化 GitHub Release 生成流水线，修复历史 release 丢失与资源 404 问题；
  - Linux aarch64 接入 GitHub Actions 官方原生 `ubuntu-24.04-arm` Runner，macOS 统一迁移至 `macos-latest`；
  - 发布压缩包中同时打包 `k0maru` 主二进制及 `k0maru-memory` 别名。

---

## [0.8.1] - 2026-10-09

### 🚀 What's Changed (English)

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

### 🚀 详细更新日志（简体中文）

#### 核心亮点与新特性
- **发表级 VitePress 双语技术文档系统上线**:
  - 正式上线基于 VitePress 构建的官方双语技术文档站点，定制极客暗黑主题（Developer Dark Cockpit，采用 `#0F172A` 底色、`#22C55E` 翡翠绿重音、`#1B2336` 卡片表面及 `JetBrains Mono` 等宽字体，与项目内嵌控制台美学高度契合）；
  - 达成英文版（`docs/en/`）与中文版（`docs/zh/`）100% 章节严格对称与顶栏无缝一键切换；
  - 内置零 SaaS 依赖、离线优先的本地 Minisearch 全文检索引擎，实现中英文关键词毫秒级精准定位与高亮跳转；
  - 完整覆盖 11 大核心板块：产品哲学与 BYOM 原则、10 维度竞品全景对比矩阵、5 大安装路径与 3 分钟极速上手、四层解耦架构模型与 L0~L3 知识分级、8 大核心实现技术剖析、全生态 Agent 挂载（Claude Code, Cursor, Windsurf, Hermes, OpenClaw 等）、工业级生产场景实战复盘、CLI 与 FastMCP 协议接口详尽规范、NVIDIA A100 实测看板与 Token 经济学；
  - 配置自动化 GitHub Pages 部署流水线（`.github/workflows/deploy-docs.yml`）。
- **专业排版与写作规范合规**:
  - 英文文档严格遵循 **Google Developer Documentation Style Guide**（第二人称 "you"、主动语态、现在时、动作导向标题与平行列表结构）；
  - 中文文档严格遵循 **阮一峰《中文技术文档写作规范》** 与 **《中文文案排版指北》**（中英文与数字间保留半角空格盘古之白、全角中文标点、官方专有名词严格大小写、代码块语言全标注）。

#### 质量与测试指标
- VitePress 静态构建在 1.42 秒内完成，0 死链、0 警告。
- 232 / 232 单元与集成测试全部通过，通过率 100%。
- `cargo clippy --all-targets -- -D warnings` 零警告。
- `cargo fmt --check` 代码格式规范验证通过。

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
