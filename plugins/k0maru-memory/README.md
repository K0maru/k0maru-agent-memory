# k0maru-memory

> **Standalone, zero-daemon LLM-Wiki & Obsidian memory hub for AI coding agents** — hybrid BM25 + ONNX vector + WikiLinks RRF recall, a sub-10ms Living ADR Guard, project loadouts, and knowledge crystallization.

This plugin bundles the `k0maru-memory` MCP server plus the `k0maru-guard` skill for both **omp** and **Claude Code**.

---

## Prerequisites

Install the `k0maru` binary so the MCP server can be spawned by your agent:

```bash
# macOS / Linux (standalone binary, no Rust toolchain)
curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash

# Homebrew
brew install K0maru/tap/k0maru

# From source
cargo install --git https://github.com/K0maru/k0maru-agent-memory
```

Then confirm the binary resolves on `PATH`:

```bash
k0maru --version
```

The MCP server needs a Markdown vault. It resolves the vault by walking up from the agent's working directory looking for `.k0maru/`, `.obsidian/`, `10_Projects/`, or `20_Cards/`. To pin a specific vault, add `--vault /path/to/your/vault` to the `args` array in whichever config below you use.

---

## Install in omp

```text
/marketplace add K0maru/k0maru-agent-memory
/marketplace install k0maru-memory@k0maru-agent-memory
```

omp loads this plugin through `.omp-plugin/plugin.json` → `mcp-omp.json`, which starts the server in **sidecar mode**:

```json
{ "command": "k0maru", "args": ["mcp", "--sidecar"] }
```

Sidecar mode exposes the **5 core memory tools**, intentionally omitting `offload_context`, `inspect_log_node`, and `gc_session_context` so it composes orthogonally with omp's own context/offload pipeline:

| Tool | Purpose |
| :--- | :--- |
| `get_project_loadout` | High-priority architectural blueprint, invariants, vision, and recent ADRs |
| `recall_memory` | Hybrid RRF retrieval across BM25, vectors, and WikiLinks |
| `flush_session` | Crystallize decisions, fixes, and ADRs into your vault as Markdown |
| `distill_session_skill` | Turn a trace/failure into a reusable troubleshooting playbook |
| `guard_check` | Audit the current git diff against Living ADR invariants |

---

## Install in Claude Code

```text
/plugin marketplace add K0maru/k0maru-agent-memory
/plugin install k0maru-memory@k0maru-agent-memory
```

Claude Code loads this plugin through `.claude-plugin/plugin.json` → `.mcp.json`, which starts the server with the **full 8-tool surface**:

```json
{ "command": "k0maru", "args": ["mcp"] }
```

| Tool | Purpose |
| :--- | :--- |
| `get_project_loadout` | Project blueprint, invariants, vision, and recent ADRs (<300 tokens) |
| `recall_memory` | Search notes, research, ADRs, configs, and setup guides in the vault |
| `flush_session` | Persist technical decisions and verified fixes as permanent notes |
| `distill_session_skill` | Distill an execution trace into a reusable skill |
| `offload_context` | Compress verbose logs/traces into a symbolic Mermaid state diagram on disk |
| `inspect_log_node` | Retrieve offloaded raw log content by node ID (`node_1a2b3c4d`) |
| `gc_session_context` | Context GC across a long agent trajectory |
| `guard_check` | Audit the current git diff against Living ADR invariants |

---

## Bundled skill: `k0maru-guard`

`skills/k0maru-guard/SKILL.md` teaches the agent to run Living ADR checks before commits, during `/implement`, and during `/code-review`, and to triage `deny` violations instead of bypassing them with `--no-verify`.

The same guard is reachable outside the agent:

```bash
k0maru guard check --diff          # audit the working tree/index (<5ms)
k0maru guard check --strict        # warnings become exit code 1
k0maru guard check --json          # structured output for CI
k0maru guard install               # non-destructive pre-commit hook
```

Invariants are declared as YAML frontmatter in ADR notes (a `invariants:` block) or as a `.k0maru/guard.toml` fallback for repositories without a vault.

---

## Layout

```text
plugins/k0maru-memory/
├── .claude-plugin/plugin.json   # Claude Code manifest (8 tools)
├── .omp-plugin/plugin.json      # omp manifest (5 core tools)
├── .mcp.json                    # k0maru mcp
├── mcp-omp.json                 # k0maru mcp --sidecar
├── README.md
└── skills/k0maru-guard/SKILL.md # Living ADR Guard skill
```

## Links

- [Repository](https://github.com/K0maru/k0maru-agent-memory)
- [Main README](https://github.com/K0maru/k0maru-agent-memory#readme)
- [CLI & MCP reference](https://github.com/K0maru/k0maru-agent-memory/blob/main/docs/en/reference/cli-and-mcp.md)

## License

MIT
