# Ticket 31-1: Distill Domain Models, Heuristic Extractor, and Convention NoteCategory::Skill

## Context & Goal
Establish the domain layer and heuristic extraction algorithms for Trace-to-Skill (`src/distill/`). The extractor parses raw execution traces, compiler diagnostics, stack traces, and commands, extracting structured components: Trigger Context, Root Cause/Error Signatures, Remediation Commands, and Prevention Rules. Also add `NoteCategory::Skill` to `src/convention/mod.rs` and `sniffer.rs` for dynamic directory resolution (`skills/`, `playbooks/`, `recipes/`).

## Blocked by
None

## Scope of Work
1. **Extend `NoteCategory` in `src/convention/mod.rs`**:
   - Add `NoteCategory::Skill`;
   - Update `as_str()` -> `"skill"`;
   - Update `from_str_loose()` -> recognize `"skill"`, `"skills"`, `"playbook"`, `"recipe"`, `"troubleshooting"`;
   - Update `probe_category_topology` in `sniffer.rs` to probe `["skills", "playbooks", "recipes", "troubleshooting", "cards"]`.
2. **Domain Models (`src/distill/model.rs`)**:
   - `DistilledSkill` struct with fields:
     - `title: String`
     - `trigger_context: String`
     - `root_cause: String`
     - `remediation: String`
     - `prevention_rules: Vec<String>`
     - `tags: Vec<String>`
     - `related_notes: Vec<String>`
3. **Skill Extractor (`src/distill/extractor.rs`)**:
   - `SkillExtractor::extract(raw_trace: &str, context_hint: Option<&str>, explicit_title: Option<&str>) -> DistilledSkill`;
   - Support pattern matching for:
     - Rust compiler diagnostics (`error[E...]`, `cannot borrow`, `panic`);
     - Python stack traces (`Traceback`, `Exception`, `Error:`);
     - Shell / command line failures (`command not found`, `exit status 1`, `FATAL`);
     - Remediation and success trails;
     - Generate structured Hermes & LLM-Wiki formatted Markdown.
4. **Unit Tests**:
   - Test extraction against realistic Rust compiler error log;
   - Test extraction against Python traceback log;
   - Test fallback behavior when log is ambiguous;
   - Test `NoteCategory::Skill` sniffing.

## Acceptance Criteria
- [x] `NoteCategory::Skill` serializes and maps to `"skill"`, and sniffs `skills/` directory;
- [x] `SkillExtractor` extracts 4 core sections from varied logs;
- [x] Unit tests pass 100% with `cargo test`.
