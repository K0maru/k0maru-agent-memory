# CLI 完整命令与参数手册 (v1.0.0)

K0maru 遵循 Unix 管道哲学与现代智能体编排标准，编译为单个纯 Rust 静态二进制文件（`k0maru` 与 `k0maru-memory`），运行期间 0 常驻后台守护进程、0 Docker 容器占用。

本手册提供 K0maru v1.0.0 全量 **14 个 CLI 子命令**的详细参数规范、调用选项与 JSON 输出 Schema。

---

## 🚀 常用命令一览速查

| 子命令 | 主要用途 | 核心参数与标志 | 典型调用示例 |
| :--- | :--- | :--- | :--- |
| [`init` / `bootstrap`](#k0maru-init--bootstrap) | 初始化零配置代码库 LLM-Wiki 与架构守卫 | `--repo`, `--vault`, `--install`, `--project` | `k0maru init --repo . --install` |
| [`loadout`](#k0maru-loadout) | 生成受控在 <300 Token 的项目上下文读档背包 | `[QUERY]`, `--diff`, `--branch`, `--copy`, `--json` | `k0maru loadout payment-api --diff` |
| [`guard check`](#k0maru-guard-check) | 审计 Git 变更是否违反 Living ADR 规则 (<10ms) | `--diff`, `--claude-stop-hook`, `--strict`, `--json` | `k0maru guard check --diff` |
| [`guard install`](#k0maru-guard-install) | 安装 Git pre-commit 钩子自动拦截违规代码 | `--path`, `--hook`, `--vault` | `k0maru guard install` |
| [`flush`](#k0maru-flush) | 遵循知识库规约自适应沉淀关键决策或研发记录 | `--title`, `--category`, `--supersedes`, `--tags` | `k0maru flush --title "New ADR" --supersedes ADR-001` |
| [`search`](#k0maru-search) | 多路混合检索（BM25 + 向量 + 图谱加权，自动同步） | `<QUERY>`, `--mode`, `--limit`, `--embed-model` | `k0maru search "mutex deadlock" --limit 5` |
| [`install`](#k0maru-install) | 一键安全合并挂载 FastMCP 至各大 AI 客户端与生命周期钩子 | `--vault`, `--target`, `--hooks`, `--sidecar` | `k0maru install --target all --hooks` |
| [`pack`](#k0maru-pack) | 导出、导入或透视便携式知识包（Memory Pack） | `export`, `import`, `inspect` | `k0maru pack export --name redis-rules -o pack.json` |
| [`sync`](#k0maru-sync) | 毫秒级增量感知并同步 SQLite 索引与向量 | `--vector`, `--embed-model`, `--force`, `--json` | `k0maru sync --vector` |
| [`offload`](#k0maru-offload) | 管道过滤截断终端超长日志并输出 Mermaid 状态机 (99.4% 压缩) | `--threshold`, `--task-id`, `--refs-dir` | `cargo test 2>&1 \| k0maru offload` |
| [`inspect`](#k0maru-inspect) | 凭 Node ID 透视调取已卸载的原始报错调用栈 | `<node_id>`, `--refs-dir` | `k0maru inspect node_54697ed3` |
| [`distill`](#k0maru-distill) | 从终端回溯或日志切片提炼可复用的排障技能 | `--node`, `--file`, `--target`, `--export-hermes` | `k0maru distill --node node_xxx --target hermes` |
| [`gc`](#k0maru-gc) | 4 级会话长轨迹上下文垃圾回收（Context GC，TRR $\ge 85\%$） | `[FILE]`, `--threshold`, `--hot-window`, `--json` | `k0maru gc session.json --threshold 20` |
| [`doctor`](#k0maru-doctor) | 全面体检二进制环境、知识库、缓存索引与客户端配置健康度 | `--vault`, `--home`, `--json` | `k0maru doctor` |
| [`ui`](#k0maru-ui) | 启动本地极客暗黑可视化调试控制台 | `--port`, `--open`, `--embed-model` | `k0maru ui --port 3721 --open` |
| [`mcp`](#k0maru-mcp) | 启动基于标准 stdio 的 FastMCP 服务端（全量或 Sidecar 模式） | `--vault`, `--sidecar`, `--embed-model` | `k0maru mcp --sidecar` |

---

## 📖 全量子命令详尽参考

### `k0maru init` / `bootstrap`

从任意 Git 仓库一键初始化零配置代码库 LLM-Wiki 与 Living ADR 架构哨兵（默认落盘至 `<repo>/.k0maru/wiki`）。支持一并安装智能体客户端配置与生命周期钩子。

```bash
k0maru init [OPTIONS]
# 或使用可视别名
k0maru bootstrap [OPTIONS]
```

#### 参数说明
- `-r, --repo <PATH>`：目标 Git 仓库路径（默认当前工作目录）；
- `-v, --vault <PATH>`：自定义知识库根目录（默认 `<repo>/.k0maru/wiki`）；
- `-p, --project <NAME>`：覆盖项目名称或 Slug；
- `-i, --install`：自动为各客户端写入 FastMCP 配置并安装前置生命周期钩子（`.claude/settings.local.json` SessionStart 钩子与 Git pre-commit 哨兵钩子）；
- `--home <PATH>`：覆盖用户主目录（用于沙箱测试）；
- `--json`：输出结构化 JSON 安装报告。

#### 调用示例
```bash
# 一键完成仓库知识库初始化与智能体钩子挂载
k0maru init --repo . --install
```

---

### `k0maru loadout`

在新会话开辟前提取目标项目的愿景与核心架构约束（L3），严格控制在 `< 300 Token`，防止上下文注意力漂移与架构幻觉。

```bash
k0maru loadout [QUERY] [OPTIONS]
```

#### 参数说明
- `[QUERY]`：目标项目名称或检索关键词（例如 `payment-gateway`）。若省略，自动探测当前工作目录对应的最近活跃项目；
- `-v, --vault <PATH>`：指定目标 Markdown 知识库根目录；
- `-d, --diff`：增量检查 Git 工作区与暂存区代码变动，根据涉及的模块动态激活相关的 ADR 架构规则；
- `-b, --branch <BRANCH>`：指定对比基准分支（例如 `main`、`dev`）；
- `-c, --copy`：将格式化后的 Markdown 内容直接复制到系统剪贴板（兼容 macOS、Linux 与 Windows）；
- `-l, --list`：列出当前知识库内识别到的所有活跃项目列表；
- `--json`：输出结构化 JSON 格式数据；
- `--json-stream`：以单行 NDJSON 流式输出。

#### `--json` 输出结构示例
```json
{
  "project_name": "payment-gateway",
  "vault_path": "/workspace/my-project/.k0maru/wiki",
  "token_count": 218,
  "markdown": "# Context Loadout: Payment Gateway Core\n- Stack: Go 1.22, Redis 7...",
  "l3_invariants": [
    "ADR-001: 支付回调幂等性防重规约"
  ],
  "recent_logs": []
}
```

---

### `k0maru guard`

Living ADR 架构铁律哨兵，在 $<10\text{ ms}$ 内审计代码变动是否遵守架构不变量。

#### `k0maru guard check`

```bash
k0maru guard check [OPTIONS] [FILES...]
```

- `-d, --diff`：审计当前 Git 工作区与暂存区改动；
- `--staged`：仅审计 Git 暂存区改动（`git diff --cached`，Git pre-commit 钩子默认使用）；
- `-p, --project <PROJECT>`：限定特定项目上下文；
- `-b, --branch <BRANCH>`：指定用于对比的基准分支（如 `main`）；
- `--claude-stop-hook`：Claude Code `Stop` 钩子模式：静默审计工作区，若通过则完全静默退出（退出码 `0`），若违规则向 `stderr` 打印违规原因并返回退出码 `2` 阻断智能体交付；
- `--strict`：严格模式，将建议性警告（`warn`）提升为阻断性失败（`deny`）；
- `--json`：输出结构化 JSON 审计报告；
- `[FILES...]`：直接检查指定文件列表，而非检查 diff。

#### `k0maru guard install`

```bash
k0maru guard install [OPTIONS]
```

- `--hook <TYPE>`：安装钩子类型（默认：`pre-commit`）；
- `-p, --path <REPO_DIR>`：目标 Git 仓库目录（默认当前目录）；
- `-v, --vault <PATH>`：绑定的 Markdown 知识库路径。

#### 调用示例
```bash
# 提交前检查代码改动
k0maru guard check --diff

# 作为 Claude Code 停止阻断钩子运行
k0maru guard check --diff --claude-stop-hook
```

---

### `k0maru flush`

遵循知识库既有规约，将关键决策或研发心得持久化结晶为 Markdown 卡片，自动生成 YAML Frontmatter 与 `[[WikiLinks]]`。

```bash
k0maru flush [OPTIONS]
```

#### 参数说明
- `-v, --vault <PATH>`：指定知识库根目录；
- `-t, --title <TITLE>`：必填，沉淀笔记的标题；
- `-s, --summary <TEXT>`：可选，一句话结论摘要；
- `-c, --content <TEXT>`：可选，Markdown 正文（亦支持通过标准输入管道传入）；
- `--category <CAT>`：分类名称（`decision`、`log`、`concept` [别名: `card`]、`skill`、`generic`，默认：`log`）；
- `--tags <TAGS>`：逗号分隔的标签数组；
- `--related <TITLES>`：逗号分隔的关联笔记标题（自动编织为 `[[WikiLinks]]`）；
- `--supersedes <NOTES>`：逗号分隔的被废弃笔记或旧 ADR 列表。工具将自动将旧笔记标记为 `status: superseded`，彻底化解大模型“信念固化”；
- `--dry-run`：仅在控制台预览生成路径与内容，不实际落盘；
- `--json`：输出结构化写入结果。

#### 调用示例
```bash
k0maru flush \
  --title "ADR-012: 分布式锁 Redlock 架构升级" \
  --category decision \
  --summary "从单实例 Redis 互斥迁移至 Redlock 集群以防脑裂" \
  --supersedes "ADR-001-Redis-Mutex" \
  --tags "redis,distributed-lock"
```

---

### `k0maru search`

多路混合检索知识库内容（BM25 词法 + ONNX 向量 + 1-Hop 图谱双向拓扑加权）。

**自愈式自动同步（Auto-Sync）机制**：在执行检索前，`k0maru search` 会主动对有变动的 Markdown 文件执行轻量增量扫描（`IncrementalScanner`），实现 `cache.sqlite` 自动修补与即时感知，新落盘的笔记无需手动 sync 即可秒级被检索召回。

```bash
k0maru search <QUERY> [OPTIONS]
```

#### 参数说明
- `<QUERY>`：搜索查询词、自然语言问题，或传入 `-` 从标准输入读取；
- `-v, --vault <PATH>`：指定知识库根目录；
- `-m, --mode <MODE>`：检索模式：`hybrid`（默认混合检索）、`bm25`（纯全文检索）、`vector`（纯语义向量检索）；
- `-l, --limit <N>`：返回结果数量上限（默认：`5`）；
- `--embed-model <MODEL>`：ONNX 嵌入模型（默认 `all-minilm-l6-v2`，亦可使用 `bge-small-zh-v1.5` 等）；
- `--json`：输出机器可读的 JSON 命中数组；
- `--json-stream`：以单行 NDJSON 流式输出。

---

### `k0maru install`

一键为各 AI 编码客户端配置注入 FastMCP 协议条目，采用非破坏性 JSON 合并技术。

```bash
k0maru install [OPTIONS]
```

#### 参数说明
- `-v, --vault <PATH>`：指定待挂载的目标知识库路径；
- `-t, --target <CLIENT>`：指定要配置的客户端：
  - `all`（默认，自动探测并配置所有安装的客户端）
  - `claude` (Claude Code)
  - `cursor` (Cursor IDE)
  - `gemini` (Gemini CLI)
  - `windsurf` (Windsurf)
  - `cline` (Cline / Roo Code)
  - `hermes` (Nous Research Hermes Agent)
  - `openclaw` (OpenClaw Agent)
  - `omp` (Oh My Pi — 自动配置 `--sidecar` 伴生模式)
- `--hooks`：一并安装主动式生命周期钩子（`.claude/settings.local.json` SessionStart 注入与 Git pre-commit 守卫）；
- `--home <PATH>`：覆盖用户主目录（用于沙箱测试）；
- `--dry-run`：预览配置变更差异，不执行写盘；
- `--json`：输出安装结果 JSON 报表。

---

### `k0maru pack`

导出、导入与透视便携式 **Memory Pack**（`.pack.json`）。Memory Pack 抹平了不同工程的目录拓扑差异，实现知识沉淀与 Living ADR 规则跨项目、跨智能体无缝流动。

```bash
k0maru pack <COMMAND> [OPTIONS]
```

#### 子命令

#### 1. `k0maru pack export`
扫描源知识库并打包笔记与 Living ADR 规则，自动剥离 `[[WikiLinks]]` 中的层级目录前缀。

```bash
k0maru pack export --name <NAME> [OPTIONS]
```
- `--name <NAME>`：Memory Pack 包名（必填）；
- `-v, --vault <PATH>`：源知识库路径；
- `--version <SEMVER>`：版本号（默认：`0.1.0`）；
- `--description <DESC>`：描述信息；
- `--tag <TAG>`：仅导出匹配特定标签的卡片（如 `topic/redis`）；
- `--query <QUERY>`：仅导出匹配检索词的卡片；
- `--files <FILE...>`：显式打包的文件列表；
- `-o, --output <FILE>`：输出文件路径；
- `--json`：向 stdout 打印原始 JSON。

#### 2. `k0maru pack import`
将 Memory Pack 导入任意 LLM-Wiki，FlushEngine 自动适配目标库的命名风格与目录分类规约。

```bash
k0maru pack import <PACK_FILE> [OPTIONS]
```
- `<PACK_FILE>`：`.pack.json` 文件路径（必填）；
- `-v, --vault <PATH>`：目标知识库路径；
- `--dry-run`：仅预览导入效果与目标文件路径，不落盘；
- `--json`：以 JSON 格式输出导入结果。

#### 3. `k0maru pack inspect`
在不修改磁盘的前提下，透视 Memory Pack 的元数据、笔记数量及内含的 Living ADR 规则清单。

```bash
k0maru pack inspect <PACK_FILE> [OPTIONS]
```
- `<PACK_FILE>`：`.pack.json` 文件路径（必填）；
- `-v, --vault <PATH>`：目标知识库路径；
- `--json`：输出结构化 JSON 报告。

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
- `--embed-model <MODEL>`：指定向量计算模型；
- `--json`：输出同步统计信息 JSON。

---

### `k0maru offload`

标准 Unix 管道过滤器。当终端日志超过阈值时进行截断并输出符号化 Mermaid 状态机图（实现 99.4% 的 Token 压缩）。

```bash
<COMMAND> 2>&1 | k0maru offload [OPTIONS]
```

#### 参数说明
- `-t, --threshold <N>`：触发日志截断的行数阈值（默认：`50` 行）；
- `--task-id <ID>`：可选的任务标识符前缀；
- `-r, --refs-dir <PATH>`：指定离线日志切片的持久化目录（默认：`.scratch/refs` 或 `<vault>/.k0maru/refs`）；
- `--json-stream`：以单行 NDJSON 流式输出。

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
- `--tags <TAGS>`：逗号分隔的标签列表；
- `--related <TITLES>`：逗号分隔的关联笔记列表；
- `-r, --refs-dir <PATH>`：离线日志切片存储目录；
- `--target <TARGET>`：目标格式（`default`、`hermes` 或 `omp`）；
- `--export-hermes`：一键将技能直接导出到 `~/.hermes/skills/`；
- `--export-omp`：一键将技能直接导出到 `~/.omp/agent/skills/<slug>/SKILL.md`；
- `--dry-run`：仅预览提炼结果，不写盘；
- `--json`：输出结构化 JSON。

---

### `k0maru gc`

执行多轮长会话上下文垃圾回收（Context GC），基于 4 级精简协议在保留活跃认知热窗口的前提下实现 $\ge 85\%$ 的 Token 压缩率。

```bash
k0maru gc [FILE] [OPTIONS]
```

#### 参数说明
- `[FILE]`：多轮对话会话 JSON 文件路径（省略则从 stdin 读取）；
- `-v, --vault <PATH>`：知识库路径；
- `-t, --threshold <N>`：折叠冗长工具输出的行数阈值（默认：`20` 行）；
- `-k, --hot-window <N>`：保持原样不折叠的最新活跃轮次数（默认：`5` 轮）；
- `--context-window <N>`：用于 Smart-Zone 评估的上下文总 Token 上限（默认：`128000`）；
- `--json`：输出精简后的会话 JSON；
- `--json-stream`：以单行 NDJSON 流式输出。

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
- `--open`：启动后自动唤起系统默认浏览器；
- `--embed-model <MODEL>`：指定向量模型。

---

### `k0maru mcp`

以 FastMCP Stdio 模式启动服务，供智能体客户端接入。

```bash
k0maru mcp [OPTIONS]
```

#### 参数说明
- `-v, --vault <PATH>`：指定挂载的知识库路径；
- `--embed-model <MODEL>`：指定语义检索向量模型；
- `--sidecar`：启用专为 `omp`（Oh My Pi）设计的正交伴生模式，仅暴露 5 项核心记忆与规则工具，屏蔽日志切片与 GC 工具。

---

## 🚦 环境变量与退出码

### 环境变量
- `K0MARU_VAULT`：全局指定知识库根目录路径（当省略 `--vault` 时生效）；
- `CLAUDE_PROJECT_DIR`：Claude Code 工作区目录，K0maru 优先识别 `<dir>/.k0maru/wiki`；
- `HERMES_HOME`：覆盖 Hermes Agent 默认配置发现目录；
- `OPENCLAW_HOME`：覆盖 OpenClaw 默认配置发现目录。

### 进程退出码 (Exit Codes)
- `0`：执行成功（Success）；
- `1`：常规执行失败或架构铁律审计违规（`deny`）；
- `2`：钩子阻断退出（`--claude-stop-hook` 发现违规时返回，向 Claude Code 发送明确阻断信号）；
- `130`：开发者通过 `Ctrl+C` 主动中断退出（SIGINT）。
