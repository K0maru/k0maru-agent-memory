# Issue 38-4: Integration and Regression Test Suite for Project Resolution

**Status**: Ready
**Blocked by**: 38-3

## Goal
Construct end-to-end integration tests using mock Obsidian and LLM-Wiki vaults to verify project resolution across all adversarial conditions:
1. High-frequency log clutter (20 AI logs with identical names vs 1 project file).
2. Kebab-case query against Chinese project note with English tag (`topic/funding-arbitrage`).
3. Frontmatter `aliases` and `slug` resolution.
4. Auto-discovered backlink alias resolution.
5. Verification via MCP `get_project_loadout` tool call.

## Acceptance Criteria
1. Integration test reproduces the exact issue reported by user (`okx-funding-arbitrage` resolving to `10_Projects/OKX资金费率套利与跨生态对冲系统.md`).
2. Tests pass in clean tempdir vaults.
3. `cargo test --all-targets` passes with 0 warnings.
4. `cargo clippy --all-targets -- -D warnings` clean.
