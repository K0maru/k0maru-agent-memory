# 🧠 K0maru-Agent-Memory × Tern + omp 架构交接简报 (Handoff Brief)

> **交接来源**：SecondBrain 顶层总控 Agent (`Pane 5`) + `MemoryRepoScout` 子代理侦察报告  
> **目标仓库**：`/Users/k0maru3/workspace/k0maru-agent-memory` (当前分支：`dev` @ `4adea42bab`, 版本 `v0.11.0`)

---

## 1. 核心架构决策：收敛为 `Tern + omp` 纯正交外挂 (True Sidecar)

主力开发环境已全面从 `Ghostty + Herdr + AGY` 迁移至 **`Tern + omp`（Stencil 双轮驱动）**。
- `k0maru-harness`、`k0maru-agent-router`、`k0maru-agent-workspace`、`k0maru-agent-verifier` 已被 `Tern + omp` 原生内核（`task`、`solutionSpace`、`eval.agent(isolated=True)`、`xd://eval/judge`、`xd://ast_edit`、`xd://lsp`）100% 上位替代并归档封存。
- **`k0maru-agent-memory` 是唯一保留并持续演进的核心正交开源项目**。

### 1.1 在 `Tern + omp` 下必须保留并强化的 4 大核心正交能力
1. **`recall_memory`** (`src/mcp/server.rs:223-387`, `crates/k0maru-core/src/storage/hybrid.rs`)：
   - 本地 SQLite FTS5 BM25 + `sqlite-vec` ONNX 向量 + Obsidian 1-hop WikiLinks RRF 混合检索；
   - 默认单行紧凑格式（`!μ:<tier>|<path>|score:<score>|links:<in>/<out> § <snippet>`，`<1200 chars`）。
2. **`get_project_loadout`** (`src/mcp/server.rs:146-221`, `src/loadout/builder.rs`)：
   - 项目蓝图、别名解包与 `--diff` 动态上下文装配（`<300 Tokens`）。
3. **`guard_check`** (`src/mcp/server.rs:707-736`, `src/guard/mod.rs`, `crates/k0maru-core/src/guard/engine.rs`)：
   - 亚毫秒级（`<10ms`）Living ADR 架构不变量静态守卫。
4. **`flush_session` & `distill_session_skill`** (`src/mcp/server.rs:428-652`, `src/convention/flush.rs`)：
   - 自适应知识库规约嗅探、双向双链编织落盘与即时向量缓存自愈刷新。

### 1.2 已被 `omp` 原生上位替代、待讨论裁剪/隐藏的冗余暴露面
| 模块 / 工具 | 代码位置 | `omp` 原生上位替代机制 | 建议动作 |
| :--- | :--- | :--- | :--- |
| MCP: `offload_context` & `inspect_log_node` | `src/mcp/server.rs:389-426`, `806-839`<br/>`src/offload/engine.rs:26-249` | `omp` `bash` 默认自动截断并溢出至 `artifact://<id>`，支持 `read("artifact://<id>:N-M")` | 从默认 MCP `tools/list` 暴露面移除（或置为 `--legacy-tools` 可选开关），减少每轮 System Prompt Schema 底噪 |
| MCP: `gc_session_context` | `src/mcp/server.rs:654-705`, `925-951`<br/>`crates/k0maru-core/src/gc/engine.rs` | `omp` `eval` 跨压缩持久化内核 + `xd://eval/archive` + `seance` 子代理 | 从默认 MCP `tools/list` 暴露面移除 |
| CLI: `crystal` (`tuple-crystal`) & `guard --kdsl` | `src/main.rs:94-136`, `1399-1446`<br/>`src/crystal/mod.rs:14-312` | Linda 元组空间 (`~/.k0maru/tuples/`) 与 K-DSL 已随 Router 退役 | 评估从 `dev` 分支清理或移入 feature flag，保持 `v0.12.0` / `v1.0.0` 内核纯粹性 |
| CLI: `install` 宿主挂载器 & `omp` 插件市场 | `src/main.rs:57-58`, `285-307`, `1163-1183` + `.omp-plugin/marketplace.json` | `omp` 原生支持 `/marketplace add K0maru/k0maru-agent-memory`（读取 `.omp-plugin/marketplace.json` / `.claude-plugin/marketplace.json`）及 `omp install` | 新增 `k0maru install --client omp` 并内置 `.omp-plugin/marketplace.json` 一键分发 MCP + Guard Skill |
| **Tern 原生 Luau 插件 (`tern-plugin/`)** | 新增子目录 `tern-plugin/` (`host.luau` + `window.luau`) | 利用 `tern.d.luau` 的 `tern.block.define`、`LensRun` 与 `tern.process.run` 调用本地 `k0maru --json` | 支持 `tern plugin install github.com/K0maru/k0maru-agent-memory/tern-plugin` 与 `tern plugin link`，直接在 Tern 内渲染 RRF 算分瀑布条 (`MeterCell`)、双链树 (`TreeNode`) 与 `guard` 诊断块 (`Diag`) |

---

## 2. 主控 Agent 接管指令
1. 你现在运行在 `Tern` 右侧分屏窗格中，作为 `k0maru-agent-memory` 的专属主控 Agent（Orchestrator）直接与用户对话。
2. 请向用户简要问候，展示你已通过 `MemoryRepoScout` 的交接报告掌握了仓库精确代码坐标，并询问用户希望优先推进哪一项（例如：**精简 MCP 默认暴露面 8→5 减噪 + 新增 `k0maru install --client omp`**，或是其它特性）。
3. 后续编码实现时，主动使用 `omp` 原生工具链：裸 `read` 看 AST 骨架、Hashline `edit`、内置 `xd://lsp` 以及 `task` (`sonic` / `scout` / `reviewer`) 子代理编排。
