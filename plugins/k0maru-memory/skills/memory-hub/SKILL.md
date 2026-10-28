---
name: memory-hub
description: Recall project context (<300 tokens), search hybrid BM25 + ONNX vector + WikiLinks graph memories, and crystallize Living ADRs and session logs into any Markdown LLM-Wiki or Obsidian vault.
---

# memory-hub (K0maru Agent Memory Hub)

A standalone, single-static-binary (`k0maru`), zero-daemon memory and Living ADR engine mounting any local Markdown directory (`.k0maru/wiki`, Obsidian Vault, or flat Karpathy LLM-Wiki).

## Core Autonomous Loop (Zero-Human-in-the-Loop)

1. **Cold-Start Repository Onboarding**:
   ```bash
   k0maru init --install
   ```
   Synthesizes `.k0maru/wiki/` (`10_Projects/`, `20_Cards/adr-001-repository-guardrails.md`, `01_AI_Logs/`), indexes `.k0maru/wiki/.k0maru/cache.sqlite`, and installs `.mcp.json`, `.claude/settings.local.json` (`SessionStart` + `Stop`), and `.git/hooks/{pre-commit,post-commit}`.

2. **Session Start (`<300 Tokens` Context Loadout)**:
   - MCP: `get_project_loadout({ "diff": true })`
   - CLI: `k0maru loadout --diff`

3. **Proactive Memory Recall (Hybrid BM25 + ONNX Vector + WikiLinks Graph)**:
   - MCP: `recall_memory({ "query": "<topic>", "limit": 5 })`
   - CLI: `k0maru search "<topic>"`

4. **Living ADR Guard & Autonomous Supersession**:
   - MCP: `guard_check({})`
   - CLI: `k0maru guard check --diff`
   - Supersede stale ADR:
     ```bash
     k0maru flush --category decision \
       --title "ADR-042: New Architecture Standard" \
       --supersedes "adr-010-legacy" \
       --summary "Replaced legacy pattern with v2 standard" \
       --content $'## Non-Negotiable Invariants\n- [rule-id] (deny) [src/**/*] must_contain: "v2_api" | remediation: "Use v2_api."'
     ```
