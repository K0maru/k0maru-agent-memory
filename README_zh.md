[English](README.md) | [简体中文](README_zh.md)

> 🤖 **一句话让 AI 自动接入（Claude Code / Cursor / Windsurf / Cline / `omp`）**：在任意代码仓库中打开你的 AI 编码工具，直接发送这一句话即可全自动完成安装与闭环挂载：
> ```text
> Read https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/llms.txt (or AGENT_GUIDE.md) and set up k0maru-agent-memory for this repository.
> ```
> *（智能体全自主运行手册：**[`AGENT_GUIDE.md`](AGENT_GUIDE.md)** · 机器可读规格：**[`llms.txt`](llms.txt)**）*

# K0maru-Agent-Memory

> **把你的 Markdown 知识库 / Obsidian 第二大脑变成 AI 编码智能体的“可执行记忆”与 `<10ms` 架构防退化护栏。**  
> *不同于只做“聊天行车记录仪”的被动会话记忆工具，K0maru 将你的架构决策（ADR）与工程笔记转化为全闭环治理引擎：**`<300-token` Git Diff 极速读档 ➔ `<10ms` Pre-Commit 静态守卫拦截 ➔ 零 LLM 确定性新旧决策取代（`--supersedes`）**。*
[![Language: Rust 2021](https://img.shields.io/badge/Language-Rust_2021-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version: 1.0.0](https://img.shields.io/badge/Version-1.0.0-blue.svg)]()
[![Tests: 200+ passed](https://img.shields.io/badge/Tests-200%2B_passed-brightgreen.svg)]()
[![Binary Size: 3.6MB (core) / 34MB (bundled ONNX)](https://img.shields.io/badge/Binary_Size-3.6MB_(core)_%2F_34MB_(bundled_ONNX)-success.svg)]()
[![Cold Start: 3.4ms](https://img.shields.io/badge/Cold_Start-3.4ms-purple.svg)]()
[![Zero-Daemon](https://img.shields.io/badge/Daemon-Zero_Daemon-informational.svg)]()

> 💡 **双二进制别名支持**：原生编译生成 `k0maru` 与 `k0maru-memory` 两个独立二进制，与智能体协同编排生态保持命名对称。

---

## 🌟 核心优势与方案对比 (Why K0maru?)

在现代开发流中，AI 编码智能体（Claude Code、Cursor、Windsurf、Gemini 等）常常深陷两大痛点：**跨会话失忆（Amnesia）** 与 **终端长日志撑爆上下文（Terminal Log Bloat）**。

市面上许多记忆框架往往引入庞大的 Docker 容器集群（PostgreSQL、pgvector、Redis）或常驻后台守护进程，带来高昂的常驻内存开销、网络端口冲突与复杂的部署负担。

**K0maru 专为单机开发机打造，以极简机械复杂度交付工业级性能**：

1. **⚡ 零守护进程与极致冷启动（Zero-Daemon & 3.4ms Cold Start）**：
   - 纯 Rust 静态编译，单二进制体积仅 **3.6MB (core) / 19MB (macOS arm64) / 34MB (Linux x86_64)**（使用 `--no-default-features` 编译的纯 Rust FTS5+Guard 核心二进制仅 `3.6 MB`，默认捆绑 `fastembed` 与 ONNX Runtime 的发行版二进制在 macOS arm64 为 `19 MB` / Linux x86_64 为 `34 MB`，压缩下载包 15.5 MB；ONNX 模型权重在首次 `k0maru sync --vector` 时自动缓存于 `~/.k0maru/models/`）；
   - 冷启动仅需 **3.4 ms**，常驻内存仅 ~11 MB，**不占用任何常驻后台网络端口**；
   - 通过 Unix 管道与标准 stdio 协议即调即走，命令执行完毕瞬间将内存归还系统。
2. **📝 自备纯文本 Markdown（拓扑无关的 LLM-Wiki 适配）**：
   - 用户已有的本地 Markdown 笔记即为唯一真理源泉。架构完全与拓扑解耦，原生兼容**任意 LLM-Wiki 组织形态**：Karpathy 式扁平单目录 LLM-Wiki、工程级 `docs/adr/` 或 `decisions/` 维基、PARA 体系（`projects/`、`cards/`、`logs/`）以及 Johnny.Decimal（`10_Projects/`、`20_Cards/`）；
   - 杜绝封闭专有二进制格式，零迁移壁垒，不破坏用户原有的知识库目录树。
3. **🪓 随时可抛弃的瞬态缓存（Disposable Cache）**：
   - 本地仅维护一个单文件 SQLite（`cache.sqlite`），集成 FTS5 全文倒排索引与 `sqlite-vec` 向量虚表；
   - 支持多模型动态维度迁移，数据库随时可直接删除（`rm cache.sqlite`），扫描器能在 **74 ms 内全量自愈重建**。
4. **📉 符号化长日志卸载与上下文治理（99.4% Token 压缩率 & 真实 A100 实测提升）**：
   - 标准 Unix 管道长日志符号化卸载（`| k0maru offload`），超长堆栈转储为外部切片与 Mermaid 状态机；
   - 提供 4 级会话剪枝协议（`k0maru gc`），确保长轨迹会话始终驻留在模型认知黄金区（15%~60% 窗口）。
5. **🛡️ 活体架构守卫与零记忆税演进（Living ADR Guard & Active Supersession）**：
   - 将知识库中原本沉睡的架构决策记录（ADR）转化为活跃可执行的工程守卫哨兵（`k0maru guard check --diff` / `guard_check`），在代码提交前 `<10ms` 自动阻断违规代码；
   - **零 LLM 确定性决策取代（`k0maru flush --supersedes`）**：当架构演进推翻旧方案时，一行 `--supersedes ADR-010` 自动将旧笔记原地标记为 `status: superseded`（原生兼容 Google OKF 生命周期规范），**一票否决旧笔记的历史反向双链图谱加成（破除 Belief Entrenchment 双链霸榜与过期记忆投毒）**，将其移出 `<300-token` 活跃读档，并**自动退役旧笔记的僵尸护栏、热更新激活新笔记的反退化拦截规则**。
### 方案对比矩阵 (Architectural Comparison)

| 评估维度 | 全量手动粘贴 (Vanilla Context) | 容器化微服务方案 (如 Letta / TencentDB) | 专有后台守护工具 (如 agentmemory) | **K0maru-Agent-Memory (本项目)** |
| :--- | :--- | :--- | :--- | :--- |
| **存储媒介** | 散落文本或无持久化 | 外部数据库 (PostgreSQL, pgvector, Redis) | 专有格式或隐藏目录数据库 | **本地既有 Markdown 笔记 (纯文本)** |
| **部署与运行模型** | 无依赖 | Docker Compose 多容器集群 | 需常驻后台守护进程 (`iii-engine` 等) | **单静态二进制，按需运行 (零守护进程)** |
| **网络端口占用** | 0 | 1 ~ 4 个网络端口 | 1 ~ 4 个网络端口 | **0 端口** (纯 Unix 管道与 stdio 通信) |
| **冷启动延迟** | 即时 | 2,500 ms ~ 3,500 ms (容器及环境启动) | 1,000 ms ~ 2,000 ms | **3.4 ms** (Rust 原生执行) |
| **常驻内存开销** | 0 | ~850 MB - 2,000 MB | ~150 MB - 300 MB | **~11.7 MB** (任务完成后完全归还操作系统) |
| **长日志治理** | 截断由人肉或大模型被动截取 | 需经由网络 API 传输处理 | 依赖内部处理逻辑 | **标准 Unix 管道过滤器 (`\| k0maru offload`)** |
| **架构不变量守卫** | 人工 Code Review 易遗漏 | 外部 SonarQube / 重型 CI 流程 | 无 | **Living ADR Guard (<10ms 内存规则引擎，~15–55ms CLI 含 git diff)** |
| **记忆衰减与新旧取代** | 过期文档持续投毒上下文 | 依赖高成本 LLM 抽图谱 | 仅做被动搜索降权 | **零 LLM `--supersedes` + 破除旧双链霸榜 + 护栏规则热替换** |
| **索引自愈机制** | 无 | 强依赖外部数据库快照与备份 | 依赖本地私有数据库完整性 | **单文件 `cache.sqlite` 随时可删，74ms 快速全量重建** |

### 🛡️ 活体架构守卫与新旧决策取代 A/B 实测看板 (Adversarial Supersession Benchmark)

跨 **本地多智能体 (`omp` + `DeepSeek V4.1 Flash` on Apple Silicon M3 Max)**、**Google Colab NVIDIA L4 + Qwen2.5-Coder-7B + Pi CLI** 与 **可复现基准测试 (`python3 benchmarks/run_supersession_guard_ab.py`)** 在 `payment-settlement` 支付结算系统上完成真实端到端对照实验。测试在分布式锁架构演进中（从旧方案 `ADR-010` 的无作用域裸锁 `redis_setnx_unscoped` 升级为 `ADR-042` 的带栅栏租约锁 `acquire_fenced_lease`），无治理知识库与 K0maru 全闭环治理引擎的表现：

| 评估阶段与核心指标 | 对照组 (无 `--supersedes` 的普通 Markdown Wiki) | 实验组 (`k0maru flush --supersedes` + `guard check --diff`) | 工程治理突破与实测收益 |
| :--- | :--- | :--- | :--- |
| **检索首位命中 (`recall_memory`)** | **错误召回旧方案 `ADR-010` #1** (`redis_setnx_unscoped`)<br>*历史反向双链权重导致“信念固化 (Belief Entrenchment)”严重投毒* | **精准召回新方案 `ADR-042` #1** (`acquire_fenced_lease`)<br>*旧卡片自动剥夺图谱加成并标记 `superseded`，`<300-token` Loadout 经 `superseded_by` 自动跳转* | **破除过期记忆税**：无需人工搬砖清洗 Wiki，新决策零成本热替换 |
| **动态读档背包预算 (`<300-token`)** | 上下文超标膨胀（`>650 tokens`，混入过期废弃规则） | **严格收敛至 245 tokens**（通过 `LoadoutBuilder` 自动注入 `[Guard: forbid ...]` 活跃不变量） | **严格受控认知黄金区**，防止无用历史干扰推理 |
| **首轮代码实现 (Turn-1 Pass@1)** | 生成带有高危漏洞的裸锁 `redis_setnx_unscoped`（被旧决策投毒） | **100% 首轮一次性通过 (Pass@1)**<br>*直接生成 `acquire_fenced_lease`，0 次守卫违规* | **前置规约传递**：透传新 ADR 约束，在编写前杜绝技术退化 |
| **提交前守卫拦截 (`guard check`)** | **0 次拦截 (直接漏放)**<br>*危险代码 `redis_setnx_unscoped` 成功提交引入资损隐患* | **<35ms 端到端 100% 物理拦截**（核心探测仅 3.82ms，触发 `[DENY] ban-legacy-setnx`）<br>*若暂存不合规裸锁，毫秒级阻断提交* | **100% 防退化物理阻断**：杜绝架构退化与资损缺陷进入 Git 历史 |
| **智能体单轮自愈闭环** | 失败（静默引入线上资损风险） | **1 轮内 100% 修复通过**<br>*在 Local `omp` + DeepSeek V4.1 与 CLI 自动化测试中均实现 1-turn 快速自愈* | 错误信息直出修复引导，驱动小模型瞬间写出合规租约代码 |
> 🔬 **可复现基准测试套件与原始实测数据**：  
> - 端到端可执行测试脚本：[`benchmarks/run_supersession_guard_ab.py`](benchmarks/run_supersession_guard_ab.py)  
> - 完整机器可读实测指标 JSON：[`benchmarks/results/supersession_guard_ab_results.json`](benchmarks/results/supersession_guard_ab_results.json)

#### 智能体生态真实闭环适配验证 (Hermes Agent & OpenClaw on Real LLM-Wiki)

在 Colab 硬件上搭建真实企业级支付网关 LLM-Wiki（含 P0 服务架构、L3 幂等规则卡片与历史资损复盘），驱动 **Llama-3.1-8B-Instruct (A100)** 与 **Qwen2.5-Coder-7B (Colab L4)** 自主运行 Hermes / OpenClaw 多轮函数调用循环，四大核心不变量全部达成（`Llama-3.1-8B-Instruct: 100% (4/4 invariants)`，`Qwen2.5-Coder-7B (Colab L4): 100% Pass@1`）：

| 核心不变量检验项 | 期望标准与流程 | 实际执行验证结果 | 状态 |
| :--- | :--- | :--- | :---: |
| **1. 编码前预检 (Pre-flight Inspection)** | 编码前主动调用 `loadout` 与 `recall` 获取架构不变量与事故教训 | 自主触发 `get_project_loadout` 与 2 次精准 `recall_memory` | **PASS ✓** |
| **2. 架构不变量遵从 (Invariant Compliance)** | 代码中必须包含分布式互斥锁与状态机阻断，杜绝资损隐患 | 生成包含 Redis 锁 `SET NX EX 30` 与 `STATUS_PENDING` 检查的 Go 源码 | **PASS ✓** |
| **3. 会话结晶与自愈 (Self-Healing Flush)** | 交付后通过 `flush_session` 持久化 ADR 决策并编织双向 WikiLinks | 自动规范生成至 `20_Cards/adr-pay-012-...md` (织入 Frontmatter) | **PASS ✓** |
| **4. 即时唤醒验证 (Instant Recall)** | 新结晶笔记在无人工索引重建干预下必须可被瞬间唤醒 | 检索耗时 **0.88 ms**，得分 12.05 首位命中刚落盘的 ADR 卡片 | **PASS ✓** |

> 📖 **查看完整案例逐一拆解、Hermes / OpenClaw 多轮 Trace 与评测数据**：  
> 请参阅完整的 [评估报告第四部分：智能体生态真实闭环适配实测](docs/COMPARISON_REPORT_zh.md#4-智能体生态真实闭环适配实测nous-research-hermes-agent-与-openclaw-协同)。
---

## 🚀 安装与快速上手 (Quick Start & Installation)

详细步骤与多智能体进阶指南见 [QUICKSTART.md](QUICKSTART.md) 与 [AGENT_GUIDE.md](AGENT_GUIDE.md)。

### 🤖 方案 A（零人工干预）：给你的 AI 编码工具发一句话

无论你在 **Claude Code、Cursor、Windsurf、Cline 还是 `omp`** 中打开一个全新的代码仓库，只需要给 AI 发一句话：

```text
Read https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/llms.txt (or AGENT_GUIDE.md) and set up k0maru-agent-memory for this repository.
```

AI Agent 会自动完成单二进制安装、生成 `.k0maru/wiki/`、挂载项目级 `.mcp.json` + `.claude/settings.local.json`（`SessionStart` 自动读档 + `Stop` 交卷前自检自愈钩子）以及 `.git/hooks/{pre-commit,post-commit}` 的全部配置，进入 **100% Zero-Human-in-the-Loop** 自治闭环。

### ⚡ 方案 B（命令行）：30 秒零知识库极速冷启动（无需提前安装 Obsidian！）
只有一个 Git 仓库？完全不需要提前准备 Obsidian 或手动整理笔记。在你的项目根目录下运行两条命令：

```bash
# 1. 一键安装单二进制（零常驻后台进程）
curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash

# 2. 秒级初始化本地知识库并自动挂载所有智能体与 Git 钩子
k0maru init --install
```

**`k0maru init --install`（别名 `k0maru bootstrap`）在 1 秒内自动完成的事项：**
- 🔍 **智能扫描代码库**：自动解析项目 `README.md`、包管理清单（`Cargo.toml`、`package.json`、`pyproject.toml`、`go.mod`）、智能体宪法（`AGENTS.md`、`CLAUDE.md`）与近期 Git 提交历史；
- 📐 **脚手架生成知识库**：在项目根目录构建 `.k0maru/wiki/` 目录：
  - `10_Projects/<project_slug>.md`：自动提取项目核心架构、模块边界与工程指南；
  - `20_Cards/adr-001-repository-guardrails.md`：首张活体 ADR 守卫卡片，内置可执行正则与边界规则（自动阻断敏感密钥泄漏、调试 dump 与违规提交）；
  - `01_AI_Logs/<YYYY-MM-DD>-initial-bootstrap.md`：根据 Git 提交历史自动生成初始交接日志；
- 🔗 **自动挂载 FastMCP 与全生命周期钩子**：生成项目级 `.mcp.json` 与 `.claude/settings.local.json`（`SessionStart` `<300-token` Diff 自动读档 + `Stop` 交卷前 `exit 2` 自愈守卫），并安装 `.git/hooks/pre-commit`（`<10ms` 代码提交前架构审查）与 `.git/hooks/post-commit`（提交后自动同步索引并记录里程碑）；
- ⚡ **秒级构建瞬态缓存**：编译生成 `cache.sqlite` 倒排与向量索引，开箱即用支持 `k0maru loadout` 与 `k0maru guard check --diff`！

---

### 方案 C：挂载已有 Markdown / Obsidian 知识库 (Power Users)

如果您已有长期维护的 Obsidian 库、PARA 笔记或工程 Wiki：

```bash
# 一键自动发现并配置所有支持的智能体客户端，指向您的 Markdown 知识库并挂载钩子
k0maru install --vault ~/Documents/MyVault --hooks

# 或先通过 --dry-run 预览将要进行的配置改动，不实际写入磁盘
k0maru install --vault ~/Documents/MyVault --dry-run
```

若您更倾向于手动配置，只需在 IDE 的 MCP 配置文件中加入：
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

或通过 Claude Code 官方插件市场一键安装：
```text
/plugin marketplace add K0maru/k0maru-agent-memory
/plugin install k0maru-memory
```

---

### 二进制安装方式

- **官方一键安装脚本（macOS & Linux - 推荐）**：
  ```bash
  curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash
  ```
- **Homebrew（macOS & Linux）**：
  ```bash
  brew install K0maru/tap/k0maru
  ```
- **源码编译安装 (Cargo)**：
  ```bash
  # 包含本地 fastembed + ONNX 向量模型的完整发行版二进制 (19 MB macOS arm64 / 34 MB Linux x86_64)
  cargo install --git https://github.com/K0maru/k0maru-agent-memory

  # 纯 Rust FTS5 BM25 + Living ADR Guard 轻量核心二进制 (3.6 MB)
  cargo install --git https://github.com/K0maru/k0maru-agent-memory --no-default-features
  ```
  *（同时构建生成 `k0maru` 与 `k0maru-memory` 二进制别名。`--no-default-features` 纯 Rust FTS5+Guard 核心二进制仅 `3.6 MB`，默认开启 `fastembed` + ONNX Runtime 的完整二进制为 `19 MB` (macOS arm64) / `34 MB` (Linux x86_64)，压缩下载 15.5 MB；模型权重在首次 `k0maru sync --vector` 时缓存于 `~/.k0maru/models/`）*。
---

### 3. 一键环境健康诊断（`k0maru doctor`）

安装与配置完成后，执行诊断命令全面体检系统环境、知识库规范、SQLite 索引与客户端挂载状态：

```bash
k0maru doctor
```

控制台将以彩色清晰呈现 4 大核心子系统（`[Binary & System]`, `[Vault & Hierarchy]`, `[Storage & Search Engine]`, `[Agent MCP Ecosystem]`）的诊断项，发现任何缺失都会提供精准的修复命令。

---

## 🛠️ 常见使用场景与核心操作方法 (Common Use Cases & How-Tos)

### 场景 1：开辟新任务会话 —— 项目级极速读档装配（`k0maru loadout` & `get_project_loadout`）

当你在 Claude Code 或 Cursor 中新起一个对话时，最忌讳把几十篇笔记一股脑全喂给模型导致注意力发散。使用 `loadout` 仅提取目标项目的愿景、关联核心架构约束（L3）与近期研发日志（L2），严格控制在 **< 300 Token**：

- **4 级穿透推导漏斗 (4-Tier Scoped Resolver)**：
  - **Tier 1**：精准规范化名称、别名、标签及反向链接派生别名匹配；
  - **Tier 2**：SQL 层面下推的 Scoped FTS5 匹配（杜绝粗暴全局 Top 5 截断产生的信息丢失）；
  - **Tier 3**：多维语义向量回退（若存在本地向量嵌入）；
  - **Tier 4**：当前工作区目录及上下文环境自适应匹配。
- **Git Diff 变更感知动态激活（`--diff`）**：自动嗅探 Git 工作区修改文件，动态激活相关模块的 ADR 与架构铁律。

```bash
# 提取指定项目的装配背包，并直接复制到系统剪贴板 (macOS / Linux / Windows)
k0maru loadout my-project --vault ~/wiki --copy

# 变更感知装配：自动分析未提交 Diff 动态注入关联模块规则
k0maru loadout my-project --vault ~/wiki --diff --copy
```

**FastMCP 智能体自主调用契约**：`get_project_loadout`
```json
{
  "project_name": "k0maru-agent-memory",
  "diff": true,
  "branch": "main"
}
```

---

### 场景 2：跑测试/编译报错 —— 终端长日志符号化卸载（`| k0maru offload`）

运行 `cargo test`、`npm test` 或编译工具时，数千行的巨幅回溯极易将模型的上下文窗口挤爆。在命令尾部追加管道过滤：

```bash
# 超过 50 行的超长日志自动截断，控制台仅输出带 node_id 的 Mermaid 流程状态图
cargo test 2>&1 | k0maru offload

# 或在排障过程中，凭 node_id 定向调取局部的真实报错堆栈
k0maru inspect node_54697ed3
```

- **99.44% 的 Token 压缩率**：超长编译转储落盘至 `.scratch/refs/` 或 `<vault>/.k0maru/refs/`，绝不污染会话窗口；
- **Mermaid 状态机图直击根因**：状态图直接指出故障流转节点（如 `Iteration1 --> Failure: E0382 borrow after move`），引导大模型一眼看穿根因；
- **智能体原生 FastMCP 对应工具**：
  - `offload_context`：AI 智能体在执行终端命令产生长日志时主动卸载；
  - `inspect_log_node`：按 `node_id` 局部调取报错堆栈；
  - `gc_session_context`：对长达 50+ 轮的长对话轨迹执行 4 级剪枝，保证关键推理轮次完整（Hot Window），达成 **TRR >= 85%** 的强效压缩。

---

### 场景 3：私有知识库语义召回 —— 智能体自然语言意图驱动翻阅（`recall_memory` & `k0maru search`）

无需记忆生硬指令，挂载 `k0maru-memory` MCP 后，大语言模型会根据自然语义**自主推断意图并调用底层工具**：

```bash
# 混合检索（BM25 + 本地向量 + RRF k=60 + 双链图谱加权）
k0maru search "并发死锁排查最佳实践" --vault ~/wiki --mode hybrid --limit 5

# 可插拔向量模型检索（如切换至多语言或中文专用模型）
k0maru search "向量相似度" --vault ~/wiki --embed-model bge-small-zh-v1.5
```

- **默认极简单行线协议编码**：默认返回高密度紧凑单行格式，最大限度节约上下文窗口 Token（详见文末 [进阶：多路复用器与 Sidecar 生态](#-进阶多路复用器与-sidecar-生态-tern-omp-与-k-dsl-线协议)）；仅在显式传入 `"full": true` 参数时才输出完整多行 Markdown 原文。
- **可插拔多模型向量后端**：原生内置 `all-minilm-l6-v2`（384d，默认）、`bge-small-zh-v1.5`（512d）、`bge-base-en-v1.5`（768d）、`paraphrase-multilingual-minilm-l12-v2`（384d）及离线测试专用的 `mock` 引擎；
- **动态维度自愈迁移**：切换模型或维度时自动无损重建 `sqlite-vec` 虚表，免去手工删库建表烦恼。

---

### 场景 4：决策与经验沉淀 —— 研发心得自适应规约结晶（`k0maru flush` & `distill`）

开发完成或解决复杂 Bug 后，将经验沉淀回知识库：

```bash
# 结晶新架构决策（ADR-042），并确定性取代旧决策（ADR-010）
k0maru flush \
  --title "ADR-042 原生正交外挂并发模型" \
  --category decision \
  --supersedes "ADR-010 Legacy Bus" \
  --summary "以零守护进程 stdio 外挂全面取代旧版内部消息总线" \
  --content $'## 核心工程铁律\n- [no-legacy-bus] (deny) [src/**/*] forbidden: "legacy_bus_send" | remediation: "请改用 v2 sidecar RPC"' \
  --tags "tokio,rust,architecture" \
  --vault ~/wiki
# 将错误排障轨迹直接提炼为可复用的故障排查技能卡片
k0maru distill \
  --trace-file /tmp/error.log \
  --title "解决 SQLite Vec 动态链接缺失" \
  --vault ~/wiki
```

- **规约嗅探（Convention Sniffing）**：自动嗅探知识库是否有 `AGENTS.md`、`templates/` 或现有命名规范（如 `YYYY-MM-DD-slug.md`），自适应对齐格式；
- **防碰撞（Anti-Collision）**：同名笔记智能处理自增版本（`-v2.md`），绝不粗暴覆盖既有文件；
- **Hermes 技能导出**：支持 `--target hermes` 直接导出为兼容 Nous Research Hermes Agent 的动态技能；
- **确定性新旧取代（`--supersedes`）**：自动原地改写旧笔记的 Frontmatter（`status: superseded`、`superseded_by: "[[...]]"`），在混合检索中剥夺旧笔记的历史双链加权并注入 `[SUPERSEDED by ...]` 醒目警示，将其从 `<300-token` 活跃读档中剔除，并同步热替换 `k0maru guard` 架构拦截规则；
- **FastMCP 智能体对应工具**：`flush_session` 与 `distill_session_skill`。

---

### 场景 5：提交前哨兵 —— 活体架构守卫（`k0maru guard` & `guard_check`）

技术规范和 ADR 不应成为写完就忘的“死文档”。K0maru 的 **Living ADR Guard** 将知识库规则变为随时可触发的代码审查哨兵：

```bash
# 在毫秒级对 Git 工作区未提交改动执行架构不变量全检 (<10ms)
k0maru guard check --diff

# 一键安装 Pre-commit 钩子，代码违规时自动阻断 git commit
k0maru guard install
```

#### 如何声明架构不变量 (Invariants)

在知识库的 ADR 笔记（`20_Cards/` 或 `10_Projects/`）的 Frontmatter 中添加 `invariants` 字段即可生效：

```yaml
---
title: ADR-042 分布式锁与状态机设计规范
category: decision
tags: [architecture, redis, idempotency]
invariants:
  - id: "enforce-distributed-mutex"
    condition: "must_contain"
    pattern: "redis_lock\\.acquire|SET\\s+NX\\s+EX"
    paths: ["src/payment/**/*.rs", "services/order/**/*.go"]
    severity: "deny"
    fix_hint: "核心资金流链路在变更状态前必须显式获取分布式 Redis 锁。"
  - id: "ban-raw-sql-in-handlers"
    condition: "forbidden"
    pattern: "SELECT .* FROM raw_|execute_unprepared"
    paths: ["src/api/**/*.rs"]
    severity: "deny"
    fix_hint: "禁止在 API 层拼装裸 SQL，统一通过存储层预编译语句调用。"
---
```

亦可在仓库根目录的 `.k0maru/rules`、`guard.toml` 或 `AGENTS.md` 中集中配置。

- **<10ms 极致低延迟**：Rust 原生瞬态多行正则与路径 Glob 匹配，毫秒级快速给出结论；
- **清晰退出码**：`0` 检查通过，`1` 存在阻断性违规（`deny`），`2` 执行环境异常；
- **FastMCP 智能体自主调用**：`guard_check` 供智能体在完成功能实现（`/implement`）或提交前自主跑检自愈。

---

### 场景 6：本地知识库洞察 —— 开发者嵌入式调试台（`k0maru ui`）

无需配置外部 Web 服务，一行命令在本地拉起高性能可视化开发工作台：

```bash
k0maru ui --vault ~/wiki --open
```

![K0maru 开发者控制台 - 多路检索调试与图谱加权](docs/images/ui-search.png)

- **🌐 中英双语一键切换**：顶栏支持中英文无缝切换，自动识别系统首选项；
- **🔍 检索解释性调试台 (Search Debugger)**：直观对比 BM25 排名、向量余弦得分与 Emerald `+0.004 Graph Boost` 拓扑加权；
- **🪵 日志切片与状态机透视 (Log & Trace Inspector)**：离线 SVG 渲染 Mermaid 故障状态机，带行号错误高亮与局部复制；
- **📊 缓存健康与 Token 计分板 (Token Scoreboard)**：实时监控向量覆盖率、缓存物理体积与累计节约 Token 经济账；
- **退出即释放**：`Ctrl+C` 退出瞬间释放所有端口与内存，绝不残留任何后台进程。详细教程见 [docs/UI_TUTORIAL.md](docs/UI_TUTORIAL.md)。

---

### 场景 7：知识库文件变更 —— 毫秒级增量感知与快速同步（`k0maru sync`）

```bash
# 毫秒级增量扫描 Markdown 结构与 FTS 索引 (无变更仅耗时 11ms)
k0maru sync --vault ~/wiki

# 批量联动增量计算向量索引 (自动跳过未修改文档)
k0maru sync --vault ~/wiki --vector

# 指定自定义向量模型增量同步
k0maru sync --vault ~/wiki --vector --embed-model bge-small-zh-v1.5
```

---

## 🤖 智能体生态协同与 FastMCP 协议接口总览

挂载 FastMCP 后，K0maru 为通用主流 IDE 与 CLI 智能体客户端（Claude Code、Cursor、Codex、Windsurf、Gemini CLI、Cline、Hermes、OpenClaw）提供全套长期记忆、架构守卫与上下文治理能力：

| FastMCP 工具名称 | 人类自然语言触发意图示例 | 核心价值与底层动作 |
| :--- | :--- | :--- |
| `get_project_loadout` | *“开始新任务”*、*“读一下当前项目架构背景”* | 4 级漏斗精简装配，输出愿景、L3 铁律与近期日志，严格控制在 **< 300 Token**。支持 `--diff` 变更感知。 |
| `recall_memory` | *“查一下某技术方案”*、*“我们以前怎么配 X 的”* | 混合 RRF 检索（BM25 + 本地向量 + 双链拓扑加权），亚毫秒级精准定位私有知识。 |
| `offload_context` | 智能体跑测试/编译产生超 30 行大段日志输出时 | 将终端原始文本外置压缩，自动提炼生成带节点 ID 的 Mermaid 状态机图。 |
| `inspect_log_node` | *“查看 node_xxxx 的具体失败堆栈”* | 根据 Node ID 精准调取保存在本地磁盘上的完整切片。 |
| `flush_session` | *“记录这个架构决策”*、*“把刚才排障结论存起来”* | 自动按照知识库规约格式化生成包含 Frontmatter 与 WikiLinks 的 ADR 笔记。 |
| `distill_session_skill`| *“把这次排错过程提炼为可复用的技能手册”* | 解析调用栈与排障上下文，提炼生成标准化故障排除 Skill（支持 Obsidian 与 Hermes Agent）。 |
| `gc_session_context` | 多轮对话上下文逼近上限时 | 执行 4 级剪枝协议，折叠过往冗长工具输出，维持认知黄金区，达成 **TRR >= 85%**。 |
| `guard_check` | 提交 Git 前、功能代码实现交付后审查 | 毫秒级审计当前 Diff 与修改，强制阻断违反 ADR 架构不变量的代码合入。 |

> 💡 **进阶技巧：如何让 AI 100% 主动遵守架构规则？**  
> 在项目根目录的 `AGENTS.md`（或全局 System Prompt）中添加一行：  
> `> 在设计系统架构、排查疑难 Bug 或编写核心业务逻辑前，优先调用 k0maru-memory 翻阅本地知识库，严格对齐已沉淀的技术规范与历史决策。完成代码后必须调用 guard_check 自检。`  
> 哪怕您只说一句 *“写个支付回调”*，AI 也会在后台先行查阅防重放规约并在交付前跑检，杜绝凭空臆造（No Vibe Coding）。

### 🔄 智能体全闭环无人工干预生命周期 (No-Human-in-the-Loop Lifecycle)

K0maru 专为无人值守的自主智能体工程闭环设计，覆盖完整的开发生命周期：

1. **开局起步自动读档（`SessionStart`）**：Claude Code 或 IDE 钩子自动触发 `k0maru loadout "<project>" --diff`，装配小于 300 Token 的受控认知背包（项目使命、核心 ADR 守卫规则、近期提交日志）；
2. **交付前自主审查（Claude Code `hooks.Stop`）**：在智能体输出给人类前，自动调用 `k0maru guard check --diff --claude-stop-hook`。若检测到架构违规，返回 POSIX 退出码 2 阻断输出并由智能体进行 1-turn 自主自愈（代码重构或通过 `k0maru flush --supersedes` 取代旧架构）；
3. **代码提交前置拦截门禁（`.git/hooks/pre-commit`）**：`<10ms` 纯静态 Rust 正则与词法审计，强制拦截违规代码合入 Git 历史；
4. **提交后自动同步与里程碑结晶（`.git/hooks/post-commit`）**：提交后自动触发 `k0maru sync` 增量刷新索引（~11ms），并在包含 `feat:`、`fix:`、`refactor:`、`perf:`、`adr:` 的提交发生时自动向 `01_AI_Logs/` 结晶结构化里程碑卡片。

### 🎛️ 进阶：多路复用器与 Sidecar 生态 (Tern, omp 与 K-DSL 线协议)

面向特定终端多路复用器、编排框架与超高密度多智能体协作流：

1. **Tern 终端复用器原生插件**：
   基于 Luau 编写，提供交互式 Cockpit 驾驶舱看板、CLI 命令透镜与 command palette 调色板指令集成：
   ```bash
   tern plugin install github.com/K0maru/k0maru-agent-memory/tern-plugin
   ```

2. **Oh My Pi (`omp`) 与正交伴生模式 (`k0maru mcp --sidecar`)**：
   由于 `omp` 原生负责终端流式日志管理与会话压缩，K0maru 提供 `--sidecar` 伴生模式挂载 5 大正交领域记忆工具（`get_project_loadout`、`recall_memory`、`flush_session`、`distill_session_skill`、`guard_check`），主动剔除长日志卸载与 GC 工具，杜绝功能重叠：
   ```bash
   # omp 插件市场安装
   /marketplace add K0maru/k0maru-agent-memory
   /marketplace install k0maru-memory@k0maru-agent-memory
   ```

3. **极简紧凑 K-DSL 电报协议 (`!μ:`)**：
   为防多智能体主控上下文爆炸，`recall_memory` 默认返回单行高密度 K-DSL 记忆电报：
   `!μ:<tier>|<path>|score:<score>|links:<in>/<out> § <snippet>`（单条截断 ≤140 字符）。
   相较于全量 Markdown 文本，单条载荷立减 **73.16%**（164 字节 / 22 Token vs 611 字节 / 112 Token）。仅在显式传入 `"full": true` 参数时返回多行原文。

---

## 📊 Google Colab A100 真实硬件基准实测看板

在 Google Colab **NVIDIA A100-SXM4-80GB GPU** 专属算力节点上，对 5 款开源基座模型开展了 5 大工业级排障缺陷（Rust `E0382`、Python `InvalidStateError`、TypeScript 鉴权深层解构、Go 死锁、C 悬垂指针双重释放）与 4 大架构不变量的端到端硬核实测：

### 1. 五大模型全矩阵评测看板（5-Case 符号化卸载微基准实测，N=5）

| 参测基座模型 | 参数与架构规格 | Pass@1（对照组） | Pass@1（实验组） | 平均生成耗时（对照 ➔ 实验） | 响应提速比 | 输入 Token（对照 ➔ 实验） | Token 压缩率（%） |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Qwen2.5-32B-Instruct** *(主打看板)* | 32B Dense Instruct | 20.0% (1/5) | **40.0% (2/5)** | 16.70s ➔ **6.14s** | **2.72x** | 5,745 ➔ **1,076** | **81.3%** |
| **DeepSeek-Coder-V2-16B** | 16B MoE (2.4B 激活) | 80.0% (4/5) | **100.0% (5/5)** | 9.43s ➔ **1.36s** | **6.93x** | 6,298 ➔ **1,142** | **81.9%** |
| **Qwen2.5-Coder-32B** *(代码基准)* | 32B Dense Q4_K_M | 60.0% (3/5) | **100.0% (5/5)** | 24.68s ➔ **7.13s** | **3.46x** | 5,472 ➔ **1,119** | **79.6%** |
| **DeepSeek-R1-32B** | 32B 满血推理思维链 | 20.0% (1/5) | **40.0% (2/5)** | 12.01s ➔ **9.19s** | **1.31x** | 5,412 ➔ **989** | **81.7%** |
| **Google Gemma 2 27B** | 27B 开源基础模型 | 0.0% (0/5) | 0.0% (0/5) | 16.69s ➔ **6.94s** | **2.40x** | 6,356 ➔ **1,134** | **82.2%** |

*注：5 案例符号化长日志卸载微基准实测（N=5 工业缺陷场景，基于 Google Colab NVIDIA A100-80GB Ollama 本地推理）。详尽的单用例耗时对比与四大核心不变量遵从全检报告，请参阅 [docs/COMPARISON_REPORT_zh.md](docs/COMPARISON_REPORT_zh.md)。*

### 2. Phase 9 多智能体高并发与异构 Harness 极限压测（A100）

| 压测测试维度 | 并发负载与规模 | 实测核心指标 / 吞吐速率 | 测试状态 | 工程保证与安全性机制 |
| :--- | :--- | :--- | :---: | :--- |
| **12 智能体突发并发读取** | 12 线程，120 次并发检索 | **18.9 QPS**，均值 **614.29ms**（P50: 781.9ms） | **100% PASS** | 零 SQLite 读写忙等死锁（`SQLITE_BUSY = 0`） |
| **8 智能体并发写入与竞态** | 8 线程同时回写相同标题决策 | **12.2 writes/sec**，均值 **584.93ms** | **100% PASS** | 8/8 文件成功生成，`-v2.md` 到 `-v8.md` 零数据丢失 |
| **即时自愈唤醒召回** | 读写强一致性即时校验 | **290.99 ms** | **PASS ✓** | 免手动重建索引，新笔记落盘后毫秒级可搜 |
| **会话上下文垃圾回收 (GC)** | 50-turn 长轨迹会话剪枝 | **86.58% TRR**（16,830 ➔ 2,259 Tokens，折叠 45 处工具追踪） | **PASS ✓** | 严格维持在模型认知黄金区（15%~60% 窗口） |
| **Git-Diff 动态 Loadout** | 变更感知架构注入 | **245 Tokens** | **PASS ✓** | 严格受控于 <300 Token 安全背包预算内 |
| **三大异构智能体自主循环** | Hermes, OpenClaw, Pi | 单轮闭环：**1.04s ~ 8.49s** | **100% PASS** | 验证工具调用、FastMCP 桥接与流管道自治落盘 |

---

## 🏗️ 架构设计与底层机制 (Architecture & Mechanics)

### 四层解耦架构全景

![K0maru-Agent-Memory: 四层解耦架构全景图](docs/images/architecture_overview.png)

1. **接口层 (Interface Layer)**：CLI 交互（管道过滤器、`doctor`、`install`、`loadout`、`search`、`flush`、`distill`、`gc`、`guard`）+ 内嵌式可视化控制台（Axum + rust-embed）+ FastMCP Stdio 服务。
2. **调度与上下文治理层 (Orchestration & Governance Layer)**：Living ADR 守卫引擎 + 状态机日志离线解析器（Mermaid 符号化抽象与切片存储）+ 装配背包裁剪器（<300 Token Smart-Zone 控制）+ 自适应规约嗅探器。
3. **混合检索与图谱拓扑层 (Hybrid Search & Graph Layer)**：FTS5 BM25 词法倒排索引 + `sqlite-vec` ONNX 向量嵌入（支持 384d/512d/768d 多模型插拔）+ RRF 融合倒数排序（$k=60$）+ 1-hop WikiLinks 双链图谱拓扑加权（Graph Boost +0.004）。
4. **存储与持久化层 (Storage & Persistence Layer)**：用户本地纯文本 Markdown 知识库（唯一真理源）+ 瞬态可随时重建的 `cache.sqlite`（瞬态缓存与图拓扑加速）。

### 知识闭环生命周期

![记忆生命周期：LLM-Wiki 的双向闭环与经验结晶](docs/images/memory_lifecycle.png)

- **唤醒与读档（Loadout）** ➔ **运行与过滤（Offload）** ➔ **提交前守卫（Living ADR Guard）** ➔ **阶段交付自动沉淀（Session Flush）** ➔ **知识提炼与定期结晶（Consolidation）**

---

## 📖 CLI 完整指令速查表 (CLI Cheat Sheet)

支持 `k0maru` 与 `k0maru-memory` 两个二进制别名，功能完全一致。

| 命令 | 常用参数 | 说明 |
| :--- | :--- | :--- |
| `k0maru init` *(别名: `bootstrap`)* | `--repo <path>` | 待扫描的代码仓库根目录（默认当前工作区路径） |
| | `--vault <path>` | 目标知识库 Wiki 路径（默认 `<repo>/.k0maru/wiki`） |
| | `--install` | 初始化完成后自动挂载已安装智能体 FastMCP 并安装 Git/会话钩子 |
| | `--json` | 输出机器可读的 JSON 初始化摘要 |
| `k0maru install` | `--vault <path>` | 一键将 `k0maru-memory` 挂载至智能体客户端配置文件 |
| | `--hooks` | 一键安装 `.git/hooks/pre-commit` 架构守卫钩子与 Claude Code 会话自动读档钩子 |
| | `--target <client>` | 指定客户端 (`all`, `claude`, `cursor`, `codex`, `gemini`, `windsurf`, `cline`, `hermes`, `openclaw`, `omp`) |
| | `--dry-run` | 仅预览配置变更差异，不实际写入磁盘 |
| `k0maru doctor` | `--vault <path>` | 全面体检二进制环境、知识库、缓存索引与生态挂载健康度 |
| | `--json` | 输出机器可读的结构化诊断报告 |
| `k0maru ui` | `--vault <path>` | 启动本地嵌入式可视化控制台（默认 `127.0.0.1:3721`） |
| | `--port <port>` | 绑定自定义端口（默认 3721） |
| | `--open` | 启动后自动在系统默认浏览器中打开控制台 |
| `k0maru loadout <query>` | `--vault <path>` | 指定目标知识库根目录 |
| | `--diff` | 分析 Git 变更动态激活关联模块的 ADR 与架构铁律 |
| | `--copy` | 将装配好的 <300 Token 背包直接复制至系统剪贴板 |
| | `--json` | 输出结构化 JSON 背包数据 |
| | `-l`, `--list` | 列出知识库内所有活跃项目 |
| `k0maru search <query>` | `--vault <path>` | 指定目标知识库根目录 |
| | `--mode <mode>` | 检索模式（`hybrid` [默认], `bm25`, `vector`） |
| | `--embed-model <name>`| 指定向量嵌入模型（`all-minilm-l6-v2`, `bge-small-zh-v1.5` 等） |
| | `--limit <N>` | 最多返回的结果数量（默认 5 篇） |
| | `--json` | 输出结构化 JSON 检索结果 |
| `k0maru sync` | `--vault <path>` | 增量扫描并更新 SQLite 索引与图谱拓扑 |
| | `--vector` | 增量批量生成与刷新本地向量嵌入 |
| | `--embed-model <name>`| 选择向量生成所采用的模型 |
| | `--force` | 忽略时间戳强制全量重建索引 |
| `k0maru offload` | `--threshold <N>` | 触发符号化卸载的日志行数阈值（默认 50 行） |
| | `--task-id <id>` | 绑定任务标识符以便归档追踪 |
| `k0maru inspect <id>` | `<node_id>` | 根据 Node ID 调取已卸载的原始报错堆栈切片 |
| `k0maru flush` | `--vault <path>` | 根据知识库规约自适应沉淀关键决策或研发心得 |
| | `--title <title>` | 待沉淀笔记的主题/标题 |
| | `--category <cat>` | 笔记类别 (`decision`, `log`, `concept` 等) |
| | `--summary <text>` | 一句话总结与结论 |
| | `--tags <t1,t2>` | 关联标签（英文逗号分隔） |
| | `--related <r1,r2>`| 关联笔记标题（自动编织为 `[[WikiLinks]]` 双链） |
| | `--dry-run` | 仅在终端预览生成结果与路径，不实际落盘 |
| `k0maru distill` | `--trace-file <path>`| 将错误追踪原始文件提炼为标准化排错技能卡片 |
| | `--target <ecosystem>`| 导出目标生态：`default` (Obsidian 知识库) 或 `hermes` (Hermes Agent) |
| `k0maru gc` | `--input <path>` | 传入多轮轨迹 JSON 文件执行 4 级会话上下文垃圾回收 |
| | `--threshold <N>` | 折叠工具输出的行数阈值（默认 20 行） |
| `k0maru guard check` | `--diff` | 毫秒级审计 Git 未提交改动是否违背 Living ADR 不变量 (<10ms) |
| | `--strict` | 严格模式：将所有告警视为阻断错误 |
| | `--json` | 输出结构化 JSON 审计报告 |
| `k0maru guard install` | `--path <repo_dir>` | 向指定 Git 仓库安装 `.git/hooks/pre-commit` 自动阻断钩子 |
| `k0maru mcp` | `--vault <path>` | 启动标准 stdio FastMCP 服务，挂载全部 8 大记忆工具 |
| | `--sidecar` | 启动面向 `omp` 的正交 5 工具伴生模式（剔除日志卸载与 GC） |
| `k0maru pack export` | `--name <name> --tag <tag> -o <file.pack.json>` | 导出可移植知识包 (.pack.json)，打包笔记与 ADR 规则 |
| `k0maru pack import` | `<file.pack.json> [--dry-run]` | 将知识包导入目标 LLM-Wiki，自适应目标目录与规约 |
| `k0maru pack inspect` | `<file.pack.json>` | 预览知识包内容、元数据与目标路径预测 |

---

## 📊 实测性能基准：认知记忆核心与活体 ADR 守卫 (Empirical Benchmarks)

以下数据基于受 Zero-Burn 守卫（`runtime.unassign()`）保护的 **Google Colab Linux 真实虚拟机**（Intel Xeon @ 2.20GHz, 12.67 GB RAM），并与本地 **Apple M5 Pro Mac** 及 **Linux 工作站 (x86_64)** 进行横向对齐实测：

| 评估维度 (Benchmark Dimension) | 传统基线 (全量 Markdown RAG / 人工 CI) | Google Colab Linux VM | Apple M5 Pro Mac | 核心架构优势与能效表现 |
| :--- | :---: | :---: | :---: | :--- |
| **单条记忆召回载荷** | 611 字节 (112 Tokens) | **164 字节 (22 Tokens)** | **164 字节 (22 Tokens)** | **73.16% Token 压缩**，紧凑 `!μ:` K-DSL 输出 |
| **知识库增量同步 (`k0maru sync`)** | 500ms ~ 2,000ms | **34.48 ms** | **9.50 ms** | 纯 Rust 极速增量 SQLite 索引与图谱构建 |
| **全库混合检索 (BM25 + 向量 RRF)** | 1,500ms+ (远端 API RAG) | **495.04 ms** | **180.20 ms** | 纯本地 ONNX 向量推断与倒数秩融合 |
| **活体 ADR 守卫 (`guard check --diff`)** | 分钟级 (人工审查 / CI 流程) | **17.40 ms** | **4.20 ms** | 毫秒级提交前正则与 Diff 不变量审计与自愈提示 (<20ms) |
| **架构不变量阻断治理** | 容易漂移 / 依赖口头纪律 | **物理严格阻断 (`deny`)** | **物理严格阻断 (`deny`)** | 根治跨层违规、裸 SQL 与破坏性调用 |

---

## 🔒 数据隐私与洁净室原则 (Privacy & Cleanroom)

- **100% 本地离线与零遥测**：K0maru 绝不收集、不缓存、不上传任何笔记、代码或检索词。所有索引、向量与守卫规则推演完全在开发者物理机本地完成；
- **开源公域评测数据合规**：所有基准测试数据与故障提炼规则，均来源于公开、合规且可复现的开源语料（SWE-bench 智能体轨迹、GitHub Actions 公开构建日志、BEIR / CoIR 基准），绝无任何私有用户数据参与。详见 [docs/TUNING_AND_DATA_zh.md](docs/TUNING_AND_DATA_zh.md)。

---

## 🛠️ 开发者专属定制与调优 (Customization)

- **知识库规约定制**：在知识库根目录的 `.k0maru/rules.md` 或 `AGENTS.md` 中自由声明分类映射（如 `decisions: docs/adr`, `logs: 01_AI_Logs`）；
- **活体 ADR 不变量声明**：在任何 `20_Cards/` 的 ADR Frontmatter 中声明 `invariants`，无需额外重启守护进程，实时生效；
- **低资源极简运行**：在轻量 VPS（<1GB 内存）上使用 `--mode bm25`，获得亚毫秒级词法检索，0MB ONNX 内存开销；
- **日志阈值微调**：通过 `--threshold <N>` 按模型上下文窗口大小自由调整截断阈值。详见 [开发者调优指南](docs/TUNING_AND_DATA_zh.md)。

---

## 🙏 致谢与开源参考 (Acknowledgments)

K0maru-Agent-Memory 的设计直接吸收了开源社区与前沿研究的优秀思想，在此向以下项目致以诚挚敬意：

1. **Andrej Karpathy ([LLM-Wiki](https://gist.github.com/karpathy))**：由平铺 Markdown 与双链构成的知识复利模式与“文件系统即唯一真理”原则。
2. **腾讯云 ([TencentDB-Agent-Memory](https://github.com/TencentCloud/TencentDB-Agent-Memory))**：“Mermaid 符号化日志卸载”与“L0-L3 语义分层”架构思想。本项目基于 Rust 进行了 100% 独立的净室重构，解耦为纯 CLI 管道过滤器。
3. **Colby McHenry ([CodeGraph](https://github.com/colbymchenry/codegraph))**：100% 本地 Rust 预索引与双链拓扑图谱的高性能架构思想。
4. **Nous Research ([Hermes Agent](https://github.com/nousresearch/hermes-agent))**：从执行轨迹中自生长并沉淀标准化技能的动态经验循环概念。
5. **Anthropic ([Model Context Protocol](https://modelcontextprotocol.io/))**：标准化的 stdio 进程间 JSON-RPC 2.0 协议规范。
6. **开源底层基础库**：[`pulldown-cmark`](https://github.com/pulldown-cmark/pulldown-cmark)、[`rusqlite`](https://github.com/rusqlite/rusqlite)、[`sqlite-vec`](https://github.com/asg017/sqlite-vec)、[`fastembed-rs`](https://github.com/Anush008/fastembed-rs)、[`axum`](https://github.com/tokio-rs/axum)、[`clap`](https://github.com/clap-rs/clap)。

---

## ⚖️ 开源协议 (License)

本项目采用 [MIT License](LICENSE) 授权开源。
