# Ticket 35: Chinese Documentation Suite (`docs/zh/`)

**Status**: closed
**Blocked by**: 34

## Goal
Author the comprehensive, world-class Chinese technical documentation suite under `docs/zh/` adhering strictly to standard GitHub Flavored Markdown and the **阮一峰《中文技术文档写作规范》** and **《中文文案排版指北》** (中英文盘古之白空格、全角中文标点、官方专有名词大小写、代码块语言显式标注、客观祈使文风).

## Tasks
1. `docs/zh/index.md`: Hero landing page with key value propositions, quick links, feature pills, and terminal preview.
2. `docs/zh/guide/introduction.md`:
   - 项目介绍与痛点分析（跨会话失忆、终端日志爆炸、重型微服务容器反模式）；
   - "Bring Your Own Markdown" (BYOM) 哲学与零锁死优势；
   - 核心竞品全景对比矩阵（Mem0, Letta, Chroma, SQLite 裸嵌向量）。
3. `docs/zh/guide/installation.md`:
   - 官方一键脚本（`curl | bash`）、Homebrew Tap、Cargo crates.io、预编译 GitHub Releases、源码编译构建；
   - 环境变量与系统依赖自查。
4. `docs/zh/guide/quickstart.md`:
   - 3 分钟极速闭环体验（`doctor` -> `loadout` -> `offload` -> `search` -> `flush`）；
   - 自愈检验与效果演示。
5. `docs/zh/architecture/overview.md`:
   - 四层解耦架构模型（协议表现层、领域处理层、检索融合层、物理存储层）；
   - 知识库四级层级划分（L3 永恒笔记, L2 决策日志, L1 外部资源, L0 瞬态日志）；
   - 记忆生命周期状态流转图（Mermaid）。
6. `docs/zh/architecture/key-implementations.md`:
   - xxh3 增量扫描状态机与 0-I/O 脏文件判定；
   - BM25 + ONNX 向量 + 1-Hop 图谱加权 RRF 混合检索算法与数学推导；
   - FastMCP Stdio 协议与自然语言意图对齐；
   - 符号化日志状态机离线提取与 SVG Mermaid 渲染；
   - 异构知识库拓扑嗅探器（ConventionSniffer）；
   - 经验原子结晶引擎（FlushEngine）防碰撞机制；
   - 故障排障痕迹智能提炼引擎（Trace-to-Skill）；
   - 内嵌式极客暗黑调试控制台（WebUI）。
7. `docs/zh/workflows/ecosystem.md`:
   - 一键智能体生态挂载（`k0maru install`）：Claude Code、Cursor、Windsurf、Gemini CLI、Cline / Roo Code、Hermes Agent、OpenClaw；
   - 手动配置与 JSON-RPC 调试指南；
   - `AGENTS.md` 系统提示词编写最佳实践。
8. `docs/zh/workflows/scenarios.md`:
   - 场景 A：企业级 LLM-Wiki 维护（支付网关与分布式锁不变量守卫）；
   - 场景 B：跨会话故障排查与自愈闭环；
   - 场景 C：动态技能提炼与团队协作。
9. `docs/zh/reference/cli.md`:
   - 详尽 CLI 命令参考（`loadout`, `search`, `offload`, `inspect`, `sync`, `flush`, `distill`, `install`, `doctor`, `ui`, `mcp`）包含全参数、环境变量、退出码与 JSON 输出示例。
10. `docs/zh/reference/mcp.md`:
    - FastMCP 6 大原生工具（`get_project_loadout`, `recall_memory`, `offload_context`, `inspect_log_node`, `flush_session`, `distill_session_skill`）的 JSON Schema 契约与输入输出示例。
11. `docs/zh/benchmarks/a100-evaluation.md`:
    - NVIDIA A100-SXM4-80GB 实测看板全解析（Qwen2.5-Coder-32B, DeepSeek-R1-32B, DeepSeek-Coder-V2-16B, Qwen3.8-27B）；
    - Token 压缩率（TRR）、首轮修复延迟、Pass@1 成功率、图表展示与评测复现指南。
