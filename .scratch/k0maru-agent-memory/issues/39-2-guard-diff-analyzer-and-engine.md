# Issue 39-2: Guard Diff Analyzer and Engine

**Status**: Completed
**Blocked by**: 39-1

## Goal
Implement the core `GuardEngine` in `crates/k0maru-core/src/guard/engine.rs` that audits git diffs, files, and code hunks against active `GuardRule`s with sub-10ms performance.

## Acceptance Criteria
1. Implement `GuardEngine`:
   - `check_diff(diff_text: &str, rules: &[GuardRule]) -> GuardReport`
   - `check_files(files: &[(PathBuf, String)], rules: &[GuardRule]) -> GuardReport`
2. Evaluates added lines (`+`) in diffs:
   - Matches file path against `rule.file_pattern` (e.g. `*.go`, `*.rs`, `src/**/*`).
   - Detects `Forbidden` regex/string violations, recording exact line numbers.
   - Detects `MustContain` violations for targeted files/functions.
3. Generates clean `GuardReport`:
   - Lists all violations with source ADR note link and remediation guidance.
   - Sets `is_passed: false` if any `Deny` severity violation exists.
4. Comprehensive unit tests covering:
   - Diff with forbidden pattern triggers `Deny`.
   - Diff missing required pattern triggers violation.
   - Clean compliant diff passes in <10ms.
