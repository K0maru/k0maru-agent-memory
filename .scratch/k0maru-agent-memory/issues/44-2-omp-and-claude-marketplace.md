# Ticket 44-2: Implement `.omp-plugin` & `.claude-plugin` Dual Marketplace and Plugin Package

- **Status**: `done`
- **Blocked by**: None (Parallel Slice 2)

## Target Files
- `.omp-plugin/marketplace.json`
- `.claude-plugin/marketplace.json`
- `plugins/k0maru-memory/.omp-plugin/plugin.json`
- `plugins/k0maru-memory/.claude-plugin/plugin.json`
- `plugins/k0maru-memory/.mcp.json`
- `plugins/k0maru-memory/mcp-omp.json`
- `plugins/k0maru-memory/skills/k0maru-guard/SKILL.md`
- `plugins/k0maru-memory/README.md`

- [x] Both `.omp-plugin/marketplace.json` and `.claude-plugin/marketplace.json` conform to the marketplace catalog schema (`name`, `owner.name`, `metadata.description`, `plugins`).
- [x] `plugins/k0maru-memory/mcp-omp.json` mounts `k0maru` with `["mcp", "--sidecar"]`, while `.mcp.json` mounts `k0maru` with `["mcp"]`.
- [x] `plugins/k0maru-memory/skills/k0maru-guard/SKILL.md` provides the complete Living ADR Guard skill.
- [ ] `plugins/k0maru-memory/skills/k0maru-guard/SKILL.md` provides the complete Living ADR Guard skill.
