# Ticket 44-1: Implement Native Tern Luau Plugin (`tern-plugin/`)

- **Status**: `done`
- **Blocked by**: None (Parallel Slice 1)

## Target Files
- `tern-plugin/plugin.toml`
- `tern-plugin/host.luau`
- `tern-plugin/window.luau`
- `tern-plugin/README.md`

- [x] `tern-plugin/` loads cleanly via `tern plugin link ./tern-plugin --json` with status `"Ready"` and 0 errors.
- [x] `host.luau` implements `tern.block.define("cockpit", ...)` and `tern.lens.define("cli", ...)` using `tern.ui.col`, `tern.ui.card`, `tern.ui.table`, `tern.ui.meter_cell`, and `tern.ui.badge`.
- [x] `window.luau` registers palette commands and `tern.carly.export("recall", ...)`.
- [ ] `window.luau` registers palette commands and `tern.carly.export("recall", ...)`.
