# Ticket 31-2: Distill Engine & FlushEngine Integration

## Context & Goal
Connect the extraction layer with `FlushEngine` to provide a complete pipeline: taking raw trace text, file path, or an offloaded `node_id`, extracting the `DistilledSkill`, synthesizing markdown according to vault conventions, converting to `FlushRequest`, and calling `FlushEngine::flush()` for safe disk write and instant index sync.

## Blocked by
31-1

## Scope of Work
1. **Distill Engine Implementation (`src/distill/engine.rs`)**:
   - `DistillEngine` struct holding `vault_root` and `refs_dir`;
   - `DistillOptions` struct with `title`, `context_hint`, `category`, `tags`, `related_notes`, `dry_run`;
   - Methods:
     - `distill_text(&self, raw_trace: &str, opts: DistillOptions) -> Result<DistillResult, ...>`;
     - `distill_node(&self, node_id: &str, opts: DistillOptions) -> Result<DistillResult, ...>` (reading via `inspect_node`);
     - `distill_file(&self, path: &Path, opts: DistillOptions) -> Result<DistillResult, ...>`.
2. **DistillResult Model**:
   - Contains:
     - `skill: DistilledSkill`
     - `flush_result: FlushResult`
     - `preview_markdown: String`
3. **Markdown Synthesis for Skill**:
   - Integrate structured sections:
     - `# <Title>`
     - `## 🎯 触发上下文与应用场景 (Trigger Context)`
     - `## 🔍 故障根因与错误特征 (Root Cause & Signatures)`
     - `## 🛠️ 修复策略与执行命令 (Remediation & Commands)`
     - `## 🛡️ 防范规约与常青法则 (Prevention Rules & Best Practices)`
   - Auto-weave WikiLinks and YAML frontmatter.
4. **Integration with `FlushEngine`**:
   - Delegate writing and auto-sync (`IncrementalScanner::sync`) to `FlushEngine`;
   - Respect `--dry-run` without touching disk;
   - Return clean result.
5. **Unit & Seam Tests**:
   - Mock vault test: distill from string, verify output file created in `skills/` with right content and format;
   - Mock vault test: distill from node reference in `.scratch/refs/`, verify lookup and crystallization;
   - Test dry run mode.

## Acceptance Criteria
- [x] `DistillEngine` reads from text, node, or file seamlessly;
- [x] Safe write-back creates correctly named markdown file in convention directory;
- [x] Vector & FTS5 cache auto-syncs after write;
- [x] All tests pass.
