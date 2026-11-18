//! Incremental file scanner and vault cache synchronization.
//!
//! Provides dirty-checking file traversal and synchronization into SQLite cache.

pub mod engine;
pub mod filter;
pub mod vector_sync;

pub use engine::{DirtySets, IncrementalScanner};
pub use filter::ScanFilter;
pub use vector_sync::{
    parse_content_hash, SectionVectorStats, VectorSyncEngine, VectorSyncStats, DEFAULT_BATCH_SIZE,
};
