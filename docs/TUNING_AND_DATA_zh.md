# 调优指南与数据来源说明

本文档详细说明 `K0maru-Agent-Memory` 的**数据来源透明度、隐私保护基线以及面向个人开发者的专属调优指南**。

---

## 🔒 1. 数据来源透明度与隐私铁律

K0maru 严格遵循**「洁净室与零遥测原则（Cleanroom & Zero-Telemetry Principle）」**：

1. **绝对隐私与零数据回传**：
   - K0maru **绝不收集、不缓存、不上传、不回传**用户的任何 Markdown 笔记、代码片段、终端日志或搜索 Query；
   - 所有的 Markdown 解析、SQLite FTS5 倒排索引、ONNX 向量计算均 **100% 在你本地机器物理运行**；
   - 日常运行不开启任何监听网络端口，完全基于标准输入输出流（stdio）进行 Unix 管道通信。

2. **评测基准与蒸馏数据来源透明度**：
   项目公开的所有基准测试数据（上下文压缩率 TRR、冷启动耗时、内存占用曲线）以及故障提炼规则，均来源于**公开、合规且可复现的公域开源语料**：
   - **编译器与构建日志**：从 GitHub 公开开源仓库的 Rust (`cargo build`)、TypeScript (`tsc`)、Go 构建流水线中抓取；
   - **测试崩溃调用栈**：来源于 [pytest](https://github.com/pytest-dev/pytest) 与 [Jest](https://github.com/jestjs/jest) 的公开测试失败输出；
   - **智能体真实轨迹**：提取自公认的开源基准 [SWE-bench](https://www.swebench.com/) 及公开 GitHub Actions CI 失败日志；
   - **信息检索通用基准**：对齐国际通用检索评测集 [BEIR](https://github.com/beir-cellar/beir)（MS MARCO、SciFact、NFCorpus）与代码检索评测集 [CoIR](https://github.com/project-miracle/coir)；
   - **公域数字花园与技术文档**：来源于各类开源官方手册（如 [Rust 官方手册](https://github.com/rust-lang/book)、MDN Web Docs）以及公域 Quartz / Obsidian 开源知识库。

> **铁律准则**：绝无任何维护者个人或用户的私有笔记、私有项目代码参与模型的训练、量化或评测。

---

## 🛠️ 2. 个人开发者专属定制与调优指南

不同开发者有着不同的思维习惯、知识库排版和硬件配置。你可以通过以下几个核心维度，将 K0maru 调整到最适合你的工作流：

### 2.1 知识库结构与分类路由定制

K0maru 内置自适应规约嗅探器（`ConventionSniffer`），能自动识别扁平或多层知识库。如果你希望显式指定分类文件夹，只需在知识库根目录创建 `.k0maru/rules.md`（或在已有的 `AGENTS.md` / `CLAUDE.md` / `CONVENTIONS.md` 中声明）：

```markdown
# K0maru 知识库路由规约

- decisions: docs/adr
- logs: 01_AI_Logs
- concepts: 20_Cards
- skills: playbooks
```

| 规则键名 | 兼容别名 | 默认自动匹配逻辑 |
| :--- | :--- | :--- |
| `decisions` | `decision`, `adr` | 优先匹配现有的 `decisions/`, `adr/`, 无则落入根目录 |
| `logs` | `log`, `journal`, `daily`, `records` | 优先匹配现有的 `logs/`, `journal/`, 无则落入根目录 |
| `concepts` | `concept`, `cards`, `evergreen` | 优先匹配现有的 `concepts/`, `cards/`, 无则落入根目录 |
| `skills` | `skill`, `playbooks`, `recipes`, `troubleshooting` | 优先匹配现有的 `skills/`, `playbooks/`, 无则落入根目录 |

#### 自定义笔记模版
在知识库中创建 `templates/`、`_templates/` 或 `.obsidian/templates/` 目录并放置 `.md` 模版，K0maru 会在沉淀结晶时自动替换变量：
- `{{title}}`：笔记标题
- `{{summary}}`：核心摘要或结论
- `{{content}}`：笔记正文内容
- `{{date}}`：当前日期（YYYY-MM-DD）
- `{{tags}}`：Frontmatter 标签列表

---

### 2.2 检索模式与混合排序调优

通过 `k0maru search <query> --mode <MODE>` 可以针对不同检索意图灵活切换模式：

```bash
# 1. 混合检索（默认）：融合 BM25 词法 + ONNX 向量稠密 + 图谱加权
k0maru search "sqlite lock timeout" --mode hybrid --limit 5

# 2. BM25 词法模式：纯文本倒排索引（极速、零 ONNX 内存开销）
k0maru search "fn handle_call_tool" --mode bm25 --limit 10

# 3. 语义向量模式：纯向量距离召回（跨语言、概念联想）
k0maru search "如何处理智能体跨会话长期记忆丢失" --mode vector --limit 5
```

#### RRF（倒数排名融合）工作原理：
$$\text{RRF}(d) = \frac{0.5}{60 + \text{rank}_{\text{BM25}}(d)} + \frac{0.5}{60 + \text{rank}_{\text{Vector}}(d)} + \text{GraphBoost}(d)$$

- **何时使用 BM25 模式**：检索精准的函数名、错误代号（如 `E0308`）、特定配置文件名或 CLI 参数时，BM25 响应速度 **<1ms**，且占用 **0 MB** 额外内存；
- **何时使用 Vector 模式**：当你忘记具体用词、只记得大致概念或使用自然语言盘问时；
- **图谱加权 (+0.05 Boost)**：当候选笔记之间存在相互 `[[WikiLinks]]` 双向引用时，系统会自动给予加权，将核心枢纽（MOC）笔记优先顶出。

---

### 2.3 硬件与资源开销定制（极简 vs 稠密）

#### 极低资源模式（轻量 VPS、老旧电脑、CI 容器）
如果你的运行环境内存受限（如 <1GB RAM）：
- 直接使用 `--mode bm25`，完全跳过向量模型加载，内存占用 **<12MB**；
- 也可以通过关闭默认特性自行编译：
  ```bash
  cargo build --release --no-default-features
  ```
  关闭 `fastembed` 后二进制体积进一步缩减至 **~2.8MB**，极致纯净。

#### 标准本地向量增强模式
- 默认嵌入模型为 `all-MiniLM-L6-v2`（384 维，权重体积约 80MB）；
- 模型文件自动缓存在 `~/.k0maru/models/`，全离线可用；
- 可以预先增量生成所有向量索引：
  ```bash
  k0maru sync --vector
  ```
  之后的所有向量检索均在毫秒级内完成。

---

### 2.4 日志卸载与上下文阈值调优

日志卸载器（`k0maru offload`）拦截终端冗长输出，防止上下文撑爆：

```bash
# 默认阈值：50 行（超过 50 行自动符号化）
cargo test 2>&1 | k0maru offload

# 针对短上下文小模型（如 16k/32k 窗口）：降低阈值，更激进卸载
cargo test 2>&1 | k0maru offload --threshold 25

# 针对 100 万大上下文模型：放宽阈值，保留更多即时输出
cargo test 2>&1 | k0maru offload --threshold 100
```

当发生卸载时，可通过 Node ID 精准透视原始切片：
```bash
k0maru inspect <node-id>
```

---

### 2.5 本地可视化控制台偏好

一键启动本地控制台：
```bash
k0maru ui --vault ~/my-vault --port 3721 --open
```

- **中英双语切换**：页面右上角提供语言切换按钮，支持在中文与英文间无缝切换，选择会自动保存在浏览器的 `localStorage` 中；
- **端口冲突**：如果默认 3721 端口被占用，使用 `--port <端口>` 自由绑定。
