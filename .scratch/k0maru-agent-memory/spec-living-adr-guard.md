# SPEC: Phase 10 Living ADR Guard & Architectural Invariant Sentinel (`k0maru guard`)

## 1. Background & Motivation
In LLM-assisted and autonomous agentic engineering, agents often perform pre-flight inspection via `get_project_loadout` or `recall_memory`, loading non-negotiable architectural invariants (ADRs). However, during implementation, agents may inadvertently omit mandatory patterns (e.g. omitting distributed locking, creating naked directional risk, or violating state machine guards).

Currently, there is no sub-millisecond, local physical gate preventing the agent or developer from committing such code until code review or expensive runtime tests fail.

**Phase 10 Living ADR Guard** introduces `k0maru guard`:
An ultra-fast (<10ms) architectural invariant sentinel that extracts living rules from LLM-Wiki ADRs / Project files, audits Git diffs or files, installs Git pre-commit hooks, and exposes a first-class MCP tool `guard_check` for agent self-auditing before committing.

---

## 2. Core Architecture & Workflow

```text
  LLM-Wiki (20_Cards & 10_Projects)
           │
           ▼
┌───────────────────────────────────────────────────────────┐
│               ADR Invariant Extractor                     │
│  - Parses YAML frontmatter `invariants` & body rules      │
│  - Parses `.k0maru/guard.toml` (if present)               │
└───────────────────────────────────────────────────────────┘
           │
           ▼ GuardRules: [MustContain, Forbidden, FileScope, Severity]
┌───────────────────────────────────────────────────────────┐
│                 Guard Analyzer Engine                     │
│  - Inspects Git Diff (staged / branch / unstaged)         │
│  - Filters rules by file scope (*.go, *.rs, etc.)         │
│  - Detects violations with exact line numbers & diff loci │
└───────────────────────────────────────────────────────────┘
           │
           ├──► CLI: `k0maru guard check [--diff] [--project]`
           ├──► Git Hook: `k0maru guard install [--hook pre-commit]`
           └──► FastMCP Tool: `guard_check(diff, project_name)`
```

---

## 3. Detailed Component Specifications

### 3.1 Data Structures (`k0maru-core::guard::models`)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleSeverity {
    Deny,
    Warn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleCondition {
    /// Added/modified lines must match this regex/substring
    MustContain(String),
    /// Added/modified lines must not match this forbidden regex/substring
    Forbidden(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardRule {
    pub id: String,
    pub title: String,
    pub source_note: String,
    pub file_pattern: Option<String>, // e.g. "*.go", "src/**/*.rs"
    pub condition: RuleCondition,
    pub severity: RuleSeverity,
    pub remediation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardViolation {
    pub rule_id: String,
    pub rule_title: String,
    pub file_path: String,
    pub line_number: Option<usize>,
    pub matched_content: Option<String>,
    pub severity: RuleSeverity,
    pub source_note: String,
    pub remediation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardReport {
    pub checked_files: usize,
    pub checked_diff_hunks: usize,
    pub violations: Vec<GuardViolation>,
    pub is_passed: bool,
    pub duration_ms: u128,
}
```

### 3.2 Invariant Extraction (`k0maru-core::guard::extractor`)
1. **Frontmatter Invariant Declaration**:
   ```yaml
   ---
   invariants:
     - id: "ADR-SETTLE-001"
       title: "Mandatory Distributed Lock"
       files: "*.go"
       must_contain: "redis.*SetNX|lock:"
       forbidden: "execWithoutLock"
       severity: deny
       remediation: "Acquire Redis lock before modifying ledger."
   ---
   ```
2. **Body Heuristic Extraction**:
   Scans `## Non-Negotiable Invariants` or `## 核心工程铁律` in `10_Projects/` and `20_Cards/` to extract declared rules.
3. **Optional Vault Override**:
   Reads `.k0maru/guard.toml` or `guard.yaml` if defined at vault root.

### 3.3 Guard Diff Analyzer (`k0maru-core::guard::engine`)
- Integrates with `crate::git::GitInspector` to obtain modified files and diff hunks.
- Matches file paths using glob pattern matching (`file_pattern`).
- Evaluates added lines (`+`):
  - If a file matches `file_pattern`:
    - Checks for `Forbidden` regex triggers.
    - If `MustContain` rule applies to a transaction / handler block, verifies presence of required invariant tokens.
- Produces `GuardReport`.

### 3.4 CLI Subcommand (`src/main.rs`)
- `k0maru guard check [--vault <path>] [--diff] [--project <name>] [--json] [--strict]`
  - Exits with `0` if passed, `1` if any `Deny` violation found, `2` on execution error.
- `k0maru guard install [--hook pre-commit] [--path <repo_dir>]`
  - Writes executable `.git/hooks/pre-commit` running `k0maru guard check --diff`.

### 3.5 MCP Tool Integration (`src/mcp/server.rs`)
- Exposes `guard_check`:
  - Arguments: `diff` (optional string), `project_name` (optional string).
  - Returns `GuardReport` in human and JSON format.

---

## 4. Acceptance Criteria
1. `k0maru guard check --diff` blocks commits containing violations with exit code 1 and prints exact remediation linking to the ADR.
2. `k0maru guard check --diff` passes clean compliant diffs with exit code 0 in <10ms.
3. `k0maru guard install` installs a non-destructive `.git/hooks/pre-commit` wrapper.
4. MCP `guard_check` tool callable from AI coding agents.
5. 100% test pass rate across unit, integration, and clippy gates.
