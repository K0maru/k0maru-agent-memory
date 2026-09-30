//! Cache storage subsystem.
//!
//! Provides SQLite and FTS5 implementations for fast ephemeral caching,
//! full-text search, and directed graph neighbor lookups.

pub mod sqlite;

pub use sqlite::SqliteStorage;
