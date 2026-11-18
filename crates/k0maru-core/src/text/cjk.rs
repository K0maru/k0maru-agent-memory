//! Zero-dependency symmetric CJK bigram segmenter and safe FTS5 query builder.
//!
//! Ensures index-time and query-time tokenization stay symmetric for SQLite FTS5
//! (`unicode61 remove_diacritics 2`) across CJK runs, ASCII code identifiers, and
//! mixed-script phrases.

use std::cmp::Reverse;
use std::collections::HashSet;

/// Current version of the FTS5 text segmenter.
///
/// Bumped whenever [`segment_for_index`] changes tokenization output so existing
/// SQLite caches can detect and reindex stale `fts_hash` rows without dropping vectors.
pub const FTS_SEGMENTER_VERSION: u32 = 1;

/// Maximum number of OR-joined tokens emitted by [`FtsQueryTier::AnyTokens`].
const MAX_ANY_TOKENS: usize = 8;

/// Query construction tier for FTS5 full-text search fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FtsQueryTier {
    /// Exact phrase match per whitespace-separated user term (implicit AND across terms).
    Strict,
    /// All deduplicated segmented tokens must appear anywhere in the document (`AND`).
    AllTokens,
    /// Any deduplicated segmented token may match (`OR`), capped to avoid corpus-wide scans.
    AnyTokens,
}

/// Computes the versioned FTS hash stored alongside a document's `content_hash`.
pub fn fts_hash_for(content_hash: &str) -> String {
    format!("{}:{}", content_hash, FTS_SEGMENTER_VERSION)
}

/// Returns `true` if `c` belongs to Han, Kana, or Hangul Unicode ranges.
pub fn is_cjk(c: char) -> bool {
    matches!(
        c,
        // Han (CJK Unified Ideographs + Extensions + Compatibility)
        '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2CEAF}'
            // Kana (Hiragana, Katakana, Phonetic Extensions, Halfwidth Katakana)
            | '\u{3040}'..='\u{30FF}'
            | '\u{31F0}'..='\u{31FF}'
            | '\u{FF66}'..='\u{FF9F}'
            // Hangul (Syllables, Jamo, Compatibility Jamo)
            | '\u{AC00}'..='\u{D7AF}'
            | '\u{1100}'..='\u{11FF}'
            | '\u{3130}'..='\u{318F}'
    )
}

/// Folds fullwidth ASCII (`U+FF01..=U+FF5E`) and ideographic space (`U+3000`) to ASCII.
#[inline]
fn fold_fullwidth(c: char) -> char {
    match c {
        '\u{3000}' => ' ',
        '\u{FF01}'..='\u{FF5E}' => char::from_u32((c as u32) - 0xFEE0).unwrap_or(c),
        _ => c,
    }
}

#[inline]
fn is_indexable_char(c: char) -> bool {
    c.is_alphanumeric() || is_cjk(c)
}

#[inline]
fn is_single_cjk_str(s: &str) -> bool {
    let mut chars = s.chars();
    matches!((chars.next(), chars.next()), (Some(c), None) if is_cjk(c))
}

#[inline]
fn escape_fts5_phrase(s: &str) -> String {
    s.replace('"', "\"\"")
}

fn format_single_token(tok: &str) -> String {
    let escaped = escape_fts5_phrase(tok);
    if is_single_cjk_str(tok) {
        format!("\"{}\"*", escaped)
    } else {
        format!("\"{}\"", escaped)
    }
}

fn flush_cjk_run(cjk_run: &mut Vec<char>, out: &mut String) {
    if cjk_run.is_empty() {
        return;
    }
    if !out.is_empty() && !out.ends_with(' ') {
        out.push(' ');
    }
    if cjk_run.len() == 1 {
        out.push(cjk_run[0]);
    } else {
        // Emit overlapping bigrams only; omitting a tail unigram preserves
        // cross-script phrase adjacency (e.g. "使用Rust" -> "使用 Rust").
        for i in 0..(cjk_run.len() - 1) {
            if i > 0 {
                out.push(' ');
            }
            out.push(cjk_run[i]);
            out.push(cjk_run[i + 1]);
        }
    }
    cjk_run.clear();
}

/// Segments text symmetrically for FTS5 indexing and query construction.
///
/// - Folds fullwidth ASCII (`U+FF01..=U+FF5E`) and `\u{3000}` to halfwidth ASCII / `' '`.
/// - Emits overlapping bigrams for contiguous CJK runs of length >= 2, and a single
///   character for runs of length 1 (no tail unigram).
/// - Inserts clean whitespace boundaries when transitioning between CJK and non-CJK text
///   while preserving ASCII code identifiers intact.
pub fn segment_for_index(text: &str) -> String {
    let mut out = String::with_capacity(text.len().saturating_mul(2));
    let mut cjk_run: Vec<char> = Vec::new();
    let mut in_non_cjk = false;

    for c in text.chars().map(fold_fullwidth) {
        if c.is_whitespace() {
            flush_cjk_run(&mut cjk_run, &mut out);
            if !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
            in_non_cjk = false;
        } else if is_cjk(c) {
            in_non_cjk = false;
            cjk_run.push(c);
        } else {
            if !cjk_run.is_empty() {
                flush_cjk_run(&mut cjk_run, &mut out);
                out.push(' ');
            } else if !in_non_cjk && !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
            out.push(c);
            in_non_cjk = true;
        }
    }

    flush_cjk_run(&mut cjk_run, &mut out);
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Builds a sanitized FTS5 `MATCH` expression for the requested [`FtsQueryTier`].
///
/// Returns `None` when `user_query` contains no indexable alphanumeric or CJK characters.
/// Never passes unquoted FTS5 operators (`AND`, `OR`, `NOT`, `NEAR`, `*`, etc.) from user input.
pub fn build_fts_query(user_query: &str, tier: FtsQueryTier) -> Option<String> {
    let folded: String = user_query.chars().map(fold_fullwidth).collect();
    if folded.trim().is_empty() {
        return None;
    }

    match tier {
        FtsQueryTier::Strict => build_strict_query(&folded),
        FtsQueryTier::AllTokens => build_token_query(&folded, false),
        FtsQueryTier::AnyTokens => build_token_query(&folded, true),
    }
}

fn build_strict_query(folded_query: &str) -> Option<String> {
    let mut clauses: Vec<String> = Vec::new();

    for raw_term in folded_query.split_whitespace() {
        let segmented = segment_for_index(raw_term);
        let pieces: Vec<&str> = segmented
            .split_whitespace()
            .filter_map(|p| {
                let t = p.trim_matches(|c: char| !is_indexable_char(c) && c != '_');
                if t.chars().any(is_indexable_char) {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();

        if pieces.is_empty() {
            continue;
        }

        if pieces.len() == 1 {
            clauses.push(format_single_token(pieces[0]));
        } else {
            let phrase = pieces.join(" ");
            clauses.push(format!("\"{}\"", escape_fts5_phrase(&phrase)));
        }
    }

    if clauses.is_empty() {
        None
    } else {
        Some(clauses.join(" "))
    }
}

fn build_token_query(folded_query: &str, any_mode: bool) -> Option<String> {
    let segmented = segment_for_index(folded_query);
    let mut seen: HashSet<String> = HashSet::new();
    let mut tokens: Vec<String> = Vec::new();

    for raw_tok in segmented.split_whitespace() {
        let trimmed = raw_tok.trim_matches(|c: char| !is_indexable_char(c) && c != '_');
        if trimmed.is_empty() || !trimmed.chars().any(is_indexable_char) {
            continue;
        }
        let dedup_key = trimmed.to_lowercase();
        if seen.insert(dedup_key) {
            tokens.push(trimmed.to_string());
        }
    }

    if tokens.is_empty() {
        return None;
    }

    if !any_mode {
        let formatted: Vec<String> = tokens.iter().map(|t| format_single_token(t)).collect();
        return Some(formatted.join(" AND "));
    }

    let selected: Vec<&str> = if tokens.len() <= MAX_ANY_TOKENS {
        tokens.iter().map(|s| s.as_str()).collect()
    } else {
        // Prefer ASCII/identifier tokens (and longer terms) over CJK bigrams and single chars.
        let mut ranked: Vec<(u8, Reverse<usize>, usize)> = tokens
            .iter()
            .enumerate()
            .map(|(idx, tok)| {
                let char_len = tok.chars().count();
                let has_cjk = tok.chars().any(is_cjk);
                let bucket = if !has_cjk && char_len >= 3 {
                    0
                } else if !has_cjk {
                    1
                } else if char_len >= 2 {
                    2
                } else {
                    3
                };
                (bucket, Reverse(char_len), idx)
            })
            .collect();

        ranked.sort_unstable();
        let mut chosen_indices: Vec<usize> = ranked
            .into_iter()
            .take(MAX_ANY_TOKENS)
            .map(|(_, _, idx)| idx)
            .collect();
        chosen_indices.sort_unstable();
        chosen_indices
            .into_iter()
            .map(|idx| tokens[idx].as_str())
            .collect()
    };

    let formatted: Vec<String> = selected.into_iter().map(format_single_token).collect();
    Some(formatted.join(" OR "))
}
