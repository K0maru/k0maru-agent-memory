# CONTEXT.md — K0maru-Agent-Memory Domain Model & Glossary

本文件定义了 `K0maru-Agent-Memory` 项目的领域词汇表（Domain Glossary）、架构不变量（Architectural Invariants）与设计决策。所有 Agent 在编写 SPEC、工单、测试与实现代码时，必须统一使用本词汇表中的术语。

---

## 📖 核心领域词汇表 (Domain Glossary)

| 术语 | 英文标识 | 概念定义与职责边界 |
| :--- | :--- | :--- |
| **知识库/仓库** | `Vault` | 用户的本地 Markdown 笔记根目录（如 Obsidian Vault、Karpathy LLM-Wiki、标准 Markdown 目录）。系统遵循 "Bring Your Own Wiki"，Markdown 文件是唯一的真理源泉（Single Source of Truth）。 |
| **文档实体** | `Document` | 知识库中的最小文件实体。不可变对象（`frozen=True`），包含相对路径、哈希值、最后修改时间（`mtime`）、YAML 元数据（`Frontmatter`）、正文（`body`）与语义分层。 |
| **语义分层** | `HierarchyLevel` | 文档按生命周期与治理成熟度的四级划分：<br/>• `L0 (Daily)`：临时日记、流水账、瞬态上下文<br/>• `L1 (Resource)`：静态参考手册、CLI 速查、环境配置<br/>• `L2 (Log)`：技术方案、AI 分析日志、评测记录<br/>• `L3 (Evergreen)`：常青卡片、架构规约、不可变核心原则 |
| **双链引用** | `WikiLink` | 文档之间的超链接语法，分为裸双链 `[[Target]]` 与别名双链 `[[Target|Alias]]`。代表知识图谱中的一条有向边。严禁在双链外包裹行内反引号代码块。 |
| **拓扑邻接图** | `AdjacencyGraph` | 基于双链构建的有向图谱。支持 1-hop（直接引用/被引用）与 2-hop（二阶关联）邻域检索。 |
| **一次性缓存** | `DisposableCache` | 单文件 SQLite 数据库（`cache.sqlite`），存储解析后的元数据、双链邻接表及 FTS5 全文索引。其核心特性为“随时可删、秒级自愈”，不承载持久状态。 |
| **增量扫描器** | `IncrementalScanner`| 基于 `mtime + hash` 状态机的文件变动感知器。负责比对磁盘与缓存，精准计算 `added` / `modified` / `deleted` 集合并批量同步。 |
| **读档背包** | `Loadout` | 面向 AI 代理的极简状态包（Token 预算严格控制在 <300 Token）。由项目规则、核心常青卡片与高频指令动态装配而成，供 Agent 开局快速读档。 |
| **上下文卸载** | `Offload` | 长日志与复杂运行状态的截断与符号化卸载机制。将 >50 行的原始输出落盘至临时目录，并在对话中替换为带 `node_id` 的 Mermaid 状态机图谱。 |
| **MCP 服务** | `FastMCPServer` | 遵循 Model Context Protocol 的 stdio 进程服务，为 Cursor、Antigravity、Claude Code 等外部 Agent 提供标准内存检索与写入工具。 |

---

## 🏛️ 系统架构不变量 (Invariants)

1. **真理在文件，性能在缓存 (Truth in Files, Performance in Cache)**：
   - 存储的第一真理源泉永远是文件系统中的 `.md` 文件；
   - `cache.sqlite` 仅作为瞬态加速层，被删除后必须能在 1 秒内完全通过本地 Markdown 重新生成。
2. **零封闭格式 (Zero Proprietary Formats)**：
   - 绝不引入私有二进制或不兼容的元数据格式；
   - 兼容标准 CommonMark、GitHub Flavored Markdown 及 Obsidian YAML Frontmatter。
3. **极简低损 (Low Overhead & No Vibe Coding)**：
   - 装配到 Prompt 中的背包体积严格限制在 300 Token 以内；
   - 检索链路必须在本地纯 CPU 上毫秒级响应，禁止未经请求在后台常驻无谓的重型守护进程。
