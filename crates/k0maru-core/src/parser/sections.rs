//! Markdown AST heading-aware section splitter for fine-grained recall.

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use xxhash_rust::xxh3::xxh3_64;

/// Version of the heading-aware section splitter algorithm.
///
/// Bumped whenever [`split_sections`] changes span boundaries or normalization
/// so cached `document_section_state` rows can detect and repair stale sections.
pub const SECTION_PARSER_VERSION: u32 = 1;

/// Maximum heading level that triggers a section split (H1..=H3 split; H4+ stay inside parent).
pub const MAX_SPLIT_LEVEL: u8 = 3;

/// Maximum number of sections emitted per document; any remainder is merged into the last section.
pub const MAX_SECTIONS_PER_DOC: usize = 512;

/// Maximum length in Unicode scalar characters for a normalized heading.
pub const MAX_HEADING_CHARS: usize = 120;

/// Byte and 1-based file line span for a single Markdown section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionSpan {
    /// Zero-based ordinal index of this section within the document.
    pub ord: u32,
    /// Heading level (`0` for preamble or headingless document; `1..=3` for split headings).
    pub level: u8,
    /// Normalized leaf heading text (`""` for level 0).
    pub heading: String,
    /// Hierarchical breadcrumb (`"Title > H2 > H3"`, or `"Title"` for level 0).
    pub breadcrumb: String,
    /// UTF-8 byte start offset into `body` (inclusive, guaranteed char boundary).
    pub byte_start: usize,
    /// UTF-8 byte end offset into `body` (exclusive, guaranteed char boundary).
    pub byte_end: usize,
    /// 1-based starting line number in the source file (`body_line_offset` applied).
    pub start_line: u32,
    /// 1-based inclusive ending line number in the source file (`body_line_offset` applied).
    pub end_line: u32,
}

/// Splits a Markdown document `body` into heading-aware [`SectionSpan`] slices.
///
/// Uses `pulldown_cmark::Parser::new(body)` with default CommonMark options so heading
/// detection matches `extract_first_h1`. Container headings (inside blockquotes or lists)
/// and H4..=H6 headings do not split.
pub fn split_sections(title: &str, body: &str, body_line_offset: usize) -> Vec<SectionSpan> {
    if body.trim().is_empty() {
        return Vec::new();
    }

    let newlines: Vec<usize> = body
        .bytes()
        .enumerate()
        .filter_map(|(idx, b)| (b == b'\n').then_some(idx))
        .collect();

    let line_of = |byte_idx: usize| -> u32 {
        let count = newlines.partition_point(|&pos| pos < byte_idx);
        u32::try_from(body_line_offset + count + 1).unwrap_or(u32::MAX)
    };

    let raw_headings = collect_split_headings(body);
    let mut starts: Vec<(usize, u8, String, String)> = Vec::new();

    if raw_headings.is_empty() {
        starts.push((0, 0, String::new(), title.to_string()));
    } else {
        let first_start = raw_headings[0].byte_start;
        if first_start > 0 && !body[..first_start].trim().is_empty() {
            starts.push((0, 0, String::new(), title.to_string()));
        }

        let mut stack: Vec<(u8, String)> = Vec::new();
        for h in raw_headings {
            while let Some((top_lvl, _)) = stack.last() {
                if *top_lvl >= h.level {
                    stack.pop();
                } else {
                    break;
                }
            }
            stack.push((h.level, h.text.clone()));
            let breadcrumb = build_breadcrumb(title, &stack);
            starts.push((h.byte_start, h.level, h.text, breadcrumb));
        }

        if starts.len() > MAX_SECTIONS_PER_DOC {
            starts.truncate(MAX_SECTIONS_PER_DOC);
        }
    }

    let bytes = body.as_bytes();
    let total_sections = starts.len();
    let mut spans = Vec::with_capacity(total_sections);

    for (idx, (byte_start, level, heading, breadcrumb)) in starts.iter().enumerate() {
        let byte_start = *byte_start;
        let byte_end = if idx + 1 < total_sections {
            starts[idx + 1].0
        } else {
            body.len()
        };

        debug_assert!(body.is_char_boundary(byte_start) && body.is_char_boundary(byte_end));

        let start_line = line_of(byte_start);
        let mut trimmed_end = byte_end;
        while trimmed_end > byte_start
            && (bytes[trimmed_end - 1] == b'\n' || bytes[trimmed_end - 1] == b'\r')
        {
            trimmed_end -= 1;
        }
        let end_line = if trimmed_end > byte_start {
            line_of(trimmed_end - 1)
        } else {
            start_line
        };

        spans.push(SectionSpan {
            ord: idx as u32,
            level: *level,
            heading: heading.clone(),
            breadcrumb: breadcrumb.clone(),
            byte_start,
            byte_end,
            start_line,
            end_line,
        });
    }

    spans
}

/// Computes the deterministic 64-bit hash (`xxh3_64` cast to `i64`) for section reuse tracking.
pub fn section_text_hash(breadcrumb: &str, section_text: &str) -> i64 {
    let payload = format!("{breadcrumb}\n{section_text}");
    xxh3_64(payload.as_bytes()) as i64
}

/// Builds the versioned composite hash stored in `document_section_state.sections_hash`.
pub fn sections_hash_for(content_hash: &str) -> String {
    format!(
        "{content_hash}:s{SECTION_PARSER_VERSION}:f{}",
        crate::text::FTS_SEGMENTER_VERSION
    )
}

struct RawHeading {
    byte_start: usize,
    level: u8,
    text: String,
}

fn collect_split_headings(body: &str) -> Vec<RawHeading> {
    let mut depth: usize = 0;
    let mut current: Option<(u8, usize, String)> = None;
    let mut headings = Vec::new();

    for (event, range) in Parser::new(body).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let lvl = heading_level_to_u8(level);
                if depth == 0 && lvl <= MAX_SPLIT_LEVEL {
                    current = Some((lvl, range.start, String::new()));
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((lvl, byte_start, raw_text)) = current.take() {
                    headings.push(RawHeading {
                        byte_start,
                        level: lvl,
                        text: normalize_heading(&raw_text),
                    });
                }
            }
            Event::Start(_) => {
                depth += 1;
            }
            Event::End(_) => {
                depth = depth.saturating_sub(1);
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, _, ref mut buf)) = current {
                    buf.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some((_, _, ref mut buf)) = current {
                    buf.push(' ');
                }
            }
            _ => {}
        }
    }

    headings
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn normalize_heading(raw: &str) -> String {
    let expanded = expand_wikilinks(raw);
    let mut collapsed = String::with_capacity(expanded.len());
    for word in expanded.split_whitespace() {
        if !collapsed.is_empty() {
            collapsed.push(' ');
        }
        collapsed.push_str(word);
    }

    if let Some((byte_idx, _)) = collapsed.char_indices().nth(MAX_HEADING_CHARS) {
        collapsed.truncate(byte_idx);
    }

    if collapsed.is_empty() {
        "(untitled)".to_string()
    } else {
        collapsed
    }
}

fn expand_wikilinks(input: &str) -> String {
    if !input.contains("[[") {
        return input.to_string();
    }

    let mut out = String::with_capacity(input.len());
    let mut cursor = 0;

    while let Some(rel_start) = input[cursor..].find("[[") {
        let start = cursor + rel_start;
        out.push_str(&input[cursor..start]);

        let after_open = &input[start + 2..];
        if let Some(rel_close) = after_open.find("]]") {
            let inner = &after_open[..rel_close];
            if let Some(nested) = inner.find("[[") {
                out.push_str(&input[start..start + 2 + nested]);
                cursor = start + 2 + nested;
                continue;
            }
            let replacement = match inner.split_once('|') {
                Some((target, alias)) => {
                    if alias.trim().is_empty() {
                        target
                    } else {
                        alias
                    }
                }
                None => inner,
            };
            out.push_str(replacement);
            cursor = start + 2 + rel_close + 2;
        } else {
            out.push_str(&input[start..]);
            cursor = input.len();
            break;
        }
    }

    if cursor < input.len() {
        out.push_str(&input[cursor..]);
    }
    out
}

fn build_breadcrumb(title: &str, stack: &[(u8, String)]) -> String {
    let title_lower = title.to_lowercase();
    let mut parts: Vec<&str> = Vec::with_capacity(stack.len() + 1);
    if !title.is_empty() {
        parts.push(title);
    }
    for (lvl, text) in stack {
        if *lvl == 1 && !title.is_empty() && text.to_lowercase() == title_lower {
            continue;
        }
        parts.push(text.as_str());
    }
    parts.join(" > ")
}
