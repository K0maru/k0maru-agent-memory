# Specification: Open-Source Bilingual Documentation Baseline

## Problem Statement

K0maru-Agent-Memory is positioning itself as a world-class open-source memory hub for AI coding agents. Currently, the root `README.md` is written exclusively in Chinese. 

For the global developer community:
1. **International Accessibility Barrier**: Global developers navigating to `https://github.com/K0maru/k0maru-agent-memory` immediately encounter Chinese text, creating high friction for adoption, evaluation, and community contributions.
2. **Open-Source Standard Conformity**: Top-tier open-source projects default to English in the primary `README.md` while providing localized mirrors (e.g. `README_zh.md`, `README_ja.md`) with prominent bilingual switcher pills at the top.
3. **Consistency Across Assets**: Visual screenshots in `docs/images/` and visual tutorials in `docs/UI_TUTORIAL.md` must be seamlessly linked and referenced in both English and Chinese entry points.

## Solution

Implement the standard open-source bilingual documentation baseline:

1. **Chinese Version Migration (`README_zh.md`)**:
   - Move/copy the existing comprehensive Chinese documentation to `README_zh.md`.
   - Add language switcher header at the very top:
     ```markdown
     [English](README.md) | [简体中文](README_zh.md)
     ```
   - Ensure all relative image links (`docs/images/ui-search.png`) and document links (`docs/UI_TUTORIAL.md`, `QUICKSTART.md`) resolve cleanly.

2. **Primary English `README.md` (Global Standard)**:
   - Construct an idiomatic, professional, technical English `README.md` mirroring all core sections:
     - **Hero Section**: High-impact badges (Rust 2021, MIT License, v0.4.2, 145 tests passed, binary size 3.66MB, cold start 3.4ms, zero-daemon).
     - **Motivation & Design Principles**: Cross-session amnesia, terminal log bloat, zero-daemon vs heavy Docker microservices, Bring Your Own Markdown, disposable transient SQLite cache (`cache.sqlite`).
     - **Core Capabilities**:
       1. `k0maru loadout`: Project-level rapid context loading (<300 tokens, `--copy`).
       2. `k0maru offload` & `inspect`: Symbolic terminal log offloading with Mermaid state charts.
       3. `k0maru search`: Hybrid retrieval (BM25 + `sqlite-vec` ONNX embeddings + RRF $k=60$ + 1-hop WikiLinks Graph Boost +0.05).
       4. `k0maru ui`: Embedded Developer Console with Search Debugger screenshot (`docs/images/ui-search.png`), Log Inspector, Token Scoreboard, and bilingual switcher.
       5. FastMCP Stdio Server: Zero-daemon Model Context Protocol integration, natural language intent-driven tool calling (no rigid magic words), and `AGENTS.md` system prompt best practices.
       6. `k0maru sync`: Incremental file scanning with `xxh3` dirty state machine.
     - **Quantitative Benchmarks**: Performance metrics (TRR 99.44%, cold start 3.4ms, memory ~11MB).
     - **Architecture Diagram**: Mermaid 4-layer decoupled engine.
     - **CLI Cheat Sheet & Quick Start**: Installation, basic usage, client configuration.
     - **License & Acknowledgments**: MIT License, clean-room implementation attribution.

3. **Validation & Integrity**:
   - Verify all markdown links are valid.
   - Verify `cargo test` and build remain unaffected.
