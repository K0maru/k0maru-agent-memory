# Ticket 43-2: Implement `src/pack/mod.rs` Memory Pack Export, Import & Inspect Engine

- **Status**: `done`
- **Blocked by**: None (Parallel Slice 2)

## Target Files
- `src/pack/mod.rs` (new file)

## Acceptance Criteria
- [x] Defines `MemoryPack`, `PackNote`, `PackExportOptions`, `PackImportOptions`, `PackImportResult`, `PackInspectReport`.
- [x] `export_pack(options: &PackExportOptions) -> Result<MemoryPack, Box<dyn std::error::Error>>`: scans source vault, filters notes by `tag`, `query`, or explicit `files`, extracts `GuardRule`s from matching documents, and strips folder prefixes from `[[WikiLinks]]` (e.g., `[[20_Cards/Foo|Bar]]` -> `[[Foo|Bar]]`, `[[10_Projects/Baz]]` -> `[[Baz]]`).
- [x] `import_pack(pack: &MemoryPack, options: &PackImportOptions) -> Result<PackImportResult, Box<dyn std::error::Error>>`: uses `FlushEngine::new(&options.vault_path)` (which runs `ConventionSniffer::sniff`) to write each `PackNote` into the target vault's natural category directory and naming style (preserving YAML frontmatter `invariants` when present so `k0maru guard` works in the target vault).
- [x] `inspect_pack(pack: &MemoryPack, target_vault: Option<&Path>) -> PackInspectReport`: summarizes pack metadata, note count, guard rule count, and predicted target paths.
