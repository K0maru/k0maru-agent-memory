# Ticket 36: English Documentation Suite (`docs/en/`)

**Status**: closed
**Blocked by**: 34, 35

## Goal
Author the comprehensive, world-class English technical documentation suite under `docs/en/` maintaining 100% chapter parity with `docs/zh/`, strictly adhering to the **Google Developer Documentation Style Guide** (second-person "you", active voice, present tense, clear descriptive headings, exact code and command formatting, and parallel list items).

## Tasks
1. `docs/en/index.md`: English Hero landing page with value propositions, quick links, feature badges, and architecture teaser.
2. `docs/en/guide/introduction.md`:
   - Introduction, motivation, and problem statement (cross-session amnesia, compiler log bloat, microservice bloat anti-pattern);
   - "Bring Your Own Markdown" (BYOM) design philosophy and zero lock-in;
   - Comprehensive competitive matrix comparing Mem0, Letta, Chroma, and raw SQLite vector solutions.
3. `docs/en/guide/installation.md`:
   - One-line script (`curl | bash`), Homebrew Tap, Cargo crates.io, pre-compiled GitHub releases, building from source;
   - Environment variables and verification commands.
4. `docs/en/guide/quickstart.md`:
   - 3-minute end-to-end walkthrough (`doctor` -> `loadout` -> `offload` -> `search` -> `flush`);
   - Self-healing verification step.
5. `docs/en/architecture/overview.md`:
   - 4-layer decoupled architecture (Presentation, Domain, Retrieval & Fusion, Storage);
   - Memory hierarchy tiers (L3 Evergreen, L2 Decision Logs, L1 External Resources, L0 Scratch / Logs);
   - Memory lifecycle state diagram (Mermaid).
6. `docs/en/architecture/key-implementations.md`:
   - xxh3 incremental scanner and 0-I/O dirty detection;
   - BM25 + ONNX Vector + 1-hop Graph Boost RRF hybrid retrieval algorithm and mathematical formulations;
   - FastMCP Stdio protocol and natural language intent mapping;
   - Symbolic log offloading, state machine extraction, and SVG Mermaid generation;
   - Adaptive Vault Convention Sniffer;
   - Crystallization Flush Engine with anti-collision mechanics;
   - Trace-to-Skill Heuristic Distiller;
   - Developer Dark Cockpit WebUI console.
7. `docs/en/workflows/ecosystem.md`:
   - Ecosystem integration via `k0maru install`: Claude Code, Cursor, Windsurf, Gemini CLI, Cline / Roo Code, Hermes Agent, OpenClaw;
   - Manual configuration and JSON-RPC pipe debugging;
   - `AGENTS.md` system prompt writing best practices.
8. `docs/en/workflows/scenarios.md`:
   - Scenario A: Enterprise LLM-Wiki maintenance (payment gateway & distributed lock invariants);
   - Scenario B: Cross-session debugging and self-healing loops;
   - Scenario C: Dynamic skill distillation and autonomous playbook reuse.
9. `docs/en/reference/cli.md`:
   - Comprehensive CLI reference for all subcommands (`loadout`, `search`, `offload`, `inspect`, `sync`, `flush`, `distill`, `install`, `doctor`, `ui`, `mcp`), flags, options, exit codes, and JSON outputs.
10. `docs/en/reference/mcp.md`:
    - FastMCP 6 core tools (`get_project_loadout`, `recall_memory`, `offload_context`, `inspect_log_node`, `flush_session`, `distill_session_skill`) schemas, parameter constraints, and example payloads.
11. `docs/en/benchmarks/a100-evaluation.md`:
    - Empirical NVIDIA A100-SXM4-80GB benchmark analysis (Qwen2.5-Coder-32B, DeepSeek-R1-32B, DeepSeek-Coder-V2-16B, Qwen3.8-27B);
    - Token Reduction Ratio (TRR), first-turn latency acceleration, Pass@1 accuracy, visualization figures, and reproduction steps.
