# 07 — Sub-300-token context loadout builder and CLI command

**What to build:** The `k0maru loadout <project>` CLI command and builder engine in `src/loadout/builder.rs` extracting project vision, linked L3 evergreen rules, and recent L2 logs into a compact, high-signal `< 300 Token` prompt package, with support for `--copy` via `arboard` and `--json`.

**Blocked by:** 06 — Incremental file scanner and state synchronization engine

**Status:** resolved

- [x] `LoadoutBuilder` matches target project from `10_Projects/` or wiki root using fuzzy and exact string matching
- [x] Extracts project scope/vision statement from project markdown header or blockquote
- [x] Resolves linked L3 cards (up to 5 cards) and extracts their 1-sentence core concepts
- [x] Resolves linked L2 logs (up to 3 recent logs) and extracts their identifiers
- [x] Formats final loadout into compact YAML/Markdown adhering strictly to the <300 token budget
- [x] CLI command `k0maru loadout <query>` prints loadout to stdout
- [x] CLI flag `--json` returns structured JSON output for Agent consumption
- [x] CLI flag `--copy` copies output to clipboard using `arboard`
- [x] `tests/test_loadout.rs` validates matching accuracy, token length bounds, and CLI output

## Completion Summary
- Implemented `LoadoutBuilder` in `src/loadout/builder.rs` with `src/loadout/mod.rs` and re-exported in `src/lib.rs`.
- Supported exact, prefix, substring, and subsequence fuzzy matching across `10_Projects/` and generic flat wikis with SQLite FTS5 fallback.
- Implemented robust project vision extraction (supporting blockquotes, `> [!NOTE]` callouts, and header intro paragraphs).
- Implemented L3 evergreen card resolution (up to 5 cards) with 1-sentence core concept extraction (prioritizing `## 📌 核心概念` blockquote or first blockquote).
- Implemented L2 AI log resolution (up to 3 recent logs).
- Formatted high-density, compact Markdown/YAML loadout with strict sub-300 token budget (measured via `estimate_tokens` and character bounds).
- Integrated `k0maru loadout <QUERY>` CLI command with `--vault`, `--copy` (`arboard`), `--json`, and `--list` flags in `src/main.rs`.
- Comprehensive TDD integration test suite written in `tests/test_loadout.rs` (9 test cases, 100% pass).
- Passed `cargo clippy --all-targets -- -D warnings` with zero warnings and `cargo fmt --check`.
