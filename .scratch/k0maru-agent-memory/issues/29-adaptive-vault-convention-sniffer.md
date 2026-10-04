# 29 — Adaptive Vault Convention Sniffer

**Type:** task  
**Status:** resolved  
**Blocked by:** none  

## Context
Implement the adaptive vault convention sniffer to inspect a target `vault_root` dynamically. The sniffer extracts explicit rules (from `AGENTS.md`, `RULES.md`, `templates/`) and infers implicit organization patterns (target subdirectories, filename naming styles, and YAML Frontmatter conventions) without hardcoding any specific directory structure.

## Objectives & Deliverables
1. **Domain Models (`src/convention/mod.rs`)**:
   - `NamingStyle` enum: `DateSlug` (`YYYY-MM-DD-title.md`), `TimestampSlug`, `SlugOnly`, `TitleCase`.
   - `NoteCategory` enum / string mapping: `Decision`, `Log`, `Concept`, `Generic`.
   - `VaultConvention` struct:
     - `vault_root: PathBuf`
     - `has_explicit_rules: bool`
     - `rule_source: Option<PathBuf>`
     - `target_subdirs: HashMap<String, PathBuf>`
     - `naming_style: NamingStyle`
     - `has_frontmatter: bool`
     - `template: Option<String>`
   - `FlushRequest` and `FlushResult` structs.
2. **Sniffer Engine (`src/convention/sniffer.rs`)**:
   - `ConventionSniffer::sniff(vault_root: &Path) -> VaultConvention`:
     - Checks for `AGENTS.md`, `CLAUDE.md`, `RULES.md`, `CONVENTIONS.md`, `.k0maru/rules.md`.
     - Checks for `templates/` or `_templates/` or `.obsidian/templates/`.
     - Analyzes existing directories to find natural homes for `decision` (`decisions`, `adr`, `docs/adr`) and `log` (`logs`, `journal`, `daily`).
     - Defaults cleanly to `vault_root` if vault is flat or unclassified.
     - Inspects existing files in the target directory to determine dominant `NamingStyle` and whether Frontmatter is customary.
   - Slugify and filename generation helper: `generate_filename(title: &str, style: &NamingStyle, date: Option<NaiveDate>) -> String`.
3. **Markdown Synthesizer (`src/convention/synthesizer.rs`)**:
   - Assembles note content combining YAML frontmatter, substituted template or standard clean layout, and auto-generated `[[WikiLinks]]` for related references.
4. **Export in `src/lib.rs`**:
   - Export `pub mod convention;` and re-export key types.
5. **Testing (`tests/test_convention_sniffer.rs`)**:
   - Test flat vault (Karpathy LLM-Wiki style): verifies files target root with appropriate slug.
   - Test structured vault with existing `logs/` and `decisions/`: verifies correct subdirectory routing.
   - Test vault with explicit template in `templates/log.md`: verifies template extraction.
   - Test filename style deduction (date-prefixed vs slug-only).
   - Test path traversal protection: verifies sniffer refuses to route outside `vault_root`.
6. **Acceptance Criteria**:
   - 100% test pass rate on new and existing suites.
   - `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean.
