# Ticket 33-1: Hermes Agent Dynamic Skill Schema & Frontmatter Formatter

## Context & Goal
Align K0maru's dynamic experience extraction (`src/distill/`) with Nous Research Hermes Agent's native skill specification. When distilling skills for Hermes (`--target hermes`), the output must synthesize YAML frontmatter strictly compliant with Hermes dynamic tool calling conventions (`name`, `description`, `trigger`, `parameters`, `tags`), allowing Hermes models to treat the crystallized card not only as an advisory note, but as a runnable, dynamically discoverable skill.

## Blocked by
None

## Scope of Work
1. **Extend Distill Domain Models (`src/distill/model.rs`)**:
   - Add `DistillTarget` enum: `Default`, `Hermes`;
   - Implement `FromStr` and `Display` for `DistillTarget`;
   - Extend `DistilledSkill` with a method `to_hermes_markdown(&self) -> String`:
     - Synthesize Hermes standard YAML frontmatter:
       ```yaml
       ---
       name: <sanitized_kebab_slug>
       description: <one_line_summary>
       trigger: <trigger_context_or_signature>
       tags: [<tags>]
       parameters:
         type: object
         properties: {}
       ---
       ```
     - Follow with the 4 core sections in clean Markdown:
       - `## 🎯 触发上下文与应用场景 (Trigger Context)`
       - `## 🔍 故障根因与错误特征 (Root Cause & Signatures)`
       - `## 🛠️ 修复策略与执行命令 (Remediation & Commands)`
       - `## 🛡️ 防范规约与常青法则 (Prevention Rules & Best Practices)`
2. **Integration in `DistillEngine` (`src/distill/engine.rs`)**:
   - Add `target: DistillTarget` to `DistillOptions`;
   - In `DistillEngine::distill_text`, `distill_node`, and `distill_file`, honor `opts.target` when generating the skill preview and content;
3. **Unit Tests (`tests/test_distill_hermes.rs`)**:
   - Verify `to_hermes_markdown()` contains valid YAML frontmatter matching Hermes schema;
   - Verify frontmatter fields `name`, `description`, `trigger`, `parameters`, `tags`;
   - Verify non-empty sections and valid markdown syntax.

## Acceptance Criteria
- [ ] `DistillTarget::Hermes` produces YAML frontmatter containing required Hermes fields;
- [ ] `DistillEngine` respects `target: DistillTarget::Hermes`;
- [ ] Unit tests pass 100% with `cargo test`.
