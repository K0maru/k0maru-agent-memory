# CLI 完整命令与参数手册

K0maru 遵循 Unix 管道与命令行哲学，提供了一套功能完备、参数清晰、支持结构化 JSON 输出的子命令体系。

---

## 常用命令一览速查

| 子命令 | 主要用途 | 典型调用示例 |
| :--- | :--- | :--- |
| [`loadout`](#k0maru-loadout) | 生成受控在 <300 Token 的项目上下文读档背包 | `k0maru loadout my-proj --copy` |
| [`search`](#k0maru-search) | 多路混合检索（BM25 + 向量 + 图谱加权） | `k0maru search "mutex deadlocks" --limit 5` |
| [`offload`](#k0maru-offload) | 管道过滤截断终端超长日志并输出 Mermaid 状态机 | `cargo test 2>&1 \| k0maru offload` |
| [`inspect`](#k0maru-inspect) | 凭 Node ID 透视调取已卸载的原始报错调用栈 | `k0maru inspect node_54697ed3` |
| [`sync`](#k0maru-sync) | 毫秒级增量感知并同步 SQLite 索引与向量 | `k0maru sync --vector` |
| [`flush`](#k0maru-flush) | 遵循知识库规约自适应沉淀关键决策或研发记录 | `k0maru flush --title "New ADR" --tags "db"` |
| [`distill`](#k0maru-distill) | 从终端回溯或日志切片提炼可复用的排障技能 | `k0maru distill --node node_xxx --export-hermes`|
| [`install`](#k0maru-install) | 一键安全合并挂载 FastMCP 至各大 AI 客户端 | `k0maru install --vault ~/my-vault` |
| [`doctor`](#k0maru-doctor) | 全面体检二进制环境、知识库、缓存索引健康度 | `k0maru doctor` |
| [`ui`](#k0maru-ui) | 启动本地极客暗黑可视化调试控制台 | `k0maru ui --open` |
| [`mcp`](#k0maru-mcp) | 启动基于标准 stdio 的 FastMCP 服务端 | `k0maru mcp --vault ~/my-vault` |

---

## 全量子命令详尽参考

### `k0maru loadout`

用于在新会话开辟前提取目标项目的愿景与核心架构约束（L3），严格控制在 `< 300 Token`。

```bash
k0maru loadout [QUERY] [OPTIONS]
```

#### 参数说明
- `[QUERY]`：目标项目名称或检索关键词（例如 `payment-gateway`）。若省略且未指定 `--list`，则默认匹配首个活跃项目；
- `-v, --vault <PATH>`：指定目标 Markdown 知识库根目录。默认自底向上探测当前工作目录；
- `-c, --copy`：将格式化后的 Markdown 内容直接复制到系统剪贴板（兼容 macOS、Linux 与 Windows）；
- `-l, --list`：列出当前知识库内识别到的所有活跃项目列表；
- `--json`：输出结构化 JSON 格式数据。

#### `--json` 输出结构示例
```json
{
  "project_name": "payment-gateway",
  "vault_path": "/Users/k0maru3/workspace/SecondBrain",
  "token_count": 218,
  "markdown": "# Context Loadout: Payment Gateway Core\n- Stack: Go 1.22, Redis 7...",
  "l3_invariants": [
    "ADR-001: 支付回调幂等性防重规约"
  ],
  "recent_logs": []
}
```

---

### `k0maru search`

多路混合检索知识库内容。

```bash
k0maru search <QUERY> [OPTIONS]
```

#### 参数说明
- `<QUERY>`：必填，搜索查询词或自然语言问题；
- `-v, --vault <PATH>`：指定知识库根目录；
- `-m, --mode <MODE>`：检索模式，可选：
  - `hybrid`（默认）：BM25 词法 + ONNX 向量 + 1-Hop 图谱拓扑加权 RRF 混合检索；
  - `bm25`：纯 FTS5 倒排索引检索（极速、0 额外内存）；
  - `vector`：纯语义稠密向量检索；
- `-l, --limit <N>`：返回结果数量上限（默认：`5`）；
- `--json`：输出机器可读的 JSON 命中数组。

#### `--json` 输出结构示例
```json
[
  {
    "path": "20_Cards/adr-001-idempotency.md",
    "title": "ADR-001: 支付回调幂等性防重规约",
    "score": 0.0325,
    "bm25_rank": 1,
    "vector_rank": 1,
    "graph_boost": 0.05,
    "snippet": "系统必须严格遵守以下两条架构铁律：1. 必须基于 SET lock:payment:{event_id} 抢占分布式锁..."
  }
]
```

---

### `k0maru offload`

标准 Unix 管道过滤器。当终端日志超过阈值时进行截断并输出符号化 Mermaid 状态机图。

```bash
<COMMAND> | k0maru offload [OPTIONS]
```

#### 参数说明
- `-t, --threshold <N>`：触发日志截断的行数阈值（默认：`50` 行）；
- `--task-id <ID>`：可选的任务标识符前缀；
- `-r, --refs-dir <PATH>`：指定离线日志切片的持久化目录（默认：`.k0maru/refs/` 或 `.scratch/refs/`）。

---

### `k0maru inspect`

根据 Node ID 提取由 `offload` 截断的完整原始报错切片。

```bash
k0maru inspect <NODE_ID> [OPTIONS]
```

#### 参数说明
- `<NODE_ID>`：必填，日志切片节点 ID（形如 `node_54697ed3`）；
- `-r, --refs-dir <PATH>`：指定日志切片所在目录。

---

### `k0maru sync`

增量扫描知识库文件系统并将元数据、FTS 索引与向量同步至本地 SQLite 缓存。

```bash
k0maru sync [OPTIONS]
```

#### 参数说明
- `-v, --vault <PATH>`：指定知识库根目录；
- `-f, --force`：强制全量重新解析所有文档，忽略 `mtime` 与哈希缓存；
- `--vector`：增量计算并刷新文档的向量嵌入；
- `--json`：输出同步统计信息 JSON。

---

### `k0maru flush`

遵循知识库既有规约，将关键决策或研发心得持久化结晶为 Markdown 卡片。

```bash
k0maru flush [OPTIONS]
```

#### 参数说明
- `--title <TITLE>`：必填，沉淀笔记的标题；
- `--category <CAT>`：分类名称（`decision`、`log`、`concept` 等，默认：`log`）；
- `--summary <TEXT>`：可选，一句话结论摘要；
- `--content <TEXT>`：可选，Markdown 正文（亦支持通过标准输入管道传入）；
- `--tags <T1,T2>`：逗号分隔的标签数组；
- `--related <R1,R2>`：逗号分隔的关联笔记标题（自动编织为 `[[WikiLinks]]`）；
- `--dry-run`：仅在控制台预览生成路径与内容，不实际落盘；
- `-v, --vault <PATH>`：指定知识库根目录；
- `--json`：输出结构化写入结果。

---

### `k0maru distill`

从终端排障堆栈或日志切片中自动提炼可复用的排障 Playbook。

```bash
k0maru distill [OPTIONS]
```

#### 参数说明
- `--node <NODE_ID>`：指定要提炼的离线切片 Node ID；
- `-f, --file <PATH>`：指定要提炼的原始日志文件路径；
- `-t, --title <TITLE>`：显式指定技能笔记标题（省略则由规则自动推断）；
- `-c, --context <HINT>`：排障上下文补充提示语；
- `--category <CAT>`：目标分类（默认：`skill`）；
- `--export-hermes`：一键将提炼出的技能卡片直接导出到 `~/.hermes/skills/`；
- `--target <TARGET>`：生态目标格式（`default` 或 `hermes`）；
- `--dry-run`：仅预览提炼结果，不写盘；
- `--json`：输出结构化 JSON。

---

### `k0maru install`

一键为各 AI 编码客户端配置注入 FastMCP 协议条目。

```bash
k0maru install [OPTIONS]
```

#### 参数说明
- `-v, --vault <PATH>`：指定待挂载的目标知识库路径；
- `-t, --target <CLIENT>`：指定要配置的客户端（`all`、`claude`、`cursor`、`gemini`、`windsurf`、`cline`、`hermes`、`openclaw`，默认：`all`）；
- `--home <PATH>`：覆盖用户主目录（用于沙箱测试）；
- `--dry-run`：预览配置变更差异，不执行写盘；
- `--json`：输出安装结果 JSON 报表。

---

### `k0maru doctor`

环境自检与健康诊断。

```bash
k0maru doctor [OPTIONS]
```

#### 参数说明
- `-v, --vault <PATH>`：指定待诊断的知识库路径；
- `--home <PATH>`：覆盖用户主目录；
- `--json`：输出包含每个检查项布尔值与详情的结构化 JSON。

---

### `k0maru ui`

拉起本地内嵌式可视化调试工作台。

```bash
k0maru ui [OPTIONS]
```

#### 参数说明
- `-v, --vault <PATH>`：指定待浏览的知识库路径；
- `-p, --port <PORT>`：绑定端口号（默认：`3721`）；
- `--open`：启动后自动唤起系统默认浏览器。

---

### `k0maru mcp`

以 FastMCP Stdio 模式启动服务，供智能体客户端接入。

```bash
k0maru mcp [OPTIONS]
```

#### 参数说明
- `-v, --vault <PATH>`：指定挂载的知识库路径。

---

## 🚦 环境变量与退出码

### 环境变量
- `K0MARU_VAULT`：全局指定知识库根目录路径（当省略 `--vault` 时生效）；
- `HERMES_HOME`：覆盖 Hermes Agent 默认配置发现目录；
- `OPENCLAW_HOME`：覆盖 OpenClaw 默认配置发现目录。

### 进程退出码 (Exit Codes)
- `0`：执行成功（Success）；
- `1`：常规执行失败（General Error，例如参数解析错误、路径不存在）；
- `2`：知识库规约冲突或文件读写受阻；
- `130`：开发者通过 `Ctrl+C` 主动中断退出（SIGINT）。
