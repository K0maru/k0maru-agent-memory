# Issue 38-2: Project Identifier Normalizer and Multi-Dimensional Alias Extraction

**Status**: Ready
**Blocked by**: None

## Goal
Implement a robust identifier normalizer and metadata extractor in `src/loadout/` that extracts explicit aliases, slug, repo, and tags from documents, normalizes them for fuzzy/token matching, and extracts derived aliases from backlink logs.

## Acceptance Criteria
1. Define `ProjectMetadata`:
   - `path: PathBuf`
   - `stem: String`
   - `title: String`
   - `aliases: Vec<String>` (from `frontmatter.aliases`, `frontmatter.alias`, `frontmatter.slug`, `frontmatter.repo`)
   - `tags: Vec<String>` (stripped of `topic/`, `type/`, `project/` prefixes into raw tokens)
   - `derived_aliases: Vec<String>` (harvested from linking documents with `YYYY-MM-DD-` prefixes)
2. Normalization function `normalize_identifier(s: &str) -> String`:
   - Lowercases, replaces hyphens and underscores with spaces, removes common punctuation.
3. Matching helper `is_project_match(query: &str, meta: &ProjectMetadata) -> bool`:
   - Tests exact, prefix, substring, token subset, and slug equality.
4. Comprehensive unit tests covering kebab-case queries against Chinese documents with English tags/aliases.
