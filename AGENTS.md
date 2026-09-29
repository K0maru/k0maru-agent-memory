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
   - 保证单测 100% 通过且 `ruff`、`mypy` 零报错后提交。

---

## 📐 代码与工程质量基线

1. **包管理与运行环境**：
   - 统一使用 `uv` 管理依赖与虚拟环境；
   - 目标 Python 版本：`>=3.11`（优先使用本地 CPython 3.12）。
2. **代码风格与静态检查**：
   - Linter / Formatter：`ruff check --fix` 与 `ruff format`；
   - 类型系统：全量类型注解，`mypy` 检查无错误；
   - 领域模型：统一采用 `Pydantic v2`，关键领域对象配置 `model_config = ConfigDict(frozen=True)`。
3. **测试规范**：
   - 框架：`pytest` 与 `pytest-asyncio`；
   - 独立性：单元测试严禁依赖或读写用户的真实 Vault（如 `SecondBrain`），必须使用基于 `tmp_path` 的 Mock Vault 夹具；
   - 深度接缝测试：优先在最高层接口接缝进行行为验证，减少内部脆弱的 Mock。
