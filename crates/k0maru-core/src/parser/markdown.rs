//! Markdown AST parsing, YAML frontmatter extraction, WikiLinks and tags extraction.

use crate::core::models::WikiLink;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

/// Parses YAML frontmatter from markdown content.
/// Returns a tuple of `(serde_json::Value, &str)` where the second element is the markdown body.
/// Returns `(json!({}), content)` if frontmatter is missing or invalid.
pub fn parse_frontmatter(content: &str) -> (serde_json::Value, &str) {
    let trimmed_start = content.trim_start_matches('\u{FEFF}'); // Strip BOM if present
    if !trimmed_start.starts_with("---") {
        return (serde_json::json!({}), content);
    }

    // Must be followed by newline
    let after_dashes = &trimmed_start[3..];
    let yaml_start = if after_dashes.starts_with("\r\n") {
        3 + 2
    } else if after_dashes.starts_with('\n') {
        3 + 1
    } else {
        return (serde_json::json!({}), content);
    };

    // Search for closing --- on its own line
    // That means \n--- or \r\n--- followed by \r\n, \n, or EOF.
    let remaining = &trimmed_start[yaml_start..];
    let mut search_idx = 0;
    let mut found_close = None;

    while let Some(rel_idx) = remaining[search_idx..].find("---") {
        let abs_idx = search_idx + rel_idx;
        // Check if preceding is a newline (start of line)
        let is_line_start = if abs_idx == 0 {
            true
        } else {
            let before = &remaining[..abs_idx];
            before.ends_with('\n')
        };

        if is_line_start {
            // Check if following is newline or EOF
            let after = &remaining[abs_idx + 3..];
            if after.is_empty() || after.starts_with('\n') || after.starts_with("\r\n") {
                // Determine where body starts
                let body_start = if after.starts_with("\r\n") {
                    yaml_start + abs_idx + 3 + 2
                } else if after.starts_with('\n') {
                    yaml_start + abs_idx + 3 + 1
                } else {
                    yaml_start + abs_idx + 3
                };

                // The YAML content ends right before the closing --- line
                let yaml_end = if abs_idx > 0 && remaining.as_bytes()[abs_idx - 1] == b'\n' {
                    if abs_idx > 1 && remaining.as_bytes()[abs_idx - 2] == b'\r' {
                        abs_idx - 2
                    } else {
                        abs_idx - 1
                    }
                } else {
                    abs_idx
                };

                found_close = Some((yaml_end, body_start));
                break;
            }
        }
        search_idx = abs_idx + 3;
    }

    let (yaml_end, body_start) = match found_close {
        Some(indices) => indices,
        None => return (serde_json::json!({}), content),
    };

    let yaml_str = &remaining[..yaml_end];
    let body = &trimmed_start[body_start..];

    if yaml_str.trim().is_empty() {
        return (serde_json::json!({}), body);
    }

    match serde_yaml::from_str::<serde_json::Value>(yaml_str) {
        Ok(val) => {
            if val.is_null() {
                (serde_json::json!({}), body)
            } else {
                (val, body)
            }
        }
        Err(_) => {
            // Invalid YAML: gracefully return empty json with body
            (serde_json::json!({}), body)
        }
    }
}

struct CodeAndHtmlRanges {
    code_only: Vec<(usize, usize)>,
    all: Vec<(usize, usize)>,
}

/// Identifies byte ranges in `content` that represent code blocks (fenced and indented)
/// and inline code (`...`), plus HTML blocks/tags, which must be ignored when extracting links and tags.
pub fn extract_code_ranges(content: &str) -> Vec<(usize, usize)> {
    extract_code_and_html_ranges(content).all
}

/// Returns code-only ranges and combined code/HTML ranges sorted by start byte offset.
fn extract_code_and_html_ranges(content: &str) -> CodeAndHtmlRanges {
    if !content.contains('`')
        && !content.contains('~')
        && !content.contains('<')
        && !content.contains("    ")
        && !content.contains('\t')
    {
        return CodeAndHtmlRanges {
            code_only: Vec::new(),
            all: Vec::new(),
        };
    }

    let parser = Parser::new(content);
    let mut code_only = Vec::with_capacity(8);
    let mut all_ranges = Vec::with_capacity(8);
    let mut code_block_start = None;
    let mut html_block_start = None;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                code_block_start = Some(range.start);
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(start) = code_block_start.take() {
                    code_only.push((start, range.end));
                    all_ranges.push((start, range.end));
                }
            }
            Event::Code(_) => {
                code_only.push((range.start, range.end));
                all_ranges.push((range.start, range.end));
            }
            Event::Start(Tag::HtmlBlock) => {
                html_block_start = Some(range.start);
            }
            Event::End(TagEnd::HtmlBlock) => {
                if let Some(start) = html_block_start.take() {
                    all_ranges.push((start, range.end));
                }
            }
            Event::Html(_) => {
                all_ranges.push((range.start, range.end));
            }
            _ => {}
        }
    }

    code_only.sort_unstable_by_key(|&(start, _)| start);
    all_ranges.sort_unstable_by_key(|&(start, _)| start);
    CodeAndHtmlRanges {
        code_only,
        all: all_ranges,
    }
}

/// Opening marker for k0maru-managed Obsidian backlink blocks.
pub const BACKLINK_BLOCK_START: &str = "<!-- k0maru:backlinks:start -->";
/// Closing marker for k0maru-managed Obsidian backlink blocks.
pub const BACKLINK_BLOCK_END: &str = "<!-- k0maru:backlinks:end -->";

/// Identifies byte ranges inside `<!-- k0maru:backlinks:start --> ... <!-- k0maru:backlinks:end -->`
/// blocks that are outside fenced/inline code spans.
pub fn extract_managed_backlink_blocks(content: &str) -> Vec<(usize, usize)> {
    if !content.contains(BACKLINK_BLOCK_START) {
        return Vec::new();
    }
    let ranges = extract_code_and_html_ranges(content);
    extract_managed_backlink_ranges(content, &ranges.code_only)
}

/// Identifies byte ranges inside `<!-- k0maru:backlinks:start --> ... <!-- k0maru:backlinks:end -->`
/// blocks that are outside `code_only_ranges`, so woven backlinks never feed back into `links` or `tags`.
pub fn extract_managed_backlink_ranges(
    content: &str,
    code_only_ranges: &[(usize, usize)],
) -> Vec<(usize, usize)> {
    if !content.contains(BACKLINK_BLOCK_START) {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut search_from = 0;

    while let Some(rel_start) = content[search_from..].find(BACKLINK_BLOCK_START) {
        let start = search_from + rel_start;
        let after_start = start + BACKLINK_BLOCK_START.len();

        if is_in_excluded_range(start, after_start, code_only_ranges) {
            search_from = after_start;
            continue;
        }

        let mut end_search = after_start;
        let mut found_end = None;
        while let Some(rel_end) = content[end_search..].find(BACKLINK_BLOCK_END) {
            let end_start = end_search + rel_end;
            let end_finish = end_start + BACKLINK_BLOCK_END.len();
            if !is_in_excluded_range(end_start, end_finish, code_only_ranges) {
                found_end = Some(end_finish);
                break;
            }
            end_search = end_finish;
        }

        if let Some(block_end) = found_end {
            ranges.push((start, block_end));
            search_from = block_end;
        } else {
            // Unclosed start marker: do not swallow the rest of the document.
            search_from = after_start;
        }
    }

    ranges
}

/// Computes merged, sorted excluded byte ranges covering code/HTML spans and managed backlink blocks.
pub fn extract_excluded_ranges(content: &str) -> Vec<(usize, usize)> {
    let parsed = extract_code_and_html_ranges(content);
    let mut ranges = parsed.all;
    let backlink_ranges = extract_managed_backlink_ranges(content, &parsed.code_only);
    if !backlink_ranges.is_empty() {
        ranges.extend(backlink_ranges);
        ranges.sort_unstable_by_key(|&(start, _)| start);
    }

    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());
    for (s, e) in ranges {
        if let Some(last) = merged.last_mut() {
            if s <= last.1 {
                if e > last.1 {
                    last.1 = e;
                }
                continue;
            }
        }
        merged.push((s, e));
    }
    merged
}

/// Checks if a range `[start, end]` overlaps with any range in `excluded_ranges`.
#[inline]
fn is_in_excluded_range(start: usize, end: usize, excluded_ranges: &[(usize, usize)]) -> bool {
    for &(ex_start, ex_end) in excluded_ranges {
        if ex_start >= end {
            break;
        }
        if start < ex_end && end > ex_start {
            return true;
        }
    }
    false
}

/// Returns `true` if a CommonMark link destination points to an external URL or non-vault scheme.
fn is_external_url(url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("data:")
        || lower.starts_with("javascript:")
        || lower.starts_with("file://")
}

/// Extracts bare `[[Card]]`, aliased `[[Card|Alias]]`, and internal CommonMark `[Alias](target)` links
/// into `Vec<WikiLink>`.
/// Strictly ignores links within code blocks (fenced ``` and inline `) and managed backlink blocks,
/// CommonMark images (`![alt](img)`), and external URL schemes.
pub fn extract_wikilinks(content: &str) -> Vec<WikiLink> {
    let excluded_ranges = extract_excluded_ranges(content);
    extract_wikilinks_with_ranges(content, &excluded_ranges)
}

/// Extracts WikiLinks and internal CommonMark relative links using pre-computed excluded ranges.
pub fn extract_wikilinks_with_ranges(
    content: &str,
    code_ranges: &[(usize, usize)],
) -> Vec<WikiLink> {
    let mut positioned: Vec<(usize, WikiLink)> = Vec::with_capacity(8);
    let mut wikilink_spans: Vec<(usize, usize)> = Vec::new();
    let mut search_from = 0;
    let bytes = content.as_bytes();
    let len = bytes.len();

    while let Some(rel) = content[search_from..].find("[[") {
        let start = search_from + rel;
        let mut cursor = start + 2;

        let mut end = None;
        while cursor + 1 < len {
            let b = bytes[cursor];
            if b == b'\n' || b == b'\r' {
                break;
            }
            if b == b'[' && bytes[cursor + 1] == b'[' {
                break;
            }
            if b == b']' && bytes[cursor + 1] == b']' {
                end = Some(cursor);
                break;
            }
            cursor += 1;
        }

        if let Some(close_idx) = end {
            let full_end = close_idx + 2;
            search_from = full_end;

            if !is_in_excluded_range(start, full_end, code_ranges) {
                let inner = &content[start + 2..close_idx];
                let raw_text = &content[start..full_end];

                let mut parts = inner.splitn(2, '|');
                let target_raw = parts.next().unwrap_or("").trim();
                let alias_raw = parts.next().map(|a| a.trim());

                let target = if let Some(stripped) = target_raw.strip_suffix(".md") {
                    stripped.trim()
                } else {
                    target_raw
                };

                let alias = match alias_raw {
                    Some(a) if !a.is_empty() => Some(a.to_string()),
                    _ => None,
                };

                if !target.is_empty() {
                    wikilink_spans.push((start, full_end));
                    positioned.push((start, WikiLink::new(target, alias, raw_text)));
                }
            }
        } else {
            search_from = start + 2;
        }
    }

    if content.contains('[') {
        let parser = Parser::new(content);
        let mut image_depth: usize = 0;
        let mut link_stack: Vec<(usize, String, String)> = Vec::new();

        for (event, range) in parser.into_offset_iter() {
            match event {
                Event::Start(Tag::Image { .. }) => {
                    image_depth += 1;
                }
                Event::End(TagEnd::Image) => {
                    image_depth = image_depth.saturating_sub(1);
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    link_stack.push((range.start, dest_url.to_string(), String::new()));
                }
                Event::Text(t) | Event::Code(t) => {
                    if image_depth == 0 {
                        if let Some((_, _, alias_buf)) = link_stack.last_mut() {
                            alias_buf.push_str(&t);
                        }
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    if image_depth == 0 {
                        if let Some((_, _, alias_buf)) = link_stack.last_mut() {
                            alias_buf.push(' ');
                        }
                    }
                }
                Event::End(TagEnd::Link) => {
                    if let Some((link_start, dest_url, alias_buf)) = link_stack.pop() {
                        if is_in_excluded_range(link_start, range.end, code_ranges)
                            || is_external_url(&dest_url)
                        {
                            continue;
                        }
                        if wikilink_spans
                            .iter()
                            .any(|&(ws, we)| link_start < we && range.end > ws)
                        {
                            continue;
                        }
                        let trimmed_dest = dest_url.trim();
                        let target = trimmed_dest
                            .strip_suffix(".md")
                            .map(str::trim)
                            .unwrap_or(trimmed_dest);
                        if target.is_empty() {
                            continue;
                        }
                        let alias_trimmed = alias_buf.trim();
                        let alias = if alias_trimmed.is_empty() {
                            None
                        } else {
                            Some(alias_trimmed.to_string())
                        };
                        let raw_text = &content[link_start..range.end];
                        positioned.push((link_start, WikiLink::new(target, alias, raw_text)));
                    }
                }
                _ => {}
            }
        }
    }

    positioned.sort_by_key(|(pos, _)| *pos);
    positioned.into_iter().map(|(_, link)| link).collect()
}

/// Extracts `#tag` and `#tag/subtag` structures.
/// Strictly ignores markdown headers, URL anchors, and tags within code or managed backlink blocks.
pub fn extract_tags(content: &str) -> Vec<String> {
    let excluded_ranges = extract_excluded_ranges(content);
    extract_tags_with_ranges(content, &excluded_ranges)
}

/// Extracts `#tag` and `#tag/subtag` structures using pre-computed code ranges.
pub fn extract_tags_with_ranges(content: &str, code_ranges: &[(usize, usize)]) -> Vec<String> {
    let mut tags = Vec::with_capacity(8);
    let bytes = content.as_bytes();
    let mut search_from = 0;

    while let Some(rel) = content[search_from..].find('#') {
        let hash_pos = search_from + rel;

        // Check 1: Excluded in code ranges
        if is_in_excluded_range(hash_pos, hash_pos + 1, code_ranges) {
            search_from = hash_pos + 1;
            continue;
        }

        // Check 2: Preceding character boundary
        let is_valid_preceding = if hash_pos == 0 {
            true
        } else {
            let prev_b = bytes[hash_pos - 1];
            if prev_b < 128 {
                !prev_b.is_ascii_alphanumeric()
                    && prev_b != b'/'
                    && prev_b != b'\\'
                    && prev_b != b'='
                    && prev_b != b'&'
                    && prev_b != b'?'
                    && prev_b != b'#'
                    && prev_b != b'-'
                    && prev_b != b'_'
                    && prev_b != b'.'
            } else {
                let prev_char = content[..hash_pos].chars().next_back().unwrap();
                !prev_char.is_alphanumeric()
            }
        };

        if !is_valid_preceding {
            search_from = hash_pos + 1;
            continue;
        }

        // Check 3: Markdown ATX Heading detection
        let line_start = content[..hash_pos]
            .rfind('\n')
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let prefix = &content[line_start..hash_pos];
        let prefix_is_hashes_or_spaces =
            prefix.bytes().all(|b| b == b'#' || b == b' ' || b == b'\t');

        if prefix_is_hashes_or_spaces {
            let after_hash = &content[hash_pos + 1..];
            let next_non_hash = after_hash.trim_start_matches('#');
            if next_non_hash.starts_with(' ')
                || next_non_hash.starts_with('\t')
                || next_non_hash.starts_with('\r')
                || next_non_hash.starts_with('\n')
                || next_non_hash.is_empty()
            {
                // Markdown heading, skip!
                search_from = hash_pos + 1;
                continue;
            }
        }

        // Check 4: Check if inside markdown link target [text](url#anchor)
        let is_url_anchor = {
            let mut is_link_dest = false;
            let mut back = hash_pos;
            while back > line_start {
                let b = bytes[back - 1];
                if b == b'(' && back >= 2 && bytes[back - 2] == b']' {
                    is_link_dest = true;
                    break;
                }
                back -= 1;
            }
            is_link_dest
        };

        if is_url_anchor {
            search_from = hash_pos + 1;
            continue;
        }

        // Check 5: Read tag body
        let rest = &content[hash_pos + 1..];
        let mut tag_len = 0;
        for (idx, ch) in rest.char_indices() {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '/' {
                tag_len = idx + ch.len_utf8();
            } else {
                break;
            }
        }

        if tag_len > 0 {
            let candidate_body = &rest[..tag_len];
            let trimmed_body = candidate_body.trim_end_matches('/');

            if !trimmed_body.is_empty()
                && !trimmed_body.starts_with('/')
                && trimmed_body.chars().any(|c| !c.is_ascii_digit())
            {
                let tag = format!("#{}", trimmed_body);
                if !tags.contains(&tag) {
                    tags.push(tag);
                }
            }

            search_from = hash_pos + 1 + tag_len;
            continue;
        }

        search_from = hash_pos + 1;
    }

    tags
}

/// Simultaneously extracts WikiLinks and tags from markdown body in an optimized single AST pass.
pub fn extract_elements(content: &str) -> (Vec<WikiLink>, Vec<String>) {
    let excluded_ranges = extract_excluded_ranges(content);
    (
        extract_wikilinks_with_ranges(content, &excluded_ranges),
        extract_tags_with_ranges(content, &excluded_ranges),
    )
}
