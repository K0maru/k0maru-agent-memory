# CLI 与 FastMCP 协议完整参考手册

K0maru 遵循 Unix 管道哲学与现代智能体编排标准，纯 Rust 静态编译为单个极简二进制（提供 `k0maru` 与 `k0maru-memory` 双二进制别名），零常驻后台守护进程、零后台网络端口占用。

本手册提供 **CLI 全部子命令**、**FastMCP JSON-RPC 2.0 协议契约**、**知识包引擎 (`k0maru pack`)**、**omp 伴生模式 (`--sidecar`)** 以及 **终端多路复用与插件市场生态** 的完整规范说明。

---

## 🚀 CLI 完整指令速查表

| 子命令 | 核心职责 | 核心参数 | 典型调用示例 |
| :--- | :--- | :--- | :--- |
| [`loadout`](#k0maru-loadout) | 生成 <300 Token 项目上下文装配背包 | `[QUERY]`, `--diff`, `--copy`, `--json` | `k0maru loadout payment-api --diff` |
| [`search`](#k0maru-search) | 混合检索（BM25 + 本地向量 + 双链拓扑） | `<QUERY>`, `--mode`, `--limit`, `--json` | `k0maru search "互斥锁死锁" --limit 5` |
| [`offload`](#k0maru-offload) | 管道过滤超长日志，符号化生成 Mermaid 状态图 | `--threshold`, `--task-id` | `cargo test 2>&1 \| k0maru offload` |
| [`inspect`](#k0maru-inspect) | 根据 Node ID 调取已卸载的完整报错切片 | `<node_id>` | `k0maru inspect node_54697ed3` |
| [`sync`](#k0maru-sync) | 增量/全量同步知识库元数据、FTS 索引与向量 | `--vector`, `--embed-model`, `--force` | `k0maru sync --vector` |
| [`flush`](#k0maru-flush) | 沉淀技术决策（ADR）或里程碑经验至 Markdown | `--title`, `--category`, `--summary`, `--tags` | `k0maru flush --title "Redis Mutex" --category decision` |
| [`distill`](#k0maru-distill) | 从错误追踪或日志中提炼标准化排错技能手册 | `--trace-file`, `--node`, `--target` | `k0maru distill --node node_xxx --target hermes` |
| [`gc`](#k0maru-gc) | 4 级会话轨迹上下文垃圾回收（TRR $\ge 85\%$） | `--input`, `--threshold` | `k0maru gc --input session.json` |
| [`guard check`](#k0maru-guard-check) | 毫秒级审计 Git Diff 是否违背 ADR 不变量 (<10ms) | `--diff`, `--strict`, `--json` | `k0maru guard check --diff` |
| [`guard install`](#k0maru-guard-install) | 向仓库安装 Git Pre-commit 自动阻断钩子 | `--path` | `k0maru guard install` |
| [`install`](#k0maru-install) | 一键安全挂载 FastMCP 配置至各 AI 智能体 | `--vault`, `--target`, `--dry-run` | `k0maru install --target omp --vault ~/Vault` |
| [`doctor`](#k0maru-doctor) | 全面诊断二进制、知识库拓扑与客户端生态状态 | `--vault`, `--json` | `k0maru doctor` |
| [`ui`](#k0maru-ui) | 启动内嵌式可视化开发者控制台驾驶舱 | `--port`, `--open` | `k0maru ui --port 3721 --open` |
| [`pack`](#k0maru-pack) | 导出、导入或预览可移植知识包 (.pack.json) | `export`, `import`, `inspect` | `k0maru pack export --name redis-rules -o pack.json` |
| [`mcp`](#k0maru-mcp) | 启动 FastMCP stdio 服务（标准全功能或 sidecar 模式） | `--vault`, `--sidecar`, `--embed-model` | `k0maru mcp --sidecar` |

---

## 📖 CLI 详细子命令手册

### `k0maru loadout`

提取项目愿景、技术栈声明与 L3 活体架构不变量，生成严格受控在 `< 300 Token` 内的紧凑 Markdown 背包，防止大模型发生注意力漂移或幻觉。

```bash
k0maru loadout [QUERY] [OPTIONS]
```

#### 参数与选项
- `[QUERY]`：目标项目名称或检索关键字。若缺省且未提供 `--list`，自动匹配知识库中发现的首个活跃项目。
- `-v, --vault <PATH>`：目标 Markdown 知识库根目录（缺省自动向上嗅探）。
- `--diff`：动态分析当前工作区 Git Diff，按模块变更精准激活关联 ADR 规则。
- `-c, --copy`：将格式化装配好的 Markdown 直接复制至系统剪贴板。
- `-l, --list`：列出知识库中所有已识别的项目。
- `--json`：输出结构化 JSON 数据。
- `--json-stream`：输出换行分隔的流式 JSON（NDJSON）。

---

### `k0maru search`

执行多路混合检索，融合 SQLite FTS5（BM25 词法）、`sqlite-vec`（ONNX 向量嵌入）、RRF 倒数秩融合（$k=60$）以及 1-hop 双链图谱拓扑加权（+0.05）。

```bash
k0maru search <QUERY> [OPTIONS]
```

#### 参数与选项
- `<QUERY>`：检索词、技术问题或概念短语。
- `-v, --vault <PATH>`：目标 Markdown 知识库根目录。
- `-m, --mode <MODE>`：检索模式（`hybrid` [默认混合], `bm25` [纯词法], `vector` [纯向量]）。
- `--embed-model <MODEL>`：ONNX 向量嵌入模型（默认 `all-minilm-l6-v2`，支持 `bge-small-zh-v1.5` 等）。
- `-l, --limit <N>`：最多返回的结果条数（默认：5）。
- `--json`：输出结构化 JSON 检索命中列表。

---

### `k0maru offload`

标准 Unix 管道过滤器。从标准输入（stdin）接管编译、测试或运行产生的海量日志，将冗长堆栈切片保存至本地磁盘，并在标准输出（stdout）发射紧凑的 Mermaid 状态机图。

```bash
<COMMAND> 2>&1 | k0maru offload [OPTIONS]
```

#### 参数与选项
- `--threshold <N>`：触发日志外置卸载的最小行数阈值（默认：50 行）。
- `--task-id <ID>`：绑定下游任务标识符，便于全链路跟踪回溯。

---

### `k0maru inspect`

通过 Node ID 精准调取此前被 `k0maru offload` 截断外置的原始未压缩错误堆栈。

```bash
k0maru inspect <NODE_ID>
```

#### 位置参数
- `<NODE_ID>`：Mermaid 状态机图中所标注的节点标识符（如 `node_54697ed3`）。

---

### `k0maru sync`

增量或全量扫描知识库笔记、解析 YAML Frontmatter、编织 WikiLinks 图谱拓扑并将文本向量化写入瞬态 `cache.sqlite`。

```bash
k0maru sync [OPTIONS]
```

#### 参数与选项
- `-v, --vault <PATH>`：目标 Markdown 知识库根目录。
- `--vector`：增量计算并缓存新笔记或已变更笔记的向量嵌入。
- `--embed-model <MODEL>`：指定向量模型。
- `--force`：忽略文件时间戳，强制全量自愈重建索引。

---

### `k0maru flush`

将技术决策（ADR）、排障经验或任务里程碑安全沉淀为本地 Markdown 笔记，自动根据知识库目录形态（Flat、Decisions、PARA、Johnny.Decimal）归档并注入规约 Frontmatter。

```bash
k0maru flush [OPTIONS]
```

#### 参数与选项
- `-v, --vault <PATH>`：目标 Markdown 知识库根目录。
- `-t, --title <TITLE>`：笔记标题。
- `-c, --category <CATEGORY>`：分类标识（`decision`, `log`, `concept`, `rule`）。
- `-s, --summary <TEXT>`：一句话总结与核心结论。
- `--tags <TAGS>`：英文逗号分隔的关联标签（如 `redis,idempotency`）。
- `--related <TITLES>`：英文逗号分隔的关联笔记标题（自动编织为 `[[WikiLinks]]`）。
- `--dry-run`：仅在控制台预览生成路径与 Markdown 内容，不实际写入磁盘。
- `--json`：输出结构化 JSON 结果。
- `--stdin`：从标准输入直接读取笔记正文。

---

### `k0maru distill`

从原始排障日志或已卸载的堆栈中提炼标准化排错技能手册，智能抽离现象、根因排查路径与验证方法。

```bash
k0maru distill [OPTIONS]
```

#### 参数与选项
- `--trace-file <PATH>`：原始日志或崩溃追踪文件路径。
- `--node <NODE_ID>`：直接从缓存存储中指定已卸载的 Node ID。
- `-t, --target <TARGET>`：输出格式（`default` 输出至 Markdown 知识库，`hermes` 导出为 Nous Research Hermes Agent Skill 规范）。
- `--dry-run`：预览技能卡片，不写入磁盘。
- `--json`：输出结构化 JSON。

---

### `k0maru gc`

对长达 50+ 轮的多轮对话轨迹执行 4 级上下文垃圾回收协议，折叠工具冗余输出，实现 $\ge 85\%$ 的 Token 压缩率（TRR），同时保护最新轮次的认知黄金区。

```bash
k0maru gc [OPTIONS]
```

#### 参数与选项
- `-i, --input <PATH>`：多轮对话记录 JSON 文件路径。
- `--threshold <N>`：折叠工具输出的行数阈值（默认：20 行）。
- `--json`：在标准输出打印剪枝后的对话 JSON。

---

### `k0maru guard`

在 $<10\text{ ms}$ 内毫秒级审计 Git 未提交修改是否满足知识库中声明的 Living ADR 架构铁律。

#### `k0maru guard check`

```bash
k0maru guard check [OPTIONS]
```

- `--diff`：针对当前工作区 Git Diff 进行架构规则匹配审计。
- `--strict`：严格模式，将所有警告（`warn`）提升为阻断性错误（`deny`）。
- `--json`：输出包含违规规则 ID、违规文件与修复建议的结构化 JSON 审计报告。

#### `k0maru guard install`

```bash
k0maru guard install [OPTIONS]
```

- `-p, --path <REPO_DIR>`：指定目标 Git 仓库目录。向 `.git/hooks/pre-commit` 安装轻量校验钩子，在本地提交时自动阻断违背 ADR 的代码合入。

---

### `k0maru install`

通过安全、无损的 JSON 合并机制，将 K0maru 的 FastMCP 服务自动挂载到已安装的 AI 编码智能体配置文件中。

```bash
k0maru install [OPTIONS]
```

#### 参数与选项
- `-v, --vault <PATH>`：目标 Markdown 知识库根目录。
- `-t, --target <CLIENT>`：指定要配置的客户端：
  - `all`（默认：自动嗅探并配置所有检测到的客户端）
  - `claude`（Claude Code）
  - `cursor`（Cursor IDE）
  - `gemini`（Gemini CLI）
  - `windsurf`（Windsurf）
  - `cline`（Cline / Roo Code）
  - `hermes`（Nous Research Hermes Agent）
  - `openclaw`（OpenClaw Agent）
  - `omp`（Oh My Pi —— 自动注入 `--sidecar` 伴生模式参数）
- `--home <PATH>`：自定义用户 Home 目录路径（适用于隔离测试或容器环境）。
- `--dry-run`：仅在终端预览配置文件差异，不实际写入磁盘。
- `--json`：以 JSON 格式输出安装报告。

```bash
# 一键配置 Oh My Pi (omp) 伴生模式
k0maru install --target omp --vault ~/Documents/MyVault
```

---

### `k0maru doctor`

对二进制路径、知识库规范、瞬态缓存完整性、向量维度与各智能体客户端挂载配置执行端到端健康体检。

```bash
k0maru doctor [OPTIONS]
```

#### 参数与选项
- `-v, --vault <PATH>`：目标 Markdown 知识库根目录。
- `--home <PATH>`：覆盖用户 Home 目录。
- `--json`：输出机器可读的诊断状态 JSON。

---

### `k0maru ui`

启动本地嵌入式可视化开发者控制台，提供免 CDN 的交互式混合检索测试、图谱拓扑全景与实时指标看板。

```bash
k0maru ui [OPTIONS]
```

#### 参数与选项
- `-v, --vault <PATH>`：目标 Markdown 知识库根目录。
- `-p, --port <PORT>`：绑定的本地端口（默认：`3721`）。
- `--open`：启动后自动在系统默认浏览器中打开控制台。
- `--embed-model <MODEL>`：指定向量模型。

---

### `k0maru pack`

打包、导入与预览可移植的 **知识包（Memory Packs，`.pack.json`）**。知识包将领域经验、笔记卡片与 Living ADR 架构不变量从具体的目录结构中解耦，实现跨知识库与跨智能体迁移。

```bash
k0maru pack <COMMAND> [OPTIONS]
```

#### 子命令

#### 1. `k0maru pack export`

扫描源知识库，提取笔记及其关联的 Living ADR 规则，自动剥离特定目录前缀的 `[[WikiLinks]]`（例如 `[[20_Cards/Foo|Bar]]` $\to$ `[[Foo|Bar]]`），并打包为结构化 JSON 文件。

```bash
k0maru pack export --name <NAME> [OPTIONS]
```

- `--name <NAME>`：知识包唯一命名（必填）。
- `-v, --vault <PATH>`：源知识库根目录（缺省自动嗅探）。
- `--version <SEMVER>`：知识包版本号（默认：`0.1.0`）。
- `--description <DESC>`：知识包描述。
- `--tag <TAG>`：仅打包匹配特定标签的笔记（如 `topic/redis`）。
- `--query <QUERY>`：仅打包匹配搜索词的笔记。
- `--files <FILE...>`：显式指定打包的文件列表。
- `-o, --output <FILE>`：导出目标路径（如 `redis-rules.pack.json`）。
- `--json`：在标准输出打印原始 JSON 数据。

**使用示例**：
```bash
k0maru pack export --name redis-resilience --tag topic/redis -o redis.pack.json
```

#### 2. `k0maru pack import`

将知识包导入至任意 LLM-Wiki。K0maru 的 `FlushEngine` 会自动嗅探目标知识库的目录规律与命名习惯并完成自适应落盘：
- **Karpathy 扁平 LLM-Wiki**：笔记自动落入根目录，文件名为 kebab-case 格式；
- **工程 Wiki**：决策自动进入 `decisions/` 或 `docs/adr/`；
- **PARA 体系**：自动进入 `cards/` 或 `projects/`；
- **Johnny.Decimal / SecondBrain**：自动进入 `20_Cards/` 或 `10_Projects/`。

导入的笔记完整保留 YAML Frontmatter 中的 `invariants`，导入完成后目标仓库即可立即使用 `k0maru guard check` 进行守卫。

```bash
k0maru pack import <PACK_FILE> [OPTIONS]
```

- `<PACK_FILE>`：`.pack.json` 文件路径（必填）。
- `-v, --vault <PATH>`：目标知识库根目录（缺省自动嗅探）。
- `--dry-run`：预览将要生成的笔记与预测目标路径，不实际写入磁盘。
- `--json`：输出结构化 JSON 导入结果。

**使用示例**：
```bash
k0maru pack import redis.pack.json --dry-run
k0maru pack import redis.pack.json
```

#### 3. `k0maru pack inspect`

在不实际写入的情况下，预览知识包的元数据、所含笔记条数、包含的 Living ADR 不变量列表以及针对目标知识库的预测落盘路径。

```bash
k0maru pack inspect <PACK_FILE> [OPTIONS]
```

- `<PACK_FILE>`：`.pack.json` 文件路径（必填）。
- `-v, --vault <PATH>`：可选的目标知识库路径，用于精确测算落盘相对路径。
- `--json`：输出结构化 JSON 报告。

**使用示例**：
```bash
k0maru pack inspect redis.pack.json
```

---

### `k0maru mcp`

启动遵循标准 I/O（`stdio`）的 FastMCP JSON-RPC 2.0 服务。

```bash
k0maru mcp [OPTIONS]
```

#### 参数与选项
- `-v, --vault <PATH>`：目标 Markdown 知识库根目录。
- `--embed-model <MODEL>`：ONNX 向量嵌入模型（`all-minilm-l6-v2`、`bge-small-zh-v1.5` 等）。
- `--sidecar`：启用面向 `omp`（Oh My Pi）的正交伴生模式，仅暴露 5 大领域记忆与守卫工具，主动剔除终端日志卸载与上下文 GC 工具。

---

## 🤖 FastMCP 协议规范与运行模式

K0maru 严格遵循 Anthropic Model Context Protocol（MCP）`2024-11-05` 规范，采用基于 `stdio` 的 JSON-RPC 2.0 通信。当智能体客户端启动时自动以子进程拉起 `k0maru mcp`，会话结束时进程立即注销，不残留任何后台网络端口与常驻内存。

### 运行模式对比

| 功能特性 | 默认全功能模式 (Full Mode) | 正交伴生模式 (`--sidecar`) |
| :--- | :---: | :---: |
| **适配客户端** | Claude Code, Cursor, Windsurf, Hermes, OpenClaw | Oh My Pi (`omp`) |
| **挂载工具总数** | **8 大工具** | **5 大核心工具** |
| `get_project_loadout` | ✅ 挂载 | ✅ 挂载 |
| `recall_memory` | ✅ 挂载 | ✅ 挂载 |
| `offload_context` | ✅ 挂载 | ❌ 剔除（由 `omp` 原生接管） |
| `inspect_log_node` | ✅ 挂载 | ❌ 剔除（由 `omp` 原生接管） |
| `flush_session` | ✅ 挂载 | ✅ 挂载 |
| `distill_session_skill` | ✅ 挂载 | ✅ 挂载 |
| `gc_session_context` | ✅ 挂载 | ❌ 剔除（由 `omp` 原生接管） |
| `guard_check` | ✅ 挂载 | ✅ 挂载 |

> **为什么为 `omp` 设计 Sidecar 模式？**  
> `omp` 本身具备极佳的终端输出治理、会话压缩与日志流管理能力。在 `omp` 环境下通过 `--sidecar` 启动 K0maru，可以彻底消除工具冲突与流水线冗余，使大模型专注调用知识召回、决策沉淀与架构守卫核心工具。

---

## 🛠️ FastMCP 工具契约与 JSON Schema

### 1. `get_project_loadout`
*可用模式：全功能模式 & Sidecar 伴生模式*

装配当前项目的核心愿景、技术栈与活跃 L3 架构不变量，严格限制在 `< 300 Token` 内。支持传入 `diff` 参数实现变更感知，动态激活与本次改动相关的规则。

```json
{
  "name": "get_project_loadout",
  "description": "Fetch high-priority architectural blueprint, core invariants, project vision, and recent ADR decision logs (<300 tokens) from the vault. Supports change-aware diff inspection to dynamically activate module-relevant ADRs.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_name": {
        "type": "string",
        "description": "目标项目名称或检索词。缺省时自动匹配最近或首个活跃项目。"
      },
      "diff": {
        "type": "string",
        "description": "可选的 Git Diff 文本，用于动态激活关联模块的 ADR 规则。"
      }
    }
  }
}
```

---

### 2. `recall_memory`
*可用模式：全功能模式 & Sidecar 伴生模式*

通过 SQLite FTS5（BM25）与 `sqlite-vec`（ONNX 向量）执行混合 RRF 检索（$k=60$），叠加 1-hop 双链图谱加权。默认返回极度紧凑的单行 K-DSL 协议串（`!μ:<tier>|<path>|...`），极大节省智能体上下文。

```json
{
  "name": "recall_memory",
  "description": "Search and retrieve documentation, technical research reports, project context, past architectural decisions, configurations, setup guides, and notes from the user's local knowledge vault.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "检索关键词、概念短语或技术提问。"
      },
      "limit": {
        "type": "integer",
        "description": "最多检索返回的笔记条数（默认：5）。"
      },
      "full": {
        "type": "boolean",
        "description": "设为 true 时返回完整多行 Markdown 正文，而非默认的紧凑单行 K-DSL 串。"
      }
    },
    "required": ["query"]
  }
}
```

---

### 3. `offload_context`
*可用模式：仅全功能模式*

将超长终端输出、测试断言失败堆栈或编译器错误日志（$>30$ 行）外置存储到磁盘切片中，生成紧凑的 Mermaid 状态图直击根因。

```json
{
  "name": "offload_context",
  "description": "Externalize and compress verbose terminal logs, build outputs, test traces, or error dumps (>30 lines) into a compact symbolic Mermaid state diagram on disk.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "content": {
        "type": "string",
        "description": "待压缩并外置存储的原始终端长日志或堆栈文本。"
      },
      "task_id": {
        "type": "string",
        "description": "可选的任务追踪标识符。"
      }
    },
    "required": ["content"]
  }
}
```

---

### 4. `inspect_log_node`
*可用模式：仅全功能模式*

根据 Mermaid 状态图中的 Node ID，精准取回保存在本地磁盘上的完整未截断堆栈。

```json
{
  "name": "inspect_log_node",
  "description": "Retrieve and inspect the full offloaded raw terminal log or compiler trace content by its node ID (e.g. node_1a2b3c4d).",
  "inputSchema": {
    "type": "object",
    "properties": {
      "node_id": {
        "type": "string",
        "description": "Mermaid 状态图中的符号化节点 ID。"
      }
    },
    "required": ["node_id"]
  }
}
```

---

### 5. `flush_session`
*可用模式：全功能模式 & Sidecar 伴生模式*

将架构设计决策、经验突破或排错复盘原子化沉淀到本地 Markdown 笔记中，自动写入 Frontmatter、编织 `[[WikiLinks]]`，并在 $<5\text{ ms}$ 内同步刷新 SQLite 缓存。

```json
{
  "name": "flush_session",
  "description": "Crystallize technical decisions, verified bug fixes, architecture decisions (ADRs), or task milestones back into the local Markdown vault as permanent notes.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "title": {
        "type": "string",
        "description": "笔记标题。"
      },
      "category": {
        "type": "string",
        "description": "分类：decision, log, concept, rule 或自定义。"
      },
      "summary": {
        "type": "string",
        "description": "一句话总结或核心结论。"
      },
      "tags": {
        "type": "array",
        "items": { "type": "string" },
        "description": "关联标签列表（如 [\"redis\", \"mutex\"]）。"
      },
      "related": {
        "type": "array",
        "items": { "type": "string" },
        "description": "需要以 [[WikiLinks]] 双向链结的关联笔记标题列表。"
      },
      "content": {
        "type": "string",
        "description": "详细的 Markdown 正文内容。"
      },
      "invariants": {
        "type": "array",
        "items": { "type": "string" },
        "description": "需要激活的 Living ADR 架构不变量规则（如 [\"deny: SELECT * FROM\"]）。"
      }
    },
    "required": ["title"]
  }
}
```

---

### 6. `distill_session_skill`
*可用模式：全功能模式 & Sidecar 伴生模式*

将一段具体的排障 Trace 或调试过程提炼为标准化的故障排除手册，支持直接存入知识库或导出为 Nous Research Hermes Agent 的标准 Skill。

```json
{
  "name": "distill_session_skill",
  "description": "Distill an execution trace, compilation error, or debugging session into a reusable troubleshooting playbook/skill note and save it to the vault or export to Hermes Agent.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "trace_content": {
        "type": "string",
        "description": "原始 Trace 或错误日志正文。"
      },
      "node_id": {
        "type": "string",
        "description": "已卸载日志的 Node ID。"
      },
      "skill_name": {
        "type": "string",
        "description": "提炼技能的命名。"
      },
      "export_hermes": {
        "type": "boolean",
        "description": "是否导出为 Hermes Agent Skill 格式。"
      }
    }
  }
}
```

---

### 7. `gc_session_context`
*可用模式：仅全功能模式*

对多轮对话消息应用 4 级剪枝协议，自动折叠过往冗长的工具调用输出，消除重复的编译器报错，达成 $\ge 85\%$ 的 Token 压缩率（TRR）。

```json
{
  "name": "gc_session_context",
  "description": "Perform long-trajectory context garbage collection (Context GC) on a multi-turn agent session trajectory, folding verbose tool outputs and repetitive compiler error dumps into symbolic references.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "messages": {
        "type": "array",
        "items": { "type": "object" },
        "description": "符合标准 ChatML 格式的消息数组。"
      },
      "threshold": {
        "type": "integer",
        "description": "折叠工具输出的行数阈值（默认：20 行）。"
      }
    },
    "required": ["messages"]
  }
}
```

---

### 8. `guard_check`
*可用模式：全功能模式 & Sidecar 伴生模式*

在 $<10\text{ ms}$ 内高速审计 Git Diff 或代码变更是否满足知识库中声明的 Living ADR 架构铁律。

```json
{
  "name": "guard_check",
  "description": "Audit git diff or code changes against active Living ADR architectural invariants and project rules (<10ms).",
  "inputSchema": {
    "type": "object",
    "properties": {
      "diff": {
        "type": "string",
        "description": "待审计的 Git Diff 文本。缺省时自动审计当前仓库未提交的改动。"
      },
      "project_name": {
        "type": "string",
        "description": "可选的项目名，用于缩小规则范围。"
      }
    }
  }
}
```

---

## 🔌 终端多路复用与插件市场生态

### Tern 终端复用器原生插件

K0maru 为 **Tern** 终端多路复用器提供了纯 Luau 编写的原生插件（位于 `tern-plugin/`）：
- **Cockpit 驾驶舱看板块**：交互式终端块，实时呈现二进制健康度、SQLite 索引状态与 Living ADR 合规仪表条（输入 `r` 即可原地刷新）；
- **CLI 命令透镜**：拦截 `k0maru *` 命令并将 JSON 或表格输出渲染为原生 UI 卡片与表格；
- **Window 调色板指令**：集成 `k0maru: cockpit`、`k0maru: doctor`、`k0maru: guard-check` 以及 Carly `recall` 导出。

```bash
# 一键安装 Tern 原生插件
tern plugin install github.com/K0maru/k0maru-agent-memory/tern-plugin
```

### Oh My Pi (`omp`) 插件市场安装

K0maru 已发布至 `omp` 官方插件目录，安装后自动以 `--sidecar` 伴生模式挂载：

```text
/marketplace add K0maru/k0maru-agent-memory
/marketplace install k0maru-memory@k0maru-agent-memory
```

### Claude Code 插件市场安装

```text
/plugin add K0maru/k0maru-agent-memory
```

---

## 🚦 环境变量与退出码

### 环境变量
- `K0MARU_VAULT`：当命令行未传入 `--vault` 时指定的默认知识库根路径。
- `HERMES_HOME`：Nous Research Hermes Agent 的自定义发现目录路径。
- `OPENCLAW_HOME`：OpenClaw Agent 的自定义发现目录路径。

### 退出状态码
- `0`：执行成功（Clean Pass）。
- `1`：存在阻断性架构违规（`deny`）或业务执行失败。
- `2`：环境错误或配置不全。
- `130`：开发者主动中断（`Ctrl+C` / SIGINT）。
