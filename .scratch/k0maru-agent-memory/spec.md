# Spec 43: Heterogeneous LLM-Wiki Decoupling & Memory Packs (`k0maru pack`)

## 1. Goal & Architectural Invariants

1. **Zero Hard-Binding to `SecondBrain` (`10_Projects` / `20_Cards` / `01_AI_Logs` / `00_Daily`)**:
   - `k0maru-agent-memory` MUST treat all LLM-Wiki topologies as first-class citizens:
     - **Flat Karpathy-style LLM-Wiki** (notes in root directory, linked via `[[WikiLinks]]`);
     - **Standard Engineering Docs / ADR Wikis** (`docs/adr/`, `decisions/`, `notes/`, `wiki/`, `logs/`);
     - **PARA / Zettelkasten Wikis** (`projects/`, `1_Projects/`, `cards/`, `concepts/`, `journal/`, `daily/`);
     - **Johnny.Decimal / Obsidian SecondBrain** (`10_Projects/`, `20_Cards/`, `01_AI_Logs/`, `00_Daily/`).
2. **Topology-Agnostic Memory Packs (`k0maru pack export / import / inspect`)**:
   - Export domain knowledge, L3 cards, skills, and Living ADR `invariants` from any source vault into a portable `.pack.json` bundle, stripping source-specific folder prefixes from internal `[[WikiLinks]]`.
   - Import a `.pack.json` into any target vault by sniffing the target vault's conventions (`ConventionSniffer::sniff`), placing notes into the target vault's natural subdirectories (or root for flat wikis) using the target's naming style (`SlugOnly`, `TitleCase`, `DateSlug`, `TimestampSlug`), and syncing SQLite FTS5 + vector indices.

---

## 2. Parallel Slices (Zero File Overlap)

- **Ticket 43-1 (`43-1-heterogeneous-wiki-decoupling.md`)**:
  - Files: `crates/k0maru-core/src/adapters/obsidian.rs`, `src/convention/sniffer.rs`, `src/guard/mod.rs`, `src/loadout/builder.rs`, `src/doctor/mod.rs`, `tests/test_guard.rs`, `tests/test_loadout.rs`.
- **Ticket 43-2 (`43-2-memory-pack-core-engine.md`)**:
  - Files: `src/pack/mod.rs` (new module).
- **Ticket 43-3 (`43-3-pack-cli-and-e2e-tests.md`)**:
  - Files: `src/lib.rs`, `src/main.rs`, `tests/test_pack.rs` (new test file).
