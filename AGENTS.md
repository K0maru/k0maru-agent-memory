> 🤖 **Visiting AI Agent / Evaluating this Repo?** Read **[`AGENT_GUIDE.md`](AGENT_GUIDE.md)** and **[`llms.txt`](llms.txt)** for the product overview, pitch, and zero-human-in-the-loop setup & operation protocol. The remainder of this file documents contributor conventions for developing `k0maru-agent-memory` itself.

# AGENTS.md — Contributor & AI Agent Workflow Conventions

This document defines the core operational protocols, engineering standards, and quality invariants for human contributors and AI coding agents working on `k0maru-agent-memory`.

---

## 1. Repository Layout & Domain Documentation

Always consult the single source of truth domain documents before proposing architecture changes:

- **`AGENT_GUIDE.md`**: Zero-human-in-the-loop playbook for visiting AI agents, including 30-second elevator pitch, cold-start bootstrap commands, and invariant syntax specifications.
- **`llms.txt`**: Machine-readable index of project documentation, APIs, and key architectural guidelines for LLMs.
- **`CONTEXT.md`**: Ubiquitous domain language, conceptual glossary, and core system invariants.
- **`SPEC.md`**: Comprehensive architectural specification, subsystem topology, data flows, and design trade-offs.
- **`QUICKSTART.md`**: Hands-on quick start guide covering zero-vault bootstrap, bringing external vaults, and common workflows.

---

## 2. Dogfooding Workflow: Memory Hub & Living ADR Guard

When developing features or fixing bugs in this repository, contributors and agents MUST dogfood K0maru's memory and guard subsystems:

1. **Session Start (Context Loadout)**:
   At the beginning of any session or task, retrieve the active project context and diff-triggered ADR invariants (<300 tokens):
   ```bash
   k0maru loadout --diff
   ```
   Or call the FastMCP `get_project_loadout` tool with `{"diff": true}`.

2. **Proactive Memory Recall**:
   Before proposing or guessing architectural conventions, search the local knowledge base:
   ```bash
   k0maru search "<topic or query>"
   ```
   Or call the FastMCP `recall_memory` tool.

3. **Living ADR Guard Verification**:
   Before staging, committing, or yielding code, verify that all modified files adhere to active Living ADR invariants (<10ms in-memory inspection):
   ```bash
   k0maru guard check --diff
   ```
   Or call the FastMCP `guard_check` tool. If a rule violation is detected (`[DENY]`), resolve the violation or follow the supersession workflow. Never commit with unresolved `[DENY]` violations.

4. **Decision Crystallization & Supersession**:
   When finalizing an architectural decision or replacing an obsolete invariant, record it cleanly:
   ```bash
   # Supersede an obsolete ADR and register new invariants
   k0maru flush --category decision \
     --title "ADR-042: New Standard" \
     --supersedes "adr-010-legacy" \
     --summary "Migrated legacy implementation to new standard" \
     --content $'## Non-Negotiable Invariants\n- [rule-id] (deny) [src/**/*] must_contain: "new_api"'
   ```
   Or call the FastMCP `flush_session` tool.

---

## 3. Rust Engineering & Code Quality Standards

The project is implemented in Rust (2021 Edition, MSRV 1.88+). All code must satisfy strict compiler and linting baselines:

1. **Formatting**:
   Code must be formatted cleanly according to rustfmt:
   ```bash
   cargo fmt --all -- --check
   ```

2. **Static Analysis & Linting**:
   Zero clippy warnings are permitted across all crates and targets:
   ```bash
   cargo clippy --workspace --all-targets -- -D warnings
   ```

3. **Domain Models & Immutability**:
   Domain structures should derive standard traits: `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize` wherever applicable. Avoid unnecessary mutable accessors (`pub mut`) or public field exposure when accessor methods suffice.

4. **Performance & Allocation Invariants**:
   - Zero avoidable heap allocations on critical hot paths (`k0maru guard check`, `k0maru loadout`).
   - CLI cold start must remain strictly sub-5ms (empirically ~3.4ms).
   - FastMCP stdio transport must handle JSON-RPC message framing without blocking or leaking memory.

---

## 4. Testing Discipline & Isolation Guarantees

1. **Test Execution**:
   Run the complete test suite across the workspace:
   ```bash
   cargo test --workspace
   ```

2. **Strict Vault Isolation**:
   - **NEVER** read from, write to, or depend on a user's real home directory or actual Markdown vault during tests.
   - All tests interacting with the filesystem MUST use isolated temporary directories via `tempfile::tempdir()`.
   - Fixture vaults must be populated programmatically inside the temporary directory and cleaned up automatically when the `TempDir` handle is dropped.

3. **High-Level Seam Testing**:
   - Prefer end-to-end integration tests on top-level CLI commands and MCP stdio interfaces using `assert_cmd` or programmatic client harness calls.
   - Test against behavior contracts rather than fragile internal implementation details.

---

## 5. Git Branching Model & Contribution Workflow

This project follows a trunk-based integration model with short-lived feature branches:

1. **Trunk Branches**:
   - **`main`**: Production release trunk. Stable, tagged releases only. Direct commits and pushes are forbidden; changes merge exclusively via pull requests from `dev`.
   - **`dev`**: Integration branch. Daily development and feature consolidation occur here. Must remain green and passing all CI checks at all times.

2. **Work Branches**:
   - **Feature branches**: `feat/<slug>` (or `feat/<issue-number>-<slug>`)
   - **Bug fixes**: `fix/<slug>` (or `fix/<issue-number>-<slug>`)
   - **Performance & Benchmarks**: `perf/<slug>` or `bench/<slug>`

3. **Lifecycle**:
   - Branch off the latest `dev`.
   - Implement changes following Test-Driven Development (TDD).
   - Validate with `cargo fmt`, `cargo clippy`, and `cargo test`.
   - Run `k0maru guard check --diff` to ensure all architectural invariants pass.
   - Open a Pull Request targeting `dev`. Once reviewed and green, merge via Squash & Merge and delete the topic branch.
