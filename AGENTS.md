# AGENTS.md — K0maru-Agent-Memory Agent 工作流与工程规约

本文件定义了 AI Agent 在 `K0maru-Agent-Memory` 项目中协同开发的核心行为准则、技能配置与工作流契约。

---

## 🛠️ Agent skills

### Issue tracker

Issues and specs are tracked as local markdown files in `.scratch/k0maru-agent-memory/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Canonical triage roles (needs-triage, ready-for-agent, ready-for-human, etc.). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context repository layout using `CONTEXT.md` glossary. See `docs/agents/domain.md`.

---

## 🎯 核心工作流模式 (Matt Pocock Master-Worker Model)

本项目严格执行 Matt Pocock 的 Master-Worker 多代理工程范式，严禁随缘编码（No Vibe Coding）：

1. **主 Agent 职责 (Master / Orchestrator Agent)**：
   - 角色定位：总架构师与任务调度器。
   - 行为纪律：**主 Agent 严禁直接手写大段业务代码**。
   - 工作链路：
     1. **需求理解与盘问**：研读 `SPEC.md`、`README.md`、`CONTEXT.md` 与参考实现。若有任何上下文不理解或模糊之处，**必须主动向人类提问澄清，确认后方可开启**；
     2. **执行 `/to-spec`**：将讨论与上下文提炼为完整规格文档（落地至 `.scratch/k0maru-agent-memory/spec.md`）；
     3. **执行 `/to-tickets`**：将规格拆解为纵向穿透型工单（Tracer-bullet Tickets），声明明确的依赖阻断关系（`Blocked by`），按依赖顺序写入 `.scratch/k0maru-agent-memory/issues/<NN>-<slug>.md`；
     4. **工单确认**：向人类呈现 Ticket DAG，确认后进入实现阶段；
     5. **启动 Subagent 执行 `/implement`**：派发子代理或在干净上下文执行 `/implement`（使用 `/tdd` 编写测试、驱动代码实现、跑类型检查并由 `/code-review` 审查）。

2. **从 Agent 职责 (Worker / Implementor Agent)**：
   - 在独立的上下文窗口中只负责单一工单；
   - 严格遵循测试驱动开发（TDD）：先写失败测试，再写最小实现代码，最后重构；
   - 保证单测 100% 通过且 `cargo test`、`cargo clippy`、`cargo fmt` 零报错后提交。

---

## 📐 代码与工程质量基线

1. **包管理与运行环境**：
   - 统一使用 `cargo` 管理 Rust 依赖与二进制编译；
   - 目标 Rust 版本：`2021 Edition` (MSRV: 1.75+)。
2. **代码风格与静态检查**：
   - Formatter：`cargo fmt --check`；
   - Linter / 静态检查：`cargo clippy --all-targets -- -D warnings`；
   - 领域模型与不可变性：关键领域数据结构推导 `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`，不提供不必要的 `pub mut`。
3. **测试规范**：
   - 框架：Rust 原生 `cargo test` 配合 `tempfile::tempdir` 与 `assert_cmd`；
   - 独立性：单元测试严禁依赖或读写用户的真实 Vault（如 `SecondBrain`），必须使用基于临时目录的 Mock Vault 夹具；
   - 深度接缝测试：优先在最高层 CLI 与 MCP 接口接缝进行行为验证，减少内部脆弱的 Mock。

---

## 🌿 Git 分支管理与工单派发契约

本项目采用**「双主干 + 工单穿透型短周期分支」**模式，严格对齐 Master-Worker 模型：

1. **`main` 分支（稳定发布主干）**：
   - 绝对稳定，对外开源发版；
   - 严禁直接 push；仅接受来自 `dev` 的阶段性里程碑 PR 或紧急 hotfix；
   - 语义化发布打 Tag（如 `v0.1.0`、`v0.2.0`），触发自动构建分发。
2. **`dev` 分支（开发集成主干）**：
   - 日常开发与工单集成主干；所有特性合并的目标；
   - 必须随时保持全套测试全绿、编译无 warning。
3. **工作分支命名与生命周期**：
   - **特性工单**：`feat/<NN>-<slug>`（与 `.scratch/k0maru-agent-memory/issues/<NN>-<slug>.md` **1:1 强绑定**）；
   - **评测实验**：`bench/<slug>`；
   - **缺陷修复**：`fix/<slug>`；
   - **工作流**：从最新 `dev` 拉出分支 ➔ 独立上下文中 Worker 完成 TDD ➔ 通过 Review ➔ Squash & Merge 回 `dev` ➔ 删除工作分支。
4. **特性验收与发版确认纪律（严禁过急提 PR / 发版）**：
   - **严禁擅自向 main 提发布 PR、合并或打 Tag**：每次开发完一个新特性或完成工单后，**必须先停下来向人类汇报**开发成果、测试报告与本地实测指南；
   - **人类主动体验与验收机制**：新特性先停留在 `dev`（或特性分支）供人类开发者在本地环境进行实测验证、交互体验；
   - **人类明确确认后方可合入 main**：只有在人类亲自体验并明确指示（如“可以合入 main”、“提 PR 发版”、“确认发版”）后，方可启动向 `main` 合并、打 Release Tag 与推送发布流程。

