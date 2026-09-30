//! Cache storage subsystem.
//!
//! Provides SQLite and FTS5 implementations for fast ephemeral caching,
//! full-text search, directed graph neighbor lookups, and vector search.

pub mod sqlite;

pub use sqlite::{ensure_sqlite_vec_registered, SqliteStorage, VECTOR_DIMENSIONS};
