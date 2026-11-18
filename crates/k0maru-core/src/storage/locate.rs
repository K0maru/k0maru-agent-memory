//! Section-level locator formatting, vector-channel model gating, and post-rank section locating.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::storage::hybrid::{
    format_doc_lifecycle_snippet, generate_snippet, HybridSearchEngine, SearchResult,
};
use crate::storage::migrate::META_EMBED_MODEL;
use crate::storage::{SqliteStorage, StoredSection};
use crate::vector::EmbeddingEngine;

/// Maximum Unicode scalar characters preserved in a locator `#heading` fragment.
pub const MAX_LOCATOR_HEADING_CHARS: usize = 48;

/// RRF smoothing constant used to fuse lexical and vector section ranks during locate.
const LOCATE_RRF_K: f64 = 60.0;

static MODEL_MISMATCH_WARNED: OnceLock<Mutex<HashSet<(String, String)>>> = OnceLock::new();

/// Fine-grained section metadata attached to a [`SearchResult`] hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionLocator {
    /// Stable SQLite `document_sections.id` primary key.
    pub section_id: i64,
    /// 1-based starting line number in the raw source file.
    pub start_line: u32,
    /// 1-based inclusive ending line number in the raw source file.
    pub end_line: u32,
    /// Normalized leaf heading (`""` for level-0 preamble or headingless document).
    pub heading: String,
    /// Hierarchical breadcrumb (`"Title > H2 > H3"`).
    pub breadcrumb: String,
}

/// Sanitizes a heading for use inside a `path:start-end#heading` locator string.
///
/// Replaces `|` with `/`, strips `§` and newline characters, and truncates to
/// [`MAX_LOCATOR_HEADING_CHARS`] Unicode scalar characters.
pub fn sanitize_locator_heading(heading: &str) -> String {
    heading
        .chars()
        .filter(|&c| c != '§' && c != '\n' && c != '\r')
        .map(|c| if c == '|' { '/' } else { c })
        .take(MAX_LOCATOR_HEADING_CHARS)
        .collect()
}

/// Renders a canonical locator string `[@global:]<path>[:<start>-<end>][#<heading>]`.
///
/// Level-0 sections (`heading.is_empty()`) omit the `#heading` fragment.
pub fn format_locator(path: &str, scope: Option<&str>, section: Option<&SectionLocator>) -> String {
    let mut out = String::with_capacity(path.len() + 32);
    if scope == Some("global") {
        out.push_str("@global:");
    }
    out.push_str(path);
    if let Some(sec) = section {
        out.push(':');
        out.push_str(&sec.start_line.to_string());
        out.push('-');
        out.push_str(&sec.end_line.to_string());
        if !sec.heading.is_empty() {
            let clean = sanitize_locator_heading(&sec.heading);
            if !clean.is_empty() {
                out.push('#');
                out.push_str(&clean);
            }
        }
    }
    out
}

/// Checks whether the vector channel may run against `storage` with `embedder`.
///
/// Returns `false` and emits a one-time stderr warning if `k0maru_meta.embed_model`
/// is recorded and differs from `embedder.model_name()` (preventing silent mock or
/// cross-model vector corruption).
pub fn vector_scope_enabled(storage: &SqliteStorage, embedder: &dyn EmbeddingEngine) -> bool {
    let current = embedder.model_name();
    let recorded = match storage.meta_get(META_EMBED_MODEL) {
        Ok(Some(model)) => model,
        Ok(None) | Err(_) => return true,
    };

    if recorded == current {
        return true;
    }

    let warned = MODEL_MISMATCH_WARNED.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut guard) = warned.lock() {
        if guard.insert((recorded.clone(), current.to_string())) {
            eprintln!(
                "[k0maru] warning: vector channel disabled due to embed_model mismatch (index='{}', active='{}'); falling back to BM25",
                recorded, current
            );
        }
    }

    false
}

/// Attaches the best matching [`SectionLocator`] and section-scoped snippet to each
/// top-`limit` [`SearchResult`].
pub(crate) fn locate_sections_for_results(
    engine: &HybridSearchEngine<'_>,
    storage: &SqliteStorage,
    query: &str,
    results: &mut [SearchResult],
    vec_section_ranks: &HashMap<i64, usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    if results.is_empty() {
        return Ok(());
    }

    let mut result_doc_ids: Vec<(usize, i64)> = Vec::with_capacity(results.len());
    let mut doc_ids: Vec<i64> = Vec::with_capacity(results.len());
    let mut seen_doc_ids: HashSet<i64> = HashSet::with_capacity(results.len());

    for (idx, res) in results.iter().enumerate() {
        if let Some(doc_id) = storage.doc_id_for_path(Path::new(&res.path))? {
            result_doc_ids.push((idx, doc_id));
            if seen_doc_ids.insert(doc_id) {
                doc_ids.push(doc_id);
            }
        }
    }

    if doc_ids.is_empty() {
        return Ok(());
    }

    let fts_hits = storage.search_section_fts(query, &doc_ids, 200)?;
    let mut lex_ranks: HashMap<i64, usize> = HashMap::with_capacity(fts_hits.len());
    for (rank_idx, (section_id, _doc_id, _bm25)) in fts_hits.into_iter().enumerate() {
        lex_ranks.entry(section_id).or_insert(rank_idx + 1);
    }

    for (res_idx, doc_id) in result_doc_ids {
        let sections = storage.get_sections(doc_id)?;
        if sections.is_empty() {
            continue;
        }

        let Some(best_sec) = pick_best_section(&sections, &lex_ranks, vec_section_ranks) else {
            continue;
        };

        let res = &mut results[res_idx];
        res.section = Some(SectionLocator {
            section_id: best_sec.id,
            start_line: best_sec.start_line,
            end_line: best_sec.end_line,
            heading: best_sec.heading.clone(),
            breadcrumb: best_sec.breadcrumb.clone(),
        });

        if let Some(doc) = storage.get_document(Path::new(&res.path))? {
            if best_sec.byte_start <= best_sec.byte_end
                && best_sec.byte_end <= doc.body.len()
                && doc.body.is_char_boundary(best_sec.byte_start)
                && doc.body.is_char_boundary(best_sec.byte_end)
            {
                let slice = &doc.body[best_sec.byte_start..best_sec.byte_end];
                let raw_snippet = generate_snippet(slice, query, 200);
                res.snippet = format_doc_lifecycle_snippet(engine, &res.path, &doc, raw_snippet);
            }
        }
    }

    Ok(())
}

fn pick_best_section<'a>(
    sections: &'a [StoredSection],
    lex_ranks: &HashMap<i64, usize>,
    vec_ranks: &HashMap<i64, usize>,
) -> Option<&'a StoredSection> {
    let mut best: Option<&'a StoredSection> = None;
    let mut best_score = 0.0f64;

    for sec in sections {
        let r_lex = lex_ranks.get(&sec.id).copied();
        let r_vec = vec_ranks.get(&sec.id).copied();
        if r_lex.is_none() && r_vec.is_none() {
            continue;
        }
        let score = r_lex.map_or(0.0, |r| 1.0 / (LOCATE_RRF_K + r as f64))
            + r_vec.map_or(0.0, |r| 1.0 / (LOCATE_RRF_K + r as f64));
        if best.is_none() || score > best_score {
            best_score = score;
            best = Some(sec);
        }
    }

    best
}
