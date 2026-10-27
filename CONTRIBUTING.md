# Contributing to K0maru-Agent-Memory

Thank you for your interest in contributing to `k0maru-agent-memory`!

## Architectural Principles

1. **Deterministic Guardrails Over Probabilistic Magic**:
   Architectural invariants must be evaluated in `<10ms` in Rust with zero runtime network or LLM dependencies.
2. **Zero-Daemon / Single Static Binary**:
   `k0maru` runs on-demand via standard stdio streams and Unix pipes. It does not spawn background daemons or listen on network ports during normal execution.
3. **Bring Your Own Markdown**:
   The local Markdown vault is the single source of truth. The SQLite cache is purely disposable (`rm cache.sqlite` must self-heal in <100ms).
4. **Context Budget Consciousness**:
   Dynamic loadouts must remain bounded to `<300 tokens` to prevent attention drift.

## Development Workflow

### Prerequisites
- Rust 1.88+ (2021 Edition)
- `git`
- Recommended: `cargo-clippy`, `rustfmt`

### Build & Test
```bash
# Build pure-Rust core binary without ONNX Runtime (~3.6MB)
cargo build --no-default-features

# Build default release binary with fastembed + bundled ONNX Runtime (~34MB)
cargo build --release

# Run code formatting check
cargo fmt --all -- --check

# Run linter
cargo clippy --all-targets -- -D warnings

# Run test suite
cargo test --all-targets
```

## Branch & PR Strategy

- **Dual-Trunk Model**:
  - `main`: Production releases only. Protected; merges arrive exclusively from `dev` milestone PRs or critical hotfixes.
  - `dev`: Daily development trunk. All feature and fix PRs target `dev`.
- **Branch Naming**:
  - `feat/<issue-id>-<slug>`: Features tied 1:1 to GitHub Issues.
  - `fix/<issue-id>-<slug>`: Bug fixes.
  - `bench/<slug>`: Performance and benchmark evaluations.

## Coding Standards

- **Error Handling**: Use `Result<T, E>` and structured domain error enums (`StorageError`, `VectorError`, `GcError`); avoid `.unwrap()` in production paths.
- **Living ADR Invariants**: Run `k0maru guard check --diff` before every commit. Do not bypass guardrails with `--no-verify`.
- **Documentation**: New features must include tests, documentation updates, and CLI/MCP reference updates.
