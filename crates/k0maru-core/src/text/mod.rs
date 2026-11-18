//! Text segmentation and FTS5 query construction utilities.

pub mod cjk;

pub use cjk::{
    build_fts_query, fts_hash_for, is_cjk, segment_for_index, FtsQueryTier, FTS_SEGMENTER_VERSION,
};
