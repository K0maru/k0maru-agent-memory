# Ticket 33-2: Hermes Skill CLI `--target hermes` & FastMCP Export

## Context & Goal
Expose Hermes skill alignment through the public user interfaces: the `k0maru distill` CLI command (accepting `--target hermes` and `--export-hermes` for direct injection into `~/.hermes/skills/`) and the FastMCP `distill_session_skill` tool (with optional `target: "hermes"` parameter).

## Blocked by
33-1

## Scope of Work
1. **CLI `k0maru distill` Extension (`src/main.rs`)**:
   - Add `#[arg(long, default_value = "default")] pub target: String` to `DistillArgs`;
   - Add `#[arg(long)] pub export_hermes: bool` to `DistillArgs`;
   - When `--export-hermes` is set:
     - Locate Hermes skill directory via `$HERMES_HOME/skills` or `~/.hermes/skills` (or fallback to `<vault>/skills`);
     - Atomically write the Hermes formatted skill card into the Hermes skill directory;
     - Confirm exported path in console output and JSON response;
2. **FastMCP Server Registration (`src/mcp/server.rs`)**:
   - In `distill_session_skill` tool registration:
     - Add optional parameter `target`: string (`"default"` or `"hermes"`);
   - In `call_distill_session_skill`:
     - Parse `target` and pass to `DistillOptions`;
3. **Integration Tests (`tests/test_distill_hermes.rs`)**:
   - CLI execution test: piping error log into `k0maru distill --target hermes --dry-run` and asserting Hermes YAML frontmatter;
   - CLI execution test: `k0maru distill --target hermes --export-hermes --home <sandbox>`;
   - FastMCP test calling `distill_session_skill` with `target: "hermes"`.

## Acceptance Criteria
- [ ] `k0maru distill --target hermes --dry-run` emits Hermes-compliant skill card;
- [ ] `k0maru distill --export-hermes` writes into Hermes skills directory respecting `--home`;
- [ ] FastMCP `distill_session_skill` supports `target: "hermes"`;
- [ ] Full test suite passes cleanly with zero warnings.
