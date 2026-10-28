//! Incremental file scanner and vault cache synchronization.
//!
//! Provides dirty-checking file traversal and synchronization into SQLite cache.

pub mod engine;
pub mod vector_sync;

pub use engine::{DirtySets, IncrementalScanner};
pub use vector_sync::{parse_content_hash, VectorSyncEngine, VectorSyncStats, DEFAULT_BATCH_SIZE};
