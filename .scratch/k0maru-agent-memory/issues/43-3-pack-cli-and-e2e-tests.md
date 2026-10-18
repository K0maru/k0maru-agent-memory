# Ticket 43-3: Wire `k0maru pack` CLI Subcommands (`export`, `import`, `inspect`) and Cross-Topology E2E Tests

- **Status**: `done`
- **Blocked by**: None (Parallel Slice 3)

## Target Files
- `src/lib.rs`
- `src/main.rs`
- `tests/test_pack.rs` (new file)

## Acceptance Criteria
- [x] `src/lib.rs` exports `pub mod pack;` and re-exports `export_pack, import_pack, inspect_pack, MemoryPack, PackExportOptions, PackImportOptions, PackImportResult, PackInspectReport, PackNote`.
- [x] `src/main.rs` adds `Pack(PackArgs)` with `PackSubcommand::{Export, Import, Inspect}` supporting `--json` and human-readable output.
- [x] `tests/test_pack.rs` verifies exporting a Memory Pack from a Johnny.Decimal vault (`20_Cards/`) and importing it into:
  1. A flat Karpathy-style LLM-Wiki (notes land in root with kebab-case filenames, and `k0maru guard check` immediately enforces the imported ADR invariants);
  2. An engineering `decisions/` + `cards/` wiki (notes land in `decisions/` / `cards/`).
