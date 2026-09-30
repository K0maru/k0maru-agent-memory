# 08 — Symbolic log offloader pipe and inspect CLI command

**What to build:** The `k0maru offload` CLI pipe filter and `k0maru inspect <node_id>` command in `src/offload/engine.rs`, intercepting stdin, externalizing outputs >50 lines into `.scratch/refs/<task_id>_<node_id>.log`, printing a compact Mermaid state transition diagram to the terminal, and allowing instant retrieval of the offloaded trace.

**Blocked by:** 01 — Project scaffolding, toolchain, and crate setup

**Status:** resolved

- [x] `OffloadEngine` streams lines from standard input (`std::io::stdin`)
- [x] Short logs (<=50 lines) are emitted directly without truncation
- [x] Long logs (>50 lines) are truncated, saved to `.scratch/refs/<task_id>_<node_id>.log`, and replaced with summary statistics
- [x] Generates valid Mermaid state machine syntax (`stateDiagram-v2`) showing execution steps and `node_id` references
- [x] CLI command `k0maru offload` works seamlessly in shell pipes (e.g. `cat test.log | k0maru offload`)
- [x] CLI command `k0maru inspect <node_id>` looks up and prints the offloaded raw log content
- [x] Handles broken pipes, empty input, and missing node IDs gracefully
- [x] `tests/test_offload.rs` tests pipe thresholds, file persistence, Mermaid generation, and CLI inspection

## Completion Summary
- Created `src/offload/engine.rs` implementing `OffloadEngine`, `OffloadResult`, and `inspect_node(&Path, &str)`.
- Re-exported offload module and structs in `src/offload/mod.rs` and `src/lib.rs`.
- Extended `src/main.rs` with `k0maru offload` (streaming stdin, configurable threshold, task id prefix, refs directory auto-detection) and `k0maru inspect <NODE_ID>` (prints full trace or exits code 2 on missing node).
- Handled broken pipe and empty inputs gracefully across both engine and CLI stdout.
- Implemented comprehensive TDD test suite in `tests/test_offload.rs` covering threshold boundaries, persistence, Mermaid syntax generation, retrieval variants, broken pipes, and CLI integration.
- Code passed 100% tests (`cargo test`), `cargo clippy --all-targets -- -D warnings` zero warnings, and `cargo fmt`.
