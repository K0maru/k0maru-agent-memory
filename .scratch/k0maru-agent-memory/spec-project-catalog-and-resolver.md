# SPEC: First-Class Project Catalog & Multi-Tier Scoped Resolver (Phase 9.1)

## 1. Background & Problem Statement

In real multi-agent coding workflows, calling `get_project_loadout(project_name: "<name>")` frequently fails with:
`Error: Project matching '<name>' not found in vault.`

### Identified Root Causes
1. **Post-Filter Truncation Anti-Pattern in FTS**:
   `search_fts(trimmed, 5)` executed an unconstrained global query across all notes in the vault (`00_Daily/`, `01_AI_Logs/`, `20_Cards/`, etc.) with a hardcoded `LIMIT 5`.
   Because project keywords frequently appear in numerous session logs (`01_AI_Logs/`), these log files monopolize the top 5 positions by BM25 score. The actual project file (e.g. `10_Projects/OKX资金费率套利与跨生态对冲系统.md`) ranked at #21.
   Then in memory, `paths.contains(&m.path)` rejected all top 5 logs because `paths` only contained candidates in `10_Projects/`.
2. **Lack of Identifier Normalization**:
   Agents often pass kebab-case slugs (e.g., `okx-funding-arbitrage`) matching git repositories or topic tags, while vault notes use descriptive human-readable titles (e.g., `OKX资金费率套利与跨生态对冲系统`).
   Existing matching only tested exact, prefix, substring, and subsequence matching without normalizing punctuation, casing, or extracting slug tokens.
3. **Unused Vector Embeddings**:
   `sqlite-vec` embeddings were not utilized for project discovery, missing cross-lingual (English slug to Chinese note) semantic matching.
4. **Ignored Metadata & Backlinks**:
   Frontmatter `aliases`, `slug`, `repo`, and `tags` (e.g., `topic/funding-arbitrage`) were not checked by `find_project`. Furthermore, backlink naming patterns from logs linking to the project were not harvested as derived aliases.

---

## 2. Architecture & Design

### 2.1 Four-Tier Resolution Funnel

```text
Query: "okx-funding-arbitrage" (or CWD)
  │
  ├── Tier 1: Normalized Exact, Alias & Tag Match (0ms)
  │     - Slug normalization (lowercase, remove hyphens/underscores/spaces, or tokenize)
  │     - Matches against:
  │         * Stem (exact, prefix, substring, normalized token subset)
  │         * H1 Title
  │         * Frontmatter `aliases` / `alias`
  │         * Frontmatter `slug` / `repo`
  │         * Tags (e.g. `topic/funding-arbitrage` ➔ `funding-arbitrage`)
  │         * Auto-discovered aliases from backlink logs
  │
  ├── Tier 2: Scoped FTS5 SQL Push-Down (<1ms)
  │     - Push scope constraint directly into SQLite SQL:
  │       WHERE documents_fts MATCH :sanitized_query
  │         AND (d.path LIKE '10_Projects/%' OR d.hierarchy = 'L1Resource')
  │       ORDER BY rank LIMIT 5;
  │     - Never pollutes project rank with daily logs or historical notes.
  │
  ├── Tier 3: Vector Semantic Fallback (<5ms)
  │     - Query embedding compared against candidate projects.
  │     - Cosine similarity threshold > 0.65 resolves cross-lingual / conceptual queries.
  │
  └── Tier 4: Ambient Workspace Resolution
        - If query is empty or matches git root folder of current workspace, resolve to associated project.
```

### 2.2 Storage Layer: `search_scoped_fts` in `SqliteStorage`
Add a scoped search function in `k0maru-core::storage::sqlite`:
```rust
impl SqliteStorage {
    pub fn search_scoped_fts(
        &self,
        query: &str,
        path_prefix: Option<&str>,
        hierarchy: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error>>;
}
```
Constructs SQL:
```sql
SELECT d.path, d.title, d.hierarchy, d.frontmatter, d.content_hash, d.mtime, d.body
FROM documents_fts
JOIN documents d ON d.path = documents_fts.path
WHERE documents_fts MATCH ?1
  AND (?2 IS NULL OR d.path LIKE ?2 || '%')
  AND (?3 IS NULL OR d.hierarchy = ?3)
ORDER BY rank
LIMIT ?4
```

### 2.3 Domain Layer: Project Metadata & Alias Extraction
Enhance `ProjectInfo`:
```rust
pub struct ProjectMetadata {
    pub path: PathBuf,
    pub stem: String,
    pub title: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub slug: Option<String>,
}
```
Helper `extract_project_metadata(&doc) -> ProjectMetadata`:
- Extracts `aliases` (list or comma-separated string) from frontmatter.
- Extracts `slug` / `repo` from frontmatter.
- Strips tag prefixes like `topic/` or `project/` into alias tokens.
- Normalizes all candidates into lowercase alphanumeric sets for token overlap matching.

### 2.4 Auto-Discovered Derived Aliases
When inspecting backlinks for candidate projects:
- Query `links WHERE target = ?1`.
- For each source document whose stem contains kebab-case words (e.g. `2026-09-07-okx-funding-arbitrage-phase-5-live-tui-monitor`):
  Strip date prefix `YYYY-MM-DD-` and common suffixes like `-phase-*`, `-audit`, `-live-*` to harvest clean slug `okx-funding-arbitrage`.
- Add to project's derived aliases.

---

## 3. Acceptance Criteria & Invariants
1. `find_project("okx-funding-arbitrage")` successfully resolves `10_Projects/OKX资金费率套利与跨生态对冲系统.md` even when dozens of `01_AI_Logs` with identical names exist.
2. `find_project("OKX")`, `find_project("资金费率")`, `find_project("funding-arbitrage")` all resolve unambiguously.
3. Scoped FTS never performs post-filtering on global top-N; scope constraint is pushed down to SQL.
4. All existing unit & integration tests pass with 0 warnings.
5. All new capabilities are covered by automated unit & integration tests using `tempfile` mock vaults.
