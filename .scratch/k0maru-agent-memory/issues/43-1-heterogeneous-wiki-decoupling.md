# Ticket 43-1: Heterogeneous LLM-Wiki Decoupling Across Adapters, Guard, Loadout, Sniffer & Doctor

- **Status**: `done`
- **Blocked by**: None (Parallel Slice 1)

## Target Files
- `crates/k0maru-core/src/adapters/obsidian.rs`
- `src/convention/sniffer.rs`
- `src/guard/mod.rs`
- `src/loadout/builder.rs`
- `src/doctor/mod.rs`
- `tests/test_guard.rs`
- `tests/test_loadout.rs`

## Acceptance Criteria
- [x] `ObsidianAdapter::determine_hierarchy` recognizes heterogeneous folder names (`cards`, `concepts`, `decisions`, `adr`, `wiki`, `rules`, `skills`, `playbooks` -> `L3Evergreen`; `logs`, `ai_logs`, `journal`, `records`, `30_logs` -> `L2Log`; `daily`, `dailies`, `inbox`, `fleeting` -> `L0Ephemeral`; `projects`, `1_projects`, `resources`, `docs` -> `L1Resource`) in addition to Johnny.Decimal folders.
- [x] `ConventionSniffer::probe_category_topology` includes `"20_Cards"`, `"cards"` in `decision_candidates`, `"01_AI_Logs"`, `"30_Logs"` in `log_candidates`, and `"20_Cards"` in `concept_candidates`.
- [x] `resolve_guard_rules` in `src/guard/mod.rs` scans all markdown files in the vault via `adapter.scan_files()` (excluding `00_Daily`, `daily`, `01_AI_Logs`, `logs`, `journal` ephemeral/log dirs) so Living ADR rules in flat wikis, `decisions/`, `docs/adr/`, `cards/`, or root notes are discovered automatically.
- [x] `LoadoutBuilder` in `src/loadout/builder.rs` discovers projects in `10_Projects/`, `projects/`, `1_Projects/`, or via tags/frontmatter/flat files, and `resolve_l3_card` / `resolve_l2_log` resolve links across `cards/`, `decisions/`, `adr/`, `concepts/`, `notes/`, `wiki/`, `logs/`, `journal/`, or any matching file stem in the vault.
- [x] `probe_vault` in `src/doctor/mod.rs` reports `PASS` for both structured vaults (`10_Projects`, `decisions`, `cards`, `notes`, `docs`, etc.) and flat LLM-Wikis (`Flat LLM-Wiki topology`) whenever markdown notes exist.
