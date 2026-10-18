# Ticket 42-1: Remove Legacy `crystal` & `kdsl` Modules and Wire CLI `--sidecar`

- **Status**: `done`
- **Blocked by**: None (Parallel Slice 1)

## Target Files
- Delete `src/crystal/mod.rs`
- Delete `crates/k0maru-core/src/guard/kdsl.rs`
- Edit `crates/k0maru-core/src/guard/mod.rs`
- Edit `crates/k0maru-core/src/guard/engine.rs`
- Edit `src/guard/mod.rs`
- Edit `src/lib.rs`
- Edit `src/main.rs`
- Edit `tests/test_distill.rs`
- Edit `tests/test_guard.rs`

## Acceptance Criteria
- [x] `src/crystal/mod.rs` and `crates/k0maru-core/src/guard/kdsl.rs` are deleted.
- [x] `GuardEngine::check_kdsl` and `GuardCheckArgs.kdsl` are removed.
- [x] `Commands::Crystal`, `CrystalArgs`, and `run_crystal` are removed from `src/main.rs` and `src/lib.rs`.
- [x] `McpArgs` in `src/main.rs` gains `#[arg(long)] pub sidecar: bool` and `run_mcp` calls `server = server.with_sidecar(args.sidecar)`.
- [x] Legacy tests for `crystal` in `tests/test_distill.rs` and `kdsl` in `tests/test_guard.rs` are removed.
