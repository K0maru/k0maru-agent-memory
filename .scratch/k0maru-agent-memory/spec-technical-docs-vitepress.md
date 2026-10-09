# SPEC: K0maru-Agent-Memory VitePress-Powered Bilingual Technical Documentation Suite

## 1. Executive Summary & Goals

This specification defines the complete architecture, content structure, internationalization design, and automated deployment pipeline for the official **K0maru-Agent-Memory Bilingual Technical Documentation Suite**.

The documentation suite serves as the definitive reference manual for developers, system architects, and autonomous AI agents integrating K0maru-Agent-Memory into production workflows. It replaces the fragmented single-file README experience with a publication-grade, developer-cockpit styled static site powered by **VitePress**, while keeping all underlying content in 100% standard, clean Markdown files.

### Core Objectives
1. **Developer Dark Cockpit Aesthetic**: Tailored VitePress theme matching `#0F172A` (background), `#1B2336` (cards), `#22C55E` (emerald accent), and `JetBrains Mono` code blocks, perfectly mirroring the built-in `k0maru ui` console.
2. **First-Class Bilingual Architecture (EN / ZH)**: Symmetrical documentation trees under `docs/en/` and `docs/zh/` with instant language switching in the top navigation bar.
3. **Zero-SaaS, Offline-First Instant Search**: Built-in Minisearch local search indexing both English and Chinese documentation without external telemetry or SaaS tokens.
4. **Comprehensive Technical Coverage**:
   - Project Introduction, Philosophy, and Competitive Matrices
   - 4-Layer Decoupled Architecture & Memory Hierarchy (L0–L3)
   - Deep Dive into Key Implementations (`xxh3`, Hybrid RRF + 1-hop Graph Boost, FastMCP, Log Offload, Convention Sniffer, Flush Engine, Trace-to-Skill)
   - Step-by-Step Installation & 3-Minute Quickstart
   - Ecosystem Integration & Multi-Agent Workflows (Claude Code, Cursor, Windsurf, Hermes Agent, OpenClaw)
   - Complete CLI & FastMCP Stdio Protocol Reference
   - Empirical NVIDIA A100 SXM4 80GB Benchmarks & Token Reduction Economics
5. **GitBook & Plain Markdown Compatibility**: Every documentation file is standard GitHub-Flavored Markdown (`.md`), enabling direct reading on GitHub, import into GitBook, or local inspection in Obsidian/VS Code.
6. **Automated CI/CD**: Seamless GitHub Actions workflow deploying to GitHub Pages upon merge to `main`.

---

## 1.1 Editorial & Typography Standards (中英文技术文档规范)

### 中文技术文档规范 (参考：阮一峰《中文技术文档写作规范》与《中文文案排版指北》)
1. **中英文盘古之白（空格规范）**：
   - 中文文字与英文单词之间，必须留有 1 个半角空格（例如：`运行 k0maru doctor 命令`，禁止 `运行k0maru doctor命令`）；
   - 中文文字与半角数字之间，必须留有 1 个半角空格（例如：`耗时仅需 0.88 ms`）；
   - 英文标点紧跟英文单词，与后续中文之间保留空格；
2. **标点符号规范**：
   - 完整中文句子内部与末尾一律使用全角标点（`，`、`。`、`：`、`？`、`！`）；
   - 英文缩写或代码引用内部使用半角标点；
3. **专有名词与拼写大小写**：
   - 严格遵循官方大小写，杜绝随意缩写：如 `GitHub`（非 github/Github）、`Rust`、`SQLite`（非 sqlite/Sqlite）、`FastMCP`、`Docker`、`NVIDIA A100`、`Claude Code`、`Windsurf`、`Hermes Agent`、`OpenClaw`、`JSON-RPC`、`API`；
4. **行内代码与引用**：
   - 所有的命令、文件路径、参数、变量名必须使用反引号包覆（例如：`` `k0maru search` ``、`` `cache.sqlite` ``、`` `--vault` ``）；
5. **文风与语气**：
   - 客观中立、去情绪化，使用祈使句与陈述句，突出“行动步骤”与“预期输出”，禁止出现模糊指代。

### English Documentation Standards (Based on Google Developer Documentation Style Guide)
1. **Active Voice & Present Tense**:
   - Write in the active voice and present tense (e.g., "K0maru indexes Markdown notes incrementally" instead of "Markdown notes will be indexed by K0maru").
2. **Second Person ("You")**:
   - Address the developer directly using "you" and imperative mood for steps (e.g., "Run `k0maru loadout` to inspect the bootstrap context"). Avoid first-person plurals ("we") in instructional passages.
3. **Descriptive & Actionable Headings**:
   - Use clear sentence case or title case consistently. Headings must indicate the topic or action clearly.
4. **Parallel List Construction**:
   - Ensure all items in bulleted and numbered lists share the same grammatical form (all imperatives or all noun phrases).
5. **Exact Code & Command Formatting**:
   - Use inline code formatting for file names, paths, flags, commands, and code symbols.
   - Specify language tags (`bash`, `rust`, `json`, `yaml`, `mermaid`) on all fenced code blocks.

---

## 2. Directory Structure & File Layout

```text
/Users/k0maru3/workspace/k0maru-agent-memory/
├── package.json                          # VitePress scripts & devDependencies
├── .gitignore                            # Excludes node_modules, cache, dist
├── docs/
│   ├── .vitepress/
│   │   ├── config.mts                    # Central VitePress configuration (i18n, nav, sidebar, search)
│   │   └── theme/
│   │       ├── index.ts                  # Theme entry
│   │       └── custom.css                # Dark Cockpit CSS overrides
│   ├── index.md                          # Root landing / redirect
│   ├── images/                           # Shared diagram & benchmark assets
│   ├── zh/                               # 简体中文技术文档
│   │   ├── index.md                      # 中文主页 Hero
│   │   ├── guide/
│   │   │   ├── introduction.md           # 项目介绍、设计哲学与差异化对比
│   │   │   ├── installation.md           # 完整安装指南 (Shell, Brew, Cargo, Binary, Source)
│   │   │   └── quickstart.md             # 3 分钟极速起步与端到端自愈检验
│   │   ├── architecture/
│   │   │   ├── overview.md               # 四层解耦架构与核心数据流
│   │   │   └── key-implementations.md    # 核心算法与关键模块深度实现剖析
│   │   ├── workflows/
│   │   │   ├── ecosystem.md              # 智能体生态挂载 (Claude, Cursor, Windsurf, Hermes, OpenClaw)
│   │   │   └── scenarios.md              # 经典工业级场景 (LLM-Wiki 维护、故障自愈、动态技能提炼)
│   │   ├── reference/
│   │   │   ├── cli.md                    # CLI 完整命令与参数手册
│   │   │   └── mcp.md                    # FastMCP 协议接口与 Stdio 契约规范
│   │   └── benchmarks/
│   │       └── a100-evaluation.md        # A100 实测全景看板与 Token 压缩率经济学
│   └── en/                               # English Technical Documentation
│       ├── index.md                      # English Hero Landing Page
│       ├── guide/
│       │   ├── introduction.md           # Introduction, Philosophy & Competitive Matrices
│       │   ├── installation.md           # Installation Guide (Shell, Brew, Cargo, Binary, Source)
│       │   └── quickstart.md             # 3-Minute Quickstart & End-to-End Self-Healing Verification
│       ├── architecture/
│       │   ├── overview.md               # 4-Layer Decoupled Architecture & Data Flow
│       │   └── key-implementations.md    # Deep Dive: Key Algorithms & Modular Implementations
│       ├── workflows/
│       │   ├── ecosystem.md              # Agent Ecosystem Integration (Claude, Cursor, Windsurf, Hermes, OpenClaw)
│       │   └── scenarios.md              # Industrial Production Scenarios (LLM-Wiki, Self-Healing, Skill Distill)
│       ├── reference/
│       │   ├── cli.md                    # CLI Subcommands & Parameters Reference
│       │   └── mcp.md                    # FastMCP Stdio Protocol & Tool Contracts
│       └── benchmarks/
│           └── a100-evaluation.md        # Empirical A100 Benchmarks & Token Reduction Economics
└── .github/
    └── workflows/
        └── deploy-docs.yml               # GitHub Actions for automated GitHub Pages deployment
```

---

## 3. Detailed Content Specifications by Chapter

### 3.1 Chapter 1: Introduction & Philosophy (`guide/introduction.md`)
- **Product Definition**: Single-static-binary, zero-daemon, transparent long-term memory hub for AI coding agents.
- **The "Bring Your Own Markdown" (BYOM) Principle**: Human-readable, transparent, zero-vendor-lockin memory.
- **Problem Statement**:
  - Context amnesia across terminal sessions.
  - Terminal compiler dump explosion (1,000+ lines of raw traces polluting context windows).
  - Microservice bloat: Why spinning up Docker, heavy Vector DBs, and Python runtimes for local AI memory is an anti-pattern.
- **Architectural Comparison Matrix**: Direct head-to-head evaluation against Mem0, Letta (MemGPT), Chroma, and basic SQLite embeddings across memory footprint, cold start latency, daemon requirements, format lock-in, and agent tool compatibility.

### 3.2 Chapter 2: Installation (`guide/installation.md`)
- **Method A: Official One-Line Script**: `curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash` with SHA-256 verification and automatic PATH export.
- **Method B: Homebrew Tap**: `brew tap K0maru/k0maru-agent-memory && brew install k0maru`.
- **Method C: Cargo Installation**: `cargo install k0maru-agent-memory`.
- **Method D: Pre-compiled GitHub Releases**: Direct binary downloads for `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`.
- **Method E: Building from Source**: Prerequisites (Rust 1.75+, CMake, clang), build flags, and verification via `cargo test`.

### 3.3 Chapter 3: Quickstart (`guide/quickstart.md`)
- **Phase 1: Environment Diagnostic (`k0maru doctor`)**: Verifying binary, PATH, client configs, and storage.
- **Phase 2: Project Context Loadout (`k0maru loadout --copy`)**: Instant generation of <300 token system bootstrap summary.
- **Phase 3: Symbolic Log Offloading (`k0maru offload`)**: Compressing 1,000+ line terminal trace into a 15-line state machine diagram.
- **Phase 4: Hybrid Search & Knowledge Recall (`k0maru search`)**: RRF hybrid search fusing BM25 lexical match, ONNX vector embeddings, and 1-hop WikiLinks graph boost.
- **Phase 5: Decision Crystallization & Self-Healing (`k0maru flush`)**: Writing structured architectural decision records back to Markdown with collision avoidance.

### 3.4 Chapter 4: Architecture & Design (`architecture/overview.md`)
- **4-Layer Decoupled Architecture**:
  1. *Presentation & Protocol Layer*: CLI subcommands, FastMCP stdio server, Axum WebUI console.
  2. *Domain & Processing Layer*: Scanner state machine, Convention sniffer, Distill heuristic engine, Flush writer.
  3. *Retrieval & Fusion Layer*: BM25 FTS5 lexical engine, ONNX FastEmbed embedding engine, Reciprocal Rank Fusion (RRF $k=60$), 1-Hop Graph Boost (+0.05).
  4. *Storage Layer*: Standard Markdown Filesystem (Source of Truth), Disposable SQLite Cache (`cache.sqlite`), `sqlite-vec` virtual table.
- **Memory Hierarchy Tiering**:
  - `L3 Evergreen`: Architecture Decision Records (`20_Cards/adr-*.md`), invariant specifications.
  - `L2 Decision Logs`: Session records, incident postmortems, task milestones.
  - `L1 Resources`: Third-party APIs, vendor documentation, external bookmarks.
  - `L0 Scratch / Dailies`: Ephemeral traces, raw terminal logs, scratch notes.
- **Memory Lifecycle State Diagram**: Mermaid state machine illustrating Read -> Ingest -> Cache -> Search -> Offload -> Distill -> Flush loop.

### 3.5 Chapter 5: Key Implementations Deep Dive (`architecture/key-implementations.md`)
- **Incremental File Scanner**: `xxh3` hashing, mtime boundary caching, dirty file detection, zero-unnecessary I/O.
- **Hybrid Retrieval & RRF Fusion**:
  - Math formula: $RRF(d) = \sum_{m \in \{bm25, vector\}} \frac{w_m}{k + rank_m(d)} + GraphBoost(d)$
  - 1-hop bidirectional Graph Boost rationale and implementation.
- **FastMCP Stdio Server**: Zero-daemon IPC, natural language intent recognition, stdio transport robustness, signal handling.
- **Symbolic Log Offloading**: Terminal trace parsing, state machine extraction, pure SVG Mermaid generation without headless browser or CDN dependencies.
- **Adaptive Vault Convention Sniffer**: Directory topological inference, naming style detection (`date_slug` vs `slug_only`), frontmatter detection, template interpolation.
- **Crystallization Flush Engine**: Atomic writes, anti-collision suffixing (`-v2.md`), automatic post-flush cache index synchronization.
- **Trace-to-Skill Heuristic Distiller**: Pattern matching across Rust, Python, and shell compiler errors, heuristic remediation extraction, dynamic skill synthesis.
- **Developer Dark Cockpit WebUI**: Embedded zero-dependency Axum HTTP server, vanilla ES6 modular SPA, SVG Mermaid rendering, token economics calculator.

### 3.6 Chapter 6: Ecosystem Integration (`workflows/ecosystem.md`)
- **One-Click Configurator (`k0maru install`)**:
  - Claude Code (`~/.claude.json`)
  - Cursor (`~/.cursor/mcp.json`)
  - Gemini CLI / Antigravity (`~/.gemini/antigravity-cli/mcp_config.json`)
  - Windsurf (`~/.codeium/windsurf/mcp_config.json`)
  - Cline / Roo Code (`settings/cline_mcp_settings.json`)
  - Nous Research Hermes Agent (`~/.hermes/skills/`)
  - OpenClaw Agent
- **Manual MCP Configuration**: Exact JSON configuration snippets for custom agents.
- **AGENTS.md Best Practices**: Writing effective system prompts instructing AI agents how and when to invoke K0maru FastMCP tools without magic word triggers.

### 3.7 Chapter 7: Industrial Production Scenarios (`workflows/scenarios.md`)
- **Scenario A: Enterprise Payment Gateway LLM-Wiki Maintenance**:
  - Maintaining distributed lock invariants (`SET NX EX 30`).
  - Pre-flight checks preventing double-charge regressions.
- **Scenario B: Cross-Session Debugging & Self-Healing Loop**:
  - Model encounters fatal compile error, recalls previous postmortem, applies verified workaround, flushes update.
- **Scenario C: Dynamic Skill Distillation**:
  - Extracting repeatable deployment and debugging playbooks directly into Hermes dynamic skill directories.

### 3.8 Chapter 8: CLI & FastMCP Protocol Reference (`reference/cli.md` & `reference/mcp.md`)
- **CLI Subcommand Reference**: Exhaustive table of all subcommands (`loadout`, `search`, `offload`, `inspect`, `sync`, `flush`, `distill`, `install`, `doctor`, `ui`, `mcp`) with arguments, environment variables, exit codes, and JSON outputs.
- **FastMCP Tool Protocol**: Formal input/output JSON schemas for `get_project_loadout`, `recall_memory`, `offload_context`, `inspect_log_node`, `flush_session`, `distill_session_skill`.

### 3.9 Chapter 9: Empirical Benchmarks & Token Economics (`benchmarks/a100-evaluation.md`)
- **Hardware Setup**: NVIDIA A100-SXM4-80GB GPU, local Ollama / vLLM runtime.
- **Tested Models**: Qwen2.5-Coder-32B, DeepSeek-R1-32B, DeepSeek-Coder-V2 (16B MoE), Qwen3.8-27B.
- **Key Empirical Results**:
  - Qwen2.5-Coder-32B: Pass@1 increased from 60% to 100% (+40%), 5.90x latency reduction (17.23s -> 2.92s).
  - DeepSeek-R1-32B: Reflection latency reduced by -48.2% (27.14s -> 14.07s), 20.2% token savings.
  - DeepSeek-Coder-V2: Sub-second 1.05s turnaround (11.86x faster).
  - Qwen3.8-27B: 23.2% Token Reduction Ratio (TRR), halved Rust turnaround.
- **Publication-Ready Figures**: Embedded graphs (`latency_comparison.png`, `pass_rate_comparison.png`, `token_reduction.png`).

---

## 4. VitePress Theme & Aesthetic Specification

- **Base Theme**: Default VitePress theme with custom Dark Cockpit CSS overrides.
- **Color Palette**:
  - Background (Dark): `#0F172A` (Slate 900)
  - Surface / Card: `#1B2336` (Slate 850)
  - Border: `#334155` (Slate 700)
  - Primary Accent: `#22C55E` (Emerald 500)
  - Primary Hover: `#16A34A` (Emerald 600)
  - Text Primary: `#F8FAFC` (Slate 50)
  - Text Muted: `#94A3B8` (Slate 400)
- **Typography**:
  - Heading & Body: `Inter`, `-apple-system`, `BlinkMacSystemFont`, `Segoe UI`, `sans-serif`
  - Code & Monospace: `JetBrains Mono`, `Fira Code`, `Consolas`, monospace
- **Search Provider**: Local Minisearch (`provider: 'local'`) configured with Chinese and English tokenizer support.
- **I18n Switcher**: Dual root configs (`/zh/` and `/en/`), with root `/` redirecting based on browser locale or defaulting to `/zh/`.

---

## 5. Verification & Acceptance Criteria

1. **Local Development Server**: `npm run docs:dev` starts cleanly without warnings on `http://localhost:5173`.
2. **Production Static Build**: `npm run docs:build` generates clean static HTML/CSS/JS in `docs/.vitepress/dist/` with 0 broken links.
3. **Bilingual Completeness**: 100% symmetrical chapter parity between `docs/zh/` and `docs/en/`.
4. **Search Verification**: In-browser search modal indexes both English and Chinese content instantly.
5. **No Regressions to Core Rust Binary**: `cargo test` (195 tests) continues to pass 100% green; `cargo clippy` and `cargo fmt` remain clean.
6. **Deployment Automation**: `.github/workflows/deploy-docs.yml` validates build and deploys to GitHub Pages upon push.
