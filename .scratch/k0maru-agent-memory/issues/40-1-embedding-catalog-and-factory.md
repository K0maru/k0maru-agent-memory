# Ticket 40-1: Embedding Catalog, Enum, and Factory Resolver

**Status**: Completed
**Blocked by**: None

## Goal
Implement `EmbeddingModelType` enum, string normalizer/alias parser, and unified factory resolver in `k0maru-core` supporting multiple models (`all-minilm-l6-v2`, `bge-small-zh-v1.5`, `bge-small-en-v1.5`, `bge-base-en-v1.5`, `paraphrase-multilingual-minilm-l12-v2`, `mock`).

## Acceptance Criteria
1. Define `EmbeddingModelType` in `crates/k0maru-core/src/vector/models.rs`:
   - Variants: `AllMiniLML6V2`, `BgeSmallEnV15`, `BgeSmallZhV15`, `BgeBaseEnV15`, `ParaphraseMlMiniLmL12V2`, `Mock`.
   - Methods: `dimension(&self) -> usize`, `canonical_name(&self) -> &'static str`, `from_str(&str) -> Result<Self, VectorError>`.
   - Case-insensitive string parsing supporting common aliases (e.g. "bge-zh", "bge-small-zh", "minilm").
2. Implement `resolve_embedding_engine(model_str: Option<&str>, cache_dir: Option<PathBuf>) -> Result<Arc<dyn EmbeddingEngine>, VectorError>` in `crates/k0maru-core/src/vector/mod.rs`:
   - If model_str is None, inspect `K0MARU_EMBED_MODEL` env var, falling back to `AllMiniLML6V2`.
   - If `K0MARU_FORCE_MOCK_EMBED` is set or model is `Mock`, return `MockEmbeddingEngine` with target dimension.
   - If FastEmbed model initialization fails (e.g. offline and weights uncached), log warning and gracefully fallback to `MockEmbeddingEngine`.
3. Export new types in `crates/k0maru-core/src/lib.rs`.
4. Unit tests in `crates/k0maru-core/tests/test_vector_catalog.rs` pass with 100% coverage on parsing, dimension mappings, and fallback behavior.
