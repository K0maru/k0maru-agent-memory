# 04 — Obsidian and Generic Wiki storage adapters

**What to build:** `ObsidianAdapter` and `GenericWikiAdapter` implementing the `VaultAdapter` trait, enabling the engine to scan directories, map folder taxonomies (`10_Projects`, `20_Cards`, `01_AI_Logs`, `00_Daily`) to `HierarchyLevel`, read files into `Document` entities, and write new cards safely.

**Blocked by:** 03 — Robust Markdown AST, Frontmatter, and WikiLinks parser

**Status:** resolved

- [x] `ObsidianAdapter` scans mock vaults using `walkdir` and identifies markdown documents with relative paths
- [x] Folder mapping correctly tags `20_Cards` as `L3Evergreen`, `01_AI_Logs` as `L2Log`, `10_Projects` as `L3Evergreen`, and `00_Daily` as `L0Ephemeral`
- [x] Frontmatter overrides: explicit `hierarchy:` or `level:` YAML keys override folder heuristics
- [x] `GenericWikiAdapter` indexes flat markdown files and relative links
- [x] `read_document(path)` constructs a complete, valid `Document` entity
- [x] `write_card(title, content, hierarchy)` writes new cards to designated folders without mutating existing notes
- [x] `tests/test_adapters.rs` passes all verification against `mock_obsidian_vault` and `mock_karpathy_wiki`

## Completion Summary

- Implemented `ObsidianAdapter` (`src/adapters/obsidian.rs`) and `GenericWikiAdapter` (`src/adapters/generic.rs`) implementing `VaultAdapter` trait.
- Implemented robust hidden-directory and trash exclusion (`.git`, `.obsidian`, `.trash`, `.k0maru`, `.scratch`) in `src/adapters/mod.rs`.
- Implemented document reading with `pulldown-cmark` H1 title extraction, frontmatter parsing, tag merging, and xxHash64 (`xxh3_64`) dirty-checking hash calculation.
- Implemented hierarchy determination with folder taxonomy heuristics (`20_Cards`/`10_Projects` -> `L3Evergreen`, `01_AI_Logs` -> `L2Log`, `00_Daily` -> `L0Ephemeral`, others -> `L1Resource`) and explicit frontmatter overrides (`hierarchy:`/`level:` keys).
- Implemented non-destructive card writing with safe auto-incremented collision avoidance (e.g. `Title_1.md`).
- Authored comprehensive test suite `tests/test_adapters.rs` with 8 end-to-end tests validating all behaviors.
- Verified 100% test pass (`cargo test`), zero clippy warnings (`cargo clippy --all-targets -- -D warnings`), and clean formatting (`cargo fmt --check`).
