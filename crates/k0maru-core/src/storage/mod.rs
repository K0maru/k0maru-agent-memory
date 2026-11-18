//! Cache storage subsystem.
//!
//! Provides SQLite and FTS5 implementations for fast ephemeral caching,
//! full-text search, directed graph neighbor lookups, and vector search.

pub mod federated;
pub mod hybrid;
pub mod locate;
pub mod migrate;
pub mod rank_signals;
pub mod sqlite;

pub use federated::{resolve_global_vault, FederatedSearch, ReadOnlyStorage, GLOBAL_PREFIX};
pub use hybrid::{GraphBoostMode, HybridSearchEngine, SearchMode, SearchResult};
pub use locate::{vector_scope_enabled, SectionLocator};
pub use migrate::{ensure_schema, CURRENT_SCHEMA_VERSION};
pub use rank_signals::DecayConfig;
pub use sqlite::{
    ensure_sqlite_vec_registered, LinkEdgeStats, SectionFtsHit, SectionVectorHit, SqliteStorage,
    StoredSection, VECTOR_DIMENSIONS,
};
