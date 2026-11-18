---
name: memory-hub
description: Recall project context (<300 tokens), progressive disclosure retrieval (timeline -> recall -> inspect), and crystallize Living ADRs and session logs into any Markdown LLM-Wiki or Obsidian vault.
---

# memory-hub (K0maru Agent Memory Hub)

A standalone, single-static-binary (`k0maru`), zero-daemon memory and Living ADR engine mounting any local Markdown directory (`.k0maru/wiki`, Obsidian Vault, or Karpathy LLM-Wiki).

## Standard LLM-Wiki Directory Structure

K0maru mounts standard Karpathy LLM-Wiki directories (and maintains full backward compatibility with Johnny.Decimal and PARA vaults):

- `projects/`: Project hubs and vision cards (e.g. `projects/<slug>.md` or `10_Projects/`)
- `decisions/`: Living ADR architectural invariants and principles (e.g. `decisions/adr-001-repository-guardrails.md` or `20_Cards/`)
- `logs/`: Engineering logs and passive session observations (e.g. `logs/<YYYY-MM-DD>-session-observations.md` or `01_AI_Logs/`)
- `INDEX.md`: Master Map of Content (MOC) linking active projects, decisions, and logs

## 3-Stage Progressive Disclosure Retrieval

Always follow the 3-stage progressive disclosure pipeline to maximize context token efficiency:

1. **Stage 1: Fast Recall (`recall_memory`)**:
   - Query hybrid BM25 + ONNX vector + WikiLinks graph memories in compact K-DSL format (~50 tokens/hit).
   - MCP: `recall_memory({ "query": "<topic>", "limit": 5 })`
   - CLI: `k0maru search "<topic>"`

2. **Stage 2: Chronological Neighborhood (`timeline_memory`)**:
   - Inspect the chronological stream of engineering logs, session checkpoints, and ADR milestones around an event, note, or date.
   - MCP: `timeline_memory({ "anchor": "<date_or_note>", "window": 10 })`
   - CLI: `k0maru timeline --anchor "<date_or_note>" --window 10`

3. **Stage 3: Deep Note Inspection (`inspect_note`)**:
   - Inspect full note metadata, frontmatter, supersession chains, inbound/outbound links, and markdown body.
   - MCP: `inspect_note({ "path": "decisions/adr-001.md" })`
   - CLI: `k0maru inspect "decisions/adr-001.md"`

## Core Autonomous Loop (Zero-Human-in-the-Loop)

1. **Cold-Start Repository Onboarding**:
   ```bash
   k0maru init --install
   ```
   Synthesizes `.k0maru/wiki/` (`projects/`, `decisions/adr-001-repository-guardrails.md`, `logs/`, `INDEX.md`), indexes `.k0maru/wiki/.k0maru/cache.sqlite`, and installs `.mcp.json`, `.claude/settings.local.json` (`SessionStart` + `Stop`), and `.git/hooks/{pre-commit,post-commit}`.

2. **Session Start (`<300 Tokens` Context Loadout)**:
   - MCP: `get_project_loadout({ "diff": true })`
   - CLI: `k0maru loadout --diff`

3. **Proactive Memory Recall & Progressive Disclosure**:
   - `recall_memory` -> `timeline_memory` -> `inspect_note`

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
