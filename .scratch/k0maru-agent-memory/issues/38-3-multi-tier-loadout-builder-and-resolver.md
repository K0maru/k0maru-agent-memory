# Issue 38-3: Multi-Tier LoadoutBuilder Project Resolver

**Status**: Ready
**Blocked by**: 38-1, 38-2

## Goal
Refactor `LoadoutBuilder::find_project` in `src/loadout/builder.rs` to implement the 4-tier resolution funnel:
1. Tier 1: Normalized Exact, Alias, Tag & Derived-Alias Match.
2. Tier 2: Scoped FTS5 SQL Push-down via `search_scoped_fts`.
3. Tier 3: Vector Semantic Fallback (via Cosine Similarity on Embeddings if available).
4. Tier 4: Fallback to ambient workspace / CWD directory matching.

## Acceptance Criteria
1. `find_project` calls `ProjectMetadata` extraction for candidate paths.
2. In Tier 1, checks normalized match against stem, H1, aliases, tags, and derived backlink prefixes.
3. In Tier 2, if Tier 1 yields no hit, executes `search_scoped_fts(query, Some("10_Projects/"), None, 5)` (or generic wiki equivalent).
4. In Tier 3, if storage has vectors, embeds query and evaluates similarity against candidate projects.
5. `cargo test -p k0maru` passes with 0 warnings.
