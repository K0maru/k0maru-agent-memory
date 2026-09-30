# 01 — Project scaffolding, toolchain, and crate setup

**What to build:** Establish the Rust project structure managed by `cargo` with `Cargo.toml`, configured with workspace dependencies (`clap`, `serde`, `rusqlite`, `pulldown-cmark`, `tempfile`), strict compiler lints (`clippy`), and the initial `k0maru` binary and library skeletons, ensuring all downstream components compile with zero external C dynamic library dependencies.

**Blocked by:** None — can start immediately.

**Status:** resolved

- [x] `Cargo.toml` configured with Rust 2021 edition, binary target `k0maru` and library target `k0maru`
- [x] Dependencies configured: `clap` (derive), `serde`, `serde_json`, `serde_yaml`, `rusqlite` (bundled), `pulldown-cmark`, `walkdir`, `arboard`, `xxhash-rust`
- [x] Dev-dependencies configured: `tempfile`, `assert_cmd`, `predicates`
- [x] `src/lib.rs` and `src/main.rs` established with initial version `0.1.0`
- [x] `cargo test` executes and passes cleanly with 0 errors
- [x] `cargo clippy -- -D warnings` passes with 0 violations
- [x] `cargo run -- --version` prints `k0maru 0.1.0`

## Implementation Summary
- Initialized Rust 2021 crate `k0maru` (v0.1.0) with binary and library targets.
- Configured all baseline dependencies (`clap`, `serde`, `rusqlite` with `bundled`, `pulldown-cmark`, `walkdir`, `arboard`, `xxhash-rust`) and dev dependencies (`tempfile`, `assert_cmd`, `predicates`).
- Created `src/lib.rs` exporting `VERSION` constant and unit test.
- Created `src/main.rs` establishing Clap CLI structure supporting `--version` and `--help`.
- Added integration smoke tests in `tests/test_cli_smoke.rs` verifying `--version` and `--help` CLI behavior via `assert_cmd`.
- Verified `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check` all pass with zero errors and zero warnings.
