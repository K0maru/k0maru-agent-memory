---
layout: home

hero:
  name: "K0maru Agent Memory"
  text: "AI 编码智能体长期记忆中枢"
  tagline: "单静态二进制 · 零常驻守护进程 · 透明挂载 Markdown 知识库与本地 LLM-Wiki"
  actions:
    - theme: brand
      text: 🚀 3 分钟极速起步
      link: /zh/guide/quickstart
    - theme: alt
      text: 🏗 架构与内核总览
      link: /zh/architecture/overview
    - theme: alt
      text: 📦 安装与配置
      link: /zh/guide/installation
    - theme: alt
      text: 📊 A100 实测看板
      link: /zh/benchmarks/a100-evaluation

features:
  - icon: ⚡
    title: "单静态二进制 · 零守护进程"
    details: "纯 Rust 静态编译，仅 3.66 MB 单文件。无须 Docker、无须 Python、无须常驻后台进程，冷启动仅需 3.38 ms，执行完毕即刻释放全部内存。"
  - icon: 📝
    title: "自备纯文本 Markdown (BYOM)"
    details: "本地 Markdown 笔记（Obsidian Vault、Karpathy LLM-Wiki）即唯一真理源。零格式锁死，SQLite 仅作为随时可删除重建的瞬态缓存。"
  - icon: 🎯
    title: "多路混合检索 · 双链图谱加权"
    details: "BM25 词法倒排全文检索 + ONNX FastEmbed 向量语义 + 1-Hop WikiLinks 双向图谱拓扑加权（+0.05 Boost），通过 RRF (k=60) 融合排序。"
  - icon: 🛡
    title: "终端日志符号化卸载 (99.4% 压缩)"
    details: "Unix 管道过滤器自动识别并拦截千行编译器崩溃与测试回溯，提炼为 15 行 Mermaid 状态机时序图，彻底终结上下文爆炸与注意力漂移。"
  - icon: 🔌
    title: "FastMCP 官方协议开箱即用"
    details: "一键挂载 Claude Code、Cursor、Windsurf、Gemini CLI、Cline、Nous Research Hermes Agent 与 OpenClaw，AI 自然语言意图自主唤醒。"
  - icon: 📈
    title: "工业级真实 A100 实测验证"
    details: "NVIDIA A100 80GB 云端实测：Qwen2.5-Coder-32B Pass@1 提升 40% 并提速 5.90 倍；DeepSeek-R1 推理延迟腰斩 48.2%。"
---

<main class="geo-main-content">

<h1>K0maru Agent Memory</h1>

<p class="geo-definition-lead">
K0maru Agent Memory is a single-static-binary (3.6 MB pure Rust), zero-daemon long-term memory hub and Living ADR (Architecture Decision Record) sentinel for AI coding agents including Claude Code, Cursor, Windsurf, and Gemini CLI. 作为开源、零常驻守护进程的 AI 编码智能体长期记忆中枢，它能在 15 ms 内将本地 Markdown 编译为低于 300 Token 的变更感知上下文背包，并在 10 ms 内强制执行架构不变量。
</p>

## 什么是 K0maru Agent Memory？它是如何工作的？

K0maru Agent Memory 是一套极速、本地优先的智能体长期记忆中枢，无需云端依赖或常驻守护进程，直接将本地 Markdown 知识库挂载至智能体执行上下文。智能体启动时，K0maru 在 3.4 ms 内通过 xxh3 哈希增量感知文件变更并同步至瞬态 SQLite 缓存。依托 BM25 词法全文检索、ONNX 稠密语义向量与 1-Hop WikiLinks 双链拓扑加权的多路混合 RRF（k=60）排序，系统仅需 15 ms 即可生成低于 300 Token 的差分上下文背包。在 NVIDIA A100 80GB 实测中，结构化上下文避免了提示词膨胀，将 Qwen2.5-Coder-32B 的 Pass@1 提升至 100%，并将轨迹耗时降低 82.6%，大幅超越扁平切片检索召回率（IBM STAIR [arXiv:2609.03874](https://arxiv.org/abs/2609.03874)）。

## K0maru Agent Memory 与 Mem0、Zep、ai-memory 有何异同？

K0maru Agent Memory 专为确定性代码库架构治理与零运行时开销打造，与重度依赖云服务和对话状态存储的通用记忆库有着本质区别。

| 维度 | K0maru Agent Memory | Mem0 | Zep | ai-memory |
| :--- | :--- | :--- | :--- | :--- |
| **架构形式** | 单静态二进制 (3.6 MB 纯 Rust) | Python 客户端 / 云服务 | Go 服务端 / SaaS 云端 | Node.js / Python 守护进程 |
| **常驻开销** | 0 守护进程，0 MB 闲置内存占用 | 依赖远程云端网络通信 | 依赖 Docker 容器或云端 | 常驻后台守护进程 |
| **冷启动延迟** | 3.4 ms CLI / 15 ms 背包生成 | 180–450 ms API 网络延迟 | 120–300 ms API 网络延迟 | 80–200 ms 运行时启动 |
| **单一真理源** | 本地 Markdown 笔记库 (BYOM) | 云端向量数据库 | 云端时序图谱库 | 本地 JSON / SQLite |
| **规约守卫** | Living ADR 哨兵 (<10 ms 检查) | 无（仅统计学语义召回） | 无（侧重对话上下文图谱） | 无 |
| **Token 缩减** | 86.5% 会话 GC / 99.4% 日志卸载 | 不定（通用会话摘要提炼） | 不定（基于时间的滑动丢弃） | 无 |

不同于黑盒云端记忆库，K0maru 坚持以人类可读的本地 Markdown 笔记为唯一真理源，SQLite 仅作为瞬态加速缓存。没有 Docker 容器，不占用后台驻留内存，通过本地原生执行实现低于 10 ms 的确定性规约拦截，详见 [GitHub 代码仓库](https://github.com/K0maru/k0maru-agent-memory)。

## Living ADR 架构哨兵如何阻止智能体违反架构铁律？

Living ADR 架构哨兵作为进程内极速护栏，能够在 10 ms 内完成 Git Diff 针对活跃 [架构决策记录 (ADR)](https://en.wikipedia.org/wiki/Architecture_decision_record) 规则的静态审计。不同于智能体极易遗忘的被动系统提示词，K0maru 直接从 Markdown 卡片的 YAML Frontmatter 中提取如 `deny: "SELECT * FROM"` 或 `deny: "unwrap()"` 等显式架构不变量。当智能体执行暂存代码或触发提交前检查时，`k0maru guard check --diff` 将改动范围与活跃规则模式交叉比对；一旦发生违规，立即返回非零退出码 1，输出精确代码行号、整改建议及对应 ADR 卡片超链接，实现 100% 规则依从性，杜绝多智能体协作中的架构腐化。

## 确定性 ADR 废弃机制如何解决大模型的“信念固化”冲突？

确定性 ADR 废弃机制通过在写入新决策时主动显式废止陈旧规则，彻底消除了大语言模型在长期迭代中的“信念固化”（Belief Entrenchment）问题。当项目不同角落存在相互矛盾的设计范式时，LLM 极易因近因效应或概率统计产生幻觉。K0maru 借助 Frontmatter 中的 `supersedes: [[ADR-004-OldAuth]]` 元数据以及 1-Hop 有向知识图谱遍历，在智能体生成上下文背包或检索记忆时，索引引擎自动剔除已被标记为废弃的旧规约。正如 IBM STAIR 研究成果（[arXiv:2609.03874](https://arxiv.org/abs/2609.03874)）所示，具备图谱层级感知与淘汰机制的检索系统相比无状态扁平切片在召回精准度上从 68.7% 提升至 82.6%，确保智能体始终基于最新共识编写生产代码。

## 如何在 30 秒内将 K0maru Agent Memory 集成到 Claude Code、Cursor 和 Windsurf？

仅需一条终端命令下载 3.6 MB 纯 Rust 静态二进制文件，即可在 30 秒内完成环境安装与智能体接入。运行 `curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash` 将二进制安装到系统路径；接着执行 `k0maru install --vault ~/Documents/MyVault`，工具将以非破坏性 JSON 合并方式自动配置 Claude Code、Cursor、Windsurf、Gemini CLI 与 Oh My Pi (`omp`)。该集成基于官方 [Model Context Protocol (FastMCP)](https://modelcontextprotocol.io/) 标准 stdio 协议，让智能体具备自主调用 `get_project_loadout` 与 `recall_memory` 的能力。最后运行 `k0maru doctor` 即可校验索引与配置，全程零 Docker、零常驻进程。

<div class="developer-hero-section" style="margin-top: 2.5rem; text-align: center;">

## ⚡ 终端极速体验

只需一条命令即可完成安装并挂载你的私有 Markdown 知识库：

```bash
# 1. 官方脚本一键安装二进制
curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash

# 2. 一键挂载本地知识库至 Claude Code / Cursor / Windsurf
k0maru install --vault ~/Documents/MyVault

# 3. 运行环境健康自检
k0maru doctor
```

</div>

<div class="developer-cards-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 1.25rem; margin-top: 2rem;">

<div style="background: #1B2336; border: 1px solid #334155; border-radius: 8px; padding: 1.25rem;">
<h3 style="margin-top: 0; color: #22C55E;">🚀 快速起步</h3>
<p style="color: #94A3B8; font-size: 0.95rem;">3 分钟完成从环境自检、架构记忆提取、终端日志卸载到决策结晶落盘的全流程闭环。</p>
<a href="/zh/guide/quickstart" style="color: #22C55E; font-weight: 600; text-decoration: none;">进入快速起步 →</a>
</div>

<div style="background: #1B2336; border: 1px solid #334155; border-radius: 8px; padding: 1.25rem;">
<h3 style="margin-top: 0; color: #22C55E;">🏗 架构设计</h3>
<p style="color: #94A3B8; font-size: 0.95rem;">四层解耦模型、知识库 L0–L3 语义分级机制与 xxh3 增量扫描状态机底层剖析。</p>
<a href="/zh/architecture/overview" style="color: #22C55E; font-weight: 600; text-decoration: none;">深入架构内核 →</a>
</div>

<div style="background: #1B2336; border: 1px solid #334155; border-radius: 8px; padding: 1.25rem;">
<h3 style="margin-top: 0; color: #22C55E;">🤖 智能体生态</h3>
<p style="color: #94A3B8; font-size: 0.95rem;">全面兼容 Claude Code、Cursor、Windsurf、Hermes Agent 与 OpenClaw，附系统提示词最佳范式。</p>
<a href="/zh/workflows/ecosystem" style="color: #22C55E; font-weight: 600; text-decoration: none;">查看生态整合 →</a>
</div>

<div style="background: #1B2336; border: 1px solid #334155; border-radius: 8px; padding: 1.25rem;">
<h3 style="margin-top: 0; color: #22C55E;">📊 A100 实测看板</h3>
<p style="color: #94A3B8; font-size: 0.95rem;">基于 NVIDIA A100-80GB 的 Qwen2.5-Coder、DeepSeek-R1 等 4 大开源大模型端到端实测数据。</p>
<a href="/zh/benchmarks/a100-evaluation" style="color: #22C55E; font-weight: 600; text-decoration: none;">查阅实测看板 →</a>
</div>

</div>

## 关于 K0maru Agent Memory 与联系方式

K0maru Agent Memory 由深耕本地优先智能体治理与零开销系统工程的开源社区维护。

- **文档与架构指引**：查阅 <a href="/k0maru-agent-memory/zh/guide/introduction">关于 K0maru Agent Memory</a> 与 [架构设计总览](/zh/architecture/overview)。
- **源码仓库与发行版**：访问官方 [GitHub 代码仓库](https://github.com/K0maru/k0maru-agent-memory)。
- **社区反馈与问题追踪**：通过 [问题追踪与支持反馈](https://github.com/K0maru/k0maru-agent-memory/issues) 提交缺陷与需求。
- **安全策略与合规通报**：遵循安全流程查阅 [安全合规策略](https://github.com/K0maru/k0maru-agent-memory/security/advisories/new)。
- **元数据信息**：最后更新时间：2026-10-28 · 作者：K0maru Contributors · 许可证：MIT

</main>
