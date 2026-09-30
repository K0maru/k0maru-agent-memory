//! Vector embeddings and similarity engine module.

pub mod engine;
pub mod error;
pub mod mock;

pub use engine::EmbeddingEngine;
pub use error::VectorError;
pub use mock::MockEmbeddingEngine;
