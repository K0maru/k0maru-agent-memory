# 26 — Open-Source Bilingual Documentation Baseline (English Primary README & Chinese Mirror)

**What to build:**
Establish an international open-source documentation baseline by setting the primary repository `README.md` to idiomatic English and maintaining `README_zh.md` as the official Chinese mirror, equipped with bidirectional language switcher links.

**Blocked by:** 25 (Hide Graph Explorer Tab)

**Status:** resolved

- [x] Create Chinese Mirror `README_zh.md`:
  - Preserve the complete Chinese README documentation.
  - Add top language navigation: `[English](README.md) | [简体中文](README_zh.md)`.
  - Ensure all relative image paths (`docs/images/ui-search.png`) and links (`docs/UI_TUTORIAL.md`, `QUICKSTART.md`) resolve correctly.
- [x] Create Primary English `README.md`:
  - Craft a world-class, professional technical English README adhering to top-tier open-source standards.
  - Add top language navigation: `[English](README.md) | [简体中文](README_zh.md)`.
  - Include:
    - Badges: Rust 2021, MIT License, Version v0.4.2, 145 Tests Passed, Binary Size 3.66MB, Cold Start 3.4ms, Zero-Daemon.
    - Motivation & Design Philosophy: Cross-session amnesia, terminal log bloat, zero-daemon vs heavy Docker microservices, Bring Your Own Markdown, disposable transient SQLite cache (`cache.sqlite`).
    - Core Capabilities:
      1. Project-level Rapid Context Loadout (`k0maru loadout` <300 tokens, `--copy`).
      2. Symbolic Terminal Log Offloading (`k0maru offload` & `inspect` with Mermaid pipeline charts).
      3. Disposable Vector Cache & Hybrid Search (`k0maru search` with BM25 + ONNX embeddings + RRF $k=60$ + 1-hop Graph Boost +0.05).
      4. Embedded Developer Console (`k0maru ui` with Search Debugger screenshot `docs/images/ui-search.png`, Log Inspector, Token Scoreboard, and bilingual switcher).
      5. FastMCP Stdio Server: Zero-daemon Model Context Protocol integration, natural language intent-driven tool calling (no rigid magic words), and `AGENTS.md` system prompt best practices.
      6. Incremental File Scanner (`k0maru sync` with `xxh3` dirty state machine).
    - Mermaid Architecture Diagrams: 4-layer decoupled engine and memory lifecycle loop.
    - CLI Reference table & Quick Start snippet.
    - License & Clean-room attribution.
- [x] Quality Assurance:
  - All relative links and image paths in both `README.md` and `README_zh.md` are valid and functional.
  - Full test suite passes: `cargo test` (145 tests).
  - Code hygiene: `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.

## Deliverables Summary
1. `README.md`: World-class, idiomatic English primary README with top language switcher, badges, comprehensive architectural motivation, core capabilities with terminal examples, Mermaid lifecycle & architecture diagrams, comparison matrix, quantitative benchmarks, FastMCP prompt guides, and CLI cheat sheet.
2. `README_zh.md`: Complete Chinese mirror documentation with top language switcher and updated version tags.
3. `QUICKSTART.md`: Added top bilingual navigation switcher pointing to `README.md` and `README_zh.md`.
4. Verification: All 145 unit, integration, and E2E tests pass cleanly. `cargo clippy` and `cargo fmt --check` pass with zero warnings. All relative markdown links and image paths across documentation verified.
