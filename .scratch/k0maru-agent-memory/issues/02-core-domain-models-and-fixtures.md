# 02 — Core domain models, traits, and test vault fixtures

**What to build:** Core Rust domain structs (`Document`, `WikiLink`, `HierarchyLevel`, `SyncStats`) with Serde serialization, abstract trait contracts (`VaultAdapter`, `CacheStorage`), and test helper fixtures generating isolated mock vaults (Obsidian and Karpathy-style) using `tempfile::TempDir`.

**Blocked by:** 01 — Project scaffolding, toolchain, and crate setup

**Status:** resolved

- [x] `HierarchyLevel` enum defined (`L0Ephemeral`, `L1Resource`, `L2Log`, `L3Evergreen`) with Serde serialization
- [x] `WikiLink` struct defined with `target: String`, `alias: Option<String>`, `raw_text: String`
- [x] `Document` struct defined with `path: PathBuf`, `title: String`, `hierarchy: HierarchyLevel`, `frontmatter: serde_json::Value`, `links: Vec<WikiLink>`, `tags: Vec<String>`, `content_hash: String`, `mtime: u64`, `body: String`
- [x] `SyncStats` struct defined with processed counts and elapsed duration
- [x] `VaultAdapter` and `CacheStorage` traits defined in `src/core/traits.rs`
- [x] Test helper `tests/common/fixtures.rs` provides functions to generate `mock_obsidian_vault()` and `mock_karpathy_wiki()` in temporary directories
- [x] `tests/test_domain_models.rs` validates serialization, deserialization, and immutability invariants
- [x] `cargo test` passes cleanly with zero warnings

## Completion Summary
- Defined `HierarchyLevel`, `WikiLink`, `Document`, and `SyncStats` with full Serde serialization/deserialization in `src/core/models.rs`.
- Defined abstract trait contracts `VaultAdapter` and `CacheStorage` in `src/core/traits.rs`.
- Exposed modules via `src/core/mod.rs` and `pub mod core;` in `src/lib.rs`.
- Implemented reusable mock vault fixtures (`mock_obsidian_vault`, `mock_karpathy_wiki`) in `tests/common/fixtures.rs` using `tempfile::TempDir`.
- Implemented complete integration test suite in `tests/test_domain_models.rs` covering serialization round-trips, trait contracts, and mock vault filesystem verifications.
- Verified 100% test pass rate, `cargo clippy --all-targets -- -D warnings` zero warnings, and `cargo fmt --check` compliance.
