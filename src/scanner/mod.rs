//! Incremental file scanner and vault cache synchronization.
//!
//! Provides dirty-checking file traversal and synchronization into SQLite cache.

pub mod engine;

pub use engine::{DirtySets, IncrementalScanner};
