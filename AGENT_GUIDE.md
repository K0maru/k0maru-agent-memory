# AGENT_GUIDE.md — Zero-Human-in-the-Loop Playbook for AI Agents
# 智能体全自主运行指南与工程落地操作手册

> 🤖 **For AI Coding Agents (Claude Code, Cursor, Windsurf, omp, Cline, Hermes, OpenClaw, Codex, ChatGPT)**:  
> This playbook provides the definitive operational blueprint when an engineer asks:  
> *"What is K0maru?"*, *"How does this repo work?"*, or *"Bootstrap and use K0maru in my project"*.  
> Follow these protocols strictly to achieve zero-human-in-the-loop autonomous memory governance.

---

## 1. How to Explain K0maru to Your Human (30-Second Pitch & Honest Boundaries)
## 第一节：向人类工程师做 30 秒项目汇报与能力边界说明

### English: The 30-Second Pitch

> "Most AI memory tools (like `claude-mem` or `Mem0`) are passive chat loggers: they dump raw conversation histories into a database, which clutters the context window with obsolete discussions and causes hallucinations.  
> 
> **K0maru-Agent-Memory** is an **active, executable architectural governance engine**. It turns your Markdown Architecture Decision Records (ADRs) and notes into sub-10ms executable Git pre-commit and Claude Code Stop-hook guardrails. It feeds agents a diff-aware `<300-token` project loadout, blocks forbidden code patterns before commit, and features zero-LLM deterministic supersession (`k0maru flush --supersedes`) that hot-swaps architectural rules without belief entrenchment.  
> 
> It is a single, zero-daemon, pure-Rust binary with sub-4ms cold start, no resident background processes, and zero network ports."

### 中文：30 秒极速汇报

> “现有的智能体记忆工具（如 `claude-mem` 或 `Mem0`）本质上只是被动的会话聊天记录器：它们把大量冗长的对话流水账塞进数据库，随着时间推移产生‘信念固化（Belief Entrenchment）’并严重投毒上下文。  
> 
> **K0maru-Agent-Memory** 是**主动、可执行的活体架构决策守卫引擎**。它将本地 Markdown 格式的架构决策记录（ADR）转化为低于 10ms 的 Git Pre-commit 与 Claude Code Stop-hook 确定性代码护栏。它在开局为智能体动态加载小于 300 Token 的增量认知背包，在提交前拦截架构违规，并通过零 LLM 确定性状态机（`k0maru flush --supersedes`）实现新旧架构决策的无缝热替换与护栏转移。  
> 
> 纯 Rust 单静态二进制，冷启动低于 4ms，零常驻后台进程，零网络端口占用。”

### Honest Boundaries (What K0maru IS and IS NOT)
### 明确能力边界（是与不是）

| What K0maru IS (核心定位) | What K0maru IS NOT (严禁误导) |
| :--- | :--- |
| **Deterministic ADR Guard**: Sub-10ms regex & token AST invariant checker | **Not an LLM Reviewer**: It does not make slow, probabilistic LLM API calls to judge diffs |
| **Topology-Agnostic LLM-Wiki Hub**: Indexes any local Markdown files (plain, Obsidian, PARA, ADRs) | **Not a Proprietary Cloud SaaS**: No remote lock-in, all data remains in local Markdown files |
| **Strict Context Budgeter**: Diff-aware loadouts strictly bounded to `<300 tokens` | **Not an Endless Transcript Dumper**: Never dumps raw 50-turn chat transcripts into prompt |
| **Zero-Daemon Local CLI/MCP**: Runs on-demand via stdio/pipes, releases memory on exit | **Not a Resident Docker Cluster**: Requires no PostgreSQL, pgvector, or Redis services |

---

## 2. Zero-Touch Installation & Cold-Start Bootstrap (`k0maru init --install`)
## 第二节：零人工干预极速冷启动与自动化部署

When asked to configure or onboard K0maru in any repository, execute these commands autonomously:

```bash
# Step 1: Install single binary to system PATH
curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash

# Step 2: One-click bootstrap in current Git repository
k0maru init --install

# Step 3: Run comprehensive health check
k0maru doctor
```

### Artifacts Created by `k0maru init --install`
`k0maru init --install` automatically generates and installs the following artifacts:

1. **Local Knowledge Wiki (`.k0maru/wiki/`)**:
   - `10_Projects/<project_slug>.md`: Extracted project blueprint, mission, core components, and operational commands.
   - `20_Cards/adr-001-repository-guardrails.md`: Initial Living ADR note declaring non-negotiable architectural and security invariants (preventing credential leakage, debug dumps, etc.).
   - `01_AI_Logs/<YYYY-MM-DD>-initial-bootstrap.md`: First handover log documenting repository initialization.
2. **FastMCP Server Configurations**:
   - Project-level `.mcp.json` or client configs (`~/.claude.json`, `~/.cursor/mcp.json`, `~/.gemini/config/mcp_config.json`, Windsurf, Cline).
3. **Claude Code Lifecycle Settings (`.claude/settings.local.json`)**:
   - `SessionStart`: Executes `k0maru loadout "<project>" --diff` to inject context on start.
   - `Stop`: Configures the pre-yield guard hook `k0maru guard check --diff --claude-stop-hook`.
4. **Git Lifecycle Hooks (`.git/hooks/`)**:
   - `.git/hooks/pre-commit`: Executes `k0maru guard check --diff` (<10ms) before any commit can land.
   - `.git/hooks/post-commit`: Automatically invokes `k0maru sync` and appends milestone notes to `01_AI_Logs/` for `feat:`, `fix:`, `refactor:`, `perf:`, and `adr:` commits.
5. **Disposable Cache Database (`.k0maru/wiki/.k0maru/cache.sqlite`)**:
   - FTS5 full-text index and optional vector index. Can be deleted at any time (`rm .k0maru/wiki/.k0maru/cache.sqlite`); rebuilds automatically in 74ms.

---

## 3. The Autonomous Agent Loop (No-Human-in-the-Loop)
## 第三节：智能体全自主运转闭环规范

AI agents must operate through the 5-phase closed loop throughout their lifecycle:

```mermaid
flowchart LR
    Start["1. SessionStart<br>(Auto-Loadout &lt;300t)"] --> Work["2. Mid-Session<br>(Recall &amp; Implement)"]
    Work --> StopHook{"3. Claude Stop Hook<br>(guard check --claude-stop-hook)"}
    StopHook -->|Exit 2: [DENY]| Heal["4. Self-Healing<br>(Refactor or --supersedes)"]
    Heal --> Work
    StopHook -->|Exit 0: PASS| Commit["5. Git Commit<br>(pre-commit guard)"]
    Commit --> Post["6. Post-Commit<br>(Sync &amp; Milestone Log)"]
```

### Phase 1: Session Initiation (`SessionStart`)
- Claude Code / agent environment automatically triggers:
  ```bash
  k0maru loadout "<project>" --diff
  ```
  or calls MCP `get_project_loadout({ "diff": true })`.
- **Result**: Injects `<300 tokens` containing project mission, relevant ADR invariants activated by current Git diff hunks, and recent engineering logs.

### Phase 2: Mid-Session Implementation
- When checking technical patterns, prior bugs, or conventions, call MCP `recall_memory` or run `k0maru search "<query>"`.
- Do not guess historical design choices; query the knowledge base first.

### Phase 3: Pre-Yield Inspection (`hooks.Stop`)
- Before yielding output to the user, Claude Code invokes `hooks.Stop`:
  ```bash
  k0maru guard check --diff --claude-stop-hook
  ```
- **Exit Code 0**: Changes comply with all active ADR invariants. The response proceeds to the human.
- **Exit Code 2 (`[DENY]`)**: The command outputs a structured error block containing rule violations and remediation instructions. Claude Code intercepts exit code 2 and keeps the turn alive, allowing the agent to self-heal autonomously without user intervention.

### Phase 4: Pre-Commit Validation (`.git/hooks/pre-commit`)
- Staged changes are audited deterministically in `<10ms`:
  ```bash
  k0maru guard check --diff
  ```
- Any `[DENY]` violation halts the commit with POSIX exit code 1.

### Phase 5: Post-Commit Synchronization & Crystallization (`.git/hooks/post-commit`)
- Once committed, the post-commit hook automatically:
  1. Increments cache indices via `k0maru sync` (~11ms).
  2. If the commit message starts with `feat:`, `fix:`, `refactor:`, `perf:`, or `adr:`, it crystallizes a milestone note into `.k0maru/wiki/01_AI_Logs/YYYY-MM-DD-<slug>.md` documenting the change.

---

## 4. Self-Healing Decision Tree When `k0maru guard` Blocks You (`[DENY]`)
## 第四节：守卫拦截（[DENY]）时的自愈决策树

When `k0maru guard` blocks execution with `[DENY]`:

```text
[DENY] Rule 'enforce-fenced-lease' violated in src/settlement/lease.rs:18
Message: Settlement flows must not use raw redis_setnx.
Remedy: Acquire distributed fenced lease via acquire_fenced_lease().
```

**CRITICAL INSTRUCTION**: **NEVER** run `git commit --no-verify` or attempt to delete the guard hook. Choose between Case A and Case B:

### Case A: Accidental Violation (Code Defect)
*The human asked for a normal feature/bugfix, but you inadvertently used a forbidden API or omitted a mandatory check.*

1. Inspect the reported file, line number, and `Remedy:`.
2. Refactor the code to satisfy the invariant (e.g. replace `redis_setnx` with `acquire_fenced_lease`).
3. Re-run:
   ```bash
   k0maru guard check --diff
   ```
4. Verify output: `✅ Ready to commit` (Exit code 0). Proceed to commit.

### Case B: Intentional Architectural Upgrade / Migration
*The human explicitly requested to deprecate, replace, or upgrade an old architecture pattern.*

1. Run `k0maru flush` with `--supersedes` to mark the old ADR superseded, eliminate its belief entrenchment, and register the new invariant:
   ```bash
   k0maru flush \
     --category decision \
     --title "ADR-042: Fenced Distributed Token Lease Standard" \
     --supersedes "ADR-010: Legacy Redis SetNX Lock" \
     --summary "Migrated settlement locking to distributed fenced token leases" \
     --content $'## Non-Negotiable Invariants\n- [fenced-lease-required] (deny) [src/settlement/**/*] must_contain: "acquire_fenced_lease" | remediation: "Use acquire_fenced_lease() for all settlement flows."' \
     --tags "architecture,redis,leases"
   ```
2. Stage the updated knowledge base together with your code changes:
   ```bash
   git add .k0maru/wiki src/
   ```
3. Re-run `k0maru guard check --diff`. The old invariant is now retired and the new invariant is active.
4. Proceed to commit.

---

## 5. Complete Invariant Syntax Reference
## 第五节：活体架构不变量（Invariant）完整语法规范

Invariants can be authored using either **Markdown List Syntax** inside note bodies or **YAML Frontmatter** in ADR cards.

### Syntax 1: Markdown List Syntax (Recommended for Notes & ADRs)

Place under a `## Invariants` or `## Non-Negotiable Invariants` header in any Markdown note:

```markdown
## Non-Negotiable Invariants

- [rule-id] (deny) [glob/pattern/**] forbidden: "raw_sql_query" | remediation: "Use safe repository query builder."
- [must-use-lease] (deny) [src/settlement/**/*.rs] must_contain: "acquire_fenced_lease" | remediation: "Settlement methods must acquire a fenced lease."
- [warn-on-unwrap] (warn) [crates/**/*.rs] forbidden: ".unwrap()" | remediation: "Prefer using '?' operator or map_err."
```

**Grammar Structure**:
`- [<id>] (<severity>) [<path_glob>] <condition>: "<literal_or_regex>" | remediation: "<instruction>"`
- `<severity>`: `deny` (exit code 1 / 2 blocking) or `warn` (advisory warning).
- `<path_glob>`: Glob expression matching relative file paths (e.g. `src/**/*.rs`, `pkg/auth/**`).
- `<condition>`:
  - `forbidden`: Pattern must **NOT** appear in added diff hunks.
  - `must_contain`: Modified files matching `<path_glob>` **MUST** include this pattern.
- `remediation`: Clear, actionable fix instructions printed directly to the agent.

### Syntax 2: YAML Frontmatter Syntax

Declare directly inside the YAML frontmatter block of any note in `20_Cards/` or `10_Projects/`:

```yaml
---
title: ADR-042 Distributed Fenced Lease
category: decision
status: active
invariants:
  - id: "fenced-lease-required"
    severity: "deny"
    scope: ["src/settlement/**/*.rs"]
    condition:
      must_contain: ["acquire_fenced_lease"]
    fix_hint: "All settlement mutations must acquire a distributed fenced lease."
  - id: "no-raw-redis-setnx"
    severity: "deny"
    scope: ["src/**/*.rs"]
    condition:
      forbidden: ["redis_setnx", "SETNX"]
    fix_hint: "Direct unscoped SETNX is strictly prohibited. Use lease module."
---
```

### Syntax 3: Local Repository Fallback (`.k0maru/rules` or `guard.toml`)

For repositories without a wiki, rules can be placed in `.k0maru/rules` or `.k0maru/guard.toml`:

```toml
[[rules]]
id = "SEC-001"
title = "No hardcoded credentials"
file_pattern = "src/**/*.{rs,ts,py,go}"
severity = "deny"
forbidden = "DANGEROUS_SECRET_KEY"
remediation = "Retrieve credentials via std::env::var or secret vault."
```
