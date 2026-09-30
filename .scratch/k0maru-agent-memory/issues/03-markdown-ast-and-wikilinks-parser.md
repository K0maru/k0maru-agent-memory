# 03 — Robust Markdown AST, Frontmatter, and WikiLinks parser

**What to build:** High-performance Markdown parser in `src/parser/markdown.rs` extracting YAML Frontmatter, bare `[[Card]]` and aliased `[[Card|Alias]]` links, and hierarchical `#tags`, while rigorously ignoring links inside code blocks (fenced ``` and inline `) using `pulldown-cmark`.

**Blocked by:** 02 — Core domain models, traits, and test vault fixtures

**Status:** resolved

- [x] `parse_frontmatter` splits YAML metadata from markdown body, gracefully handling missing or invalid YAML without panics
- [x] `extract_wikilinks` extracts bare `[[Card]]` and aliased `[[Card|Alias]]` into `Vec<WikiLink>`
- [x] Pseudo-wikilinks inside fenced code blocks (```) and inline code (`) are strictly ignored
- [x] `extract_tags` captures `#tag` and `#tag/subtag` structures while ignoring markdown headers (`# Header`) and URL hashes
- [x] `tests/test_parser.rs` passes 100% boundary tests (nested brackets, aliases, special characters, unicode, code blocks)
- [x] Benchmark or test verifies parsing of 1,000 documents completes in <50ms

## Completion Summary
- Implemented `parse_frontmatter` in `src/parser/markdown.rs` supporting UTF-8 BOM, standard YAML Frontmatter, CRLF line endings, and safe fallback on missing or malformed YAML without panic.
- Implemented `extract_wikilinks` with AST-level exclusion using `pulldown-cmark` ranges to strictly ignore code blocks (fenced and indented) and inline code (`...`), supporting bare, aliased (`[[Card|Alias]]`), relative paths (`[[01_AI_Logs/...]]`), Unicode/Chinese characters, and `.md` extension stripping.
- Implemented `extract_tags` supporting hierarchical tags (`#tag/subtag`), Unicode/Chinese tags, while ignoring Markdown ATX headings (`# Header`), URL anchors (`https://example.com#hash`), and tags within code blocks.
- Added `extract_elements` for high-throughput single-pass extraction of both links and tags.
- Authored 20 comprehensive TDD test cases in `tests/test_parser.rs` with 100% pass rate.
- Microbenchmark verified parsing 1,000 documents in ~37ms (unoptimized debug) and ~6.7ms (optimized release), well beneath the <50ms threshold.
- `cargo clippy --all-targets -- -D warnings` zero warnings; `cargo fmt --check` cleanly formatted.
