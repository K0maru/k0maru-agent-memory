//! `k0maru-core`: Cleanroom Core Knowledge and Memory Engine
//!
//! Provides domain models, Markdown AST parsing, FTS5 BM25 search,
//! sqlite-vec vector retrieval, WikiLinks graph algorithms, and incremental vault scanning.
//!
//! Designed as an independent library crate with zero dependency on CLI argument parsers (`clap`),
//! HTTP servers (`axum`), or static asset embedders (`rust-embed`).

pub mod adapters;
pub mod core;
pub mod gc;
pub mod parser;
pub mod scanner;
pub mod storage;
pub mod vector;

// Direct submodule aliases for ergonomic access
pub use core::models;
pub use core::traits;

// Domain models
pub use core::models::{CachedDocMeta, Document, HierarchyLevel, SyncStats, WikiLink};
/// Semantic alias for Document in note-taking runtimes
pub type Note = Document;

// Traits
pub use core::traits::{CacheStorage, VaultAdapter};

// Adapters
pub use adapters::{GenericWikiAdapter, ObsidianAdapter};

// Parser
pub use parser::{extract_elements, extract_tags, extract_wikilinks, parse_frontmatter};

// Storage
pub use storage::{
    ensure_sqlite_vec_registered, HybridSearchEngine, SearchMode, SearchResult, SqliteStorage,
    VECTOR_DIMENSIONS,
};

// Scanner
pub use scanner::{
    parse_content_hash, DirtySets, IncrementalScanner, VectorSyncEngine, VectorSyncStats,
    DEFAULT_BATCH_SIZE,
};

// Vector
pub use vector::{default_embedding_engine, EmbeddingEngine, MockEmbeddingEngine, VectorError};
#[cfg(feature = "fastembed")]
pub use vector::{resolve_cache_dir, FastEmbedBackend};

// Context GC
pub use gc::{
    estimate_tokens, ContextGcEngine, GcError, GcEvent, GcOptions, GcResult, SessionTrajectory,
    TrajectoryTurn, TurnRole,
};
