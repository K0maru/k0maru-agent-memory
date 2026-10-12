# Issue 39-1: Guard Core Models and ADR Invariant Extractor

**Status**: Completed
**Blocked by**: None

## Goal
Implement domain models (`GuardRule`, `RuleCondition`, `RuleSeverity`, `GuardViolation`, `GuardReport`) and the ADR invariant extractor in `crates/k0maru-core/src/guard/`.

## Acceptance Criteria
1. Define types in `crates/k0maru-core/src/guard/models.rs`:
   - `RuleSeverity` (`Deny`, `Warn`)
   - `RuleCondition` (`MustContain(String)`, `Forbidden(String)`)
   - `GuardRule`, `GuardViolation`, `GuardReport`
2. Implement extractor in `crates/k0maru-core/src/guard/extractor.rs`:
   - `extract_rules_from_document(doc: &Document) -> Vec<GuardRule>`
   - Parses YAML frontmatter `invariants` array.
   - Extracts rules from markdown body sections (`## Non-Negotiable Invariants` or `## 核心工程铁律`).
   - Supports reading `.k0maru/guard.toml` or `guard.yaml` from vault root.
3. Unit tests covering:
   - Frontmatter parsing with YAML invariants.
   - Body heuristic rule extraction.
   - Vault guard config file fallback.
4. `cargo test -p k0maru-core` passes 100%.
