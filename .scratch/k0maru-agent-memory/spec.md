# Spec 44: Tern Native Luau Plugin, omp/Claude Dual Marketplace & Documentation Convergence

## 1. Deliverables
1. **Tern Native Luau Plugin (`tern-plugin/`)**:
   - `tern-plugin/plugin.toml`: schema v1 manifest for `k0maru-memory` declaring `cockpit` block and `cli` command lens.
   - `tern-plugin/host.luau`:
     - `tern.block.define("cockpit", ...)`: interactive Tern block running `k0maru doctor --json` and `k0maru guard check --json`, rendering structured cards, tables, badges, and `meter_cell` bars (`r` to refresh).
     - `tern.lens.define("cli", ...)`: command lens matching `k0maru *` / `k0maru-memory *` that parses JSON or tabular output into native `tern.ui` tables/cards.
   - `tern-plugin/window.luau`:
     - Registers palette commands (`cockpit`, `guard_check`, `doctor`) and Carly export `recall`.
2. **Dual Marketplace Catalogs (`.omp-plugin/` & `.claude-plugin/` + `plugins/k0maru-memory/`)**:
   - `.omp-plugin/marketplace.json` & `.claude-plugin/marketplace.json` pointing to `./plugins/k0maru-memory`.
   - `plugins/k0maru-memory/.omp-plugin/plugin.json` (`"mcpServers": "./mcp-omp.json"` with `--sidecar`) and `.claude-plugin/plugin.json` (`"mcpServers": "./.mcp.json"`).
   - Bundles `skills/k0maru-guard/SKILL.md`.
3. **Bilingual Documentation Convergence (`README.md`, `README_zh.md`, `docs/`)**:
   - Document heterogeneous LLM-Wiki support (Karpathy flat wiki, `decisions/`/`docs/adr/`, PARA, SecondBrain).
   - Document `k0maru mcp --sidecar`, `k0maru install --target omp`, `k0maru pack export/import/inspect`, `tern plugin install`, and `/marketplace add K0maru/k0maru-agent-memory`.
