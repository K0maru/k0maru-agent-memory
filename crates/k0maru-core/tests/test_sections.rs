use k0maru_core::parser::markdown::parse_frontmatter;
use k0maru_core::parser::sections::{
    section_text_hash, sections_hash_for, split_sections, SectionSpan, MAX_HEADING_CHARS,
    MAX_SECTIONS_PER_DOC, MAX_SPLIT_LEVEL, SECTION_PARSER_VERSION,
};
use k0maru_core::text::FTS_SEGMENTER_VERSION;

fn slice_text<'a>(body: &'a str, span: &SectionSpan) -> &'a str {
    &body[span.byte_start..span.byte_end]
}

fn line_offset_for(full_content: &str, body: &str) -> usize {
    let prefix_bytes = body.as_ptr() as usize - full_content.as_ptr() as usize;
    full_content[..prefix_bytes]
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
}

#[test]
fn frozen_constants_and_hash_helpers() {
    assert_eq!(SECTION_PARSER_VERSION, 1);
    assert_eq!(MAX_SPLIT_LEVEL, 3);
    assert_eq!(MAX_SECTIONS_PER_DOC, 512);
    assert_eq!(MAX_HEADING_CHARS, 120);

    let h1 = section_text_hash("Gateway > Retry", "## Retry\nExponential backoff");
    let h2 = section_text_hash("Gateway > Retry", "## Retry\nExponential backoff");
    let h3 = section_text_hash("Gateway > Other", "## Retry\nExponential backoff");
    let h4 = section_text_hash("Gateway > Retry", "## Retry\nLinear backoff");
    assert_eq!(h1, h2);
    assert_ne!(h1, h3);
    assert_ne!(h1, h4);

    assert_eq!(
        sections_hash_for("0123456789abcdef"),
        format!("0123456789abcdef:s1:f{FTS_SEGMENTER_VERSION}")
    );
}

#[test]
fn empty_and_whitespace_body_return_empty() {
    assert!(split_sections("Empty", "", 0).is_empty());
    assert!(split_sections("Whitespace", "   \n\t\r\n  ", 4).is_empty());
}

#[test]
fn headingless_doc_emits_single_level_zero_section() {
    let body = "Payment gateway overview.\n#tag-not-heading\n####### seven hashes\n    ## four-space code\n";
    let spans = split_sections("Payment Gateway", body, 0);
    assert_eq!(spans.len(), 1);
    assert_eq!(
        spans[0],
        SectionSpan {
            ord: 0,
            level: 0,
            heading: String::new(),
            breadcrumb: "Payment Gateway".to_string(),
            byte_start: 0,
            byte_end: body.len(),
            start_line: 1,
            end_line: 4,
        }
    );
}

#[test]
fn preamble_before_first_h2_and_leading_blank_lines() {
    let with_preamble = "Intro line 1\nIntro line 2\n\n## Design\nBody text\n";
    let spans = split_sections("Settlement Spec", with_preamble, 0);
    assert_eq!(spans.len(), 2);

    assert_eq!(spans[0].ord, 0);
    assert_eq!(spans[0].level, 0);
    assert_eq!(spans[0].heading, "");
    assert_eq!(spans[0].breadcrumb, "Settlement Spec");
    assert_eq!(
        slice_text(with_preamble, &spans[0]),
        "Intro line 1\nIntro line 2\n\n"
    );
    assert_eq!((spans[0].start_line, spans[0].end_line), (1, 2));

    assert_eq!(spans[1].ord, 1);
    assert_eq!(spans[1].level, 2);
    assert_eq!(spans[1].heading, "Design");
    assert_eq!(spans[1].breadcrumb, "Settlement Spec > Design");
    assert_eq!(
        slice_text(with_preamble, &spans[1]),
        "## Design\nBody text\n"
    );
    assert_eq!((spans[1].start_line, spans[1].end_line), (4, 5));

    // Leading whitespace before the first heading must NOT emit a preamble.
    let leading_ws = "\n\n## Design\nBody text\n";
    let spans_ws = split_sections("Settlement Spec", leading_ws, 0);
    assert_eq!(spans_ws.len(), 1);
    assert_eq!(spans_ws[0].ord, 0);
    assert_eq!(spans_ws[0].level, 2);
    assert_eq!(spans_ws[0].byte_start, 2);
    assert_eq!((spans_ws[0].start_line, spans_ws[0].end_line), (3, 4));
}

#[test]
fn fenced_and_indented_code_do_not_split() {
    let body = "\
## Setup
```bash
# shell comment
## not a heading
```
~~~markdown
### tilde fence ignored
~~~
    ## four-space indented code
#not-a-tag-split

   ## Three-space indented IS a heading
done
";
    let spans = split_sections("Setup Guide", body, 0);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].heading, "Setup");
    assert_eq!((spans[0].start_line, spans[0].end_line), (1, 12));
    assert_eq!(spans[1].heading, "Three-space indented IS a heading");
    assert_eq!((spans[1].start_line, spans[1].end_line), (12, 13));
}

#[test]
fn unclosed_fence_swallows_remainder_of_doc() {
    let body = "## Before\n```rust\n## Inside unclosed fence\n### Still inside\n";
    let spans = split_sections("Fence Doc", body, 0);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].heading, "Before");
    assert_eq!(spans[0].byte_start, 0);
    assert_eq!(spans[0].byte_end, body.len());
    assert_eq!((spans[0].start_line, spans[0].end_line), (1, 4));
}

#[test]
fn setext_headings_and_para_hr_behavior() {
    let body = "\
Primary Title
Line Two
===

First Line  \nSecond Line
---
after setext

para text
---
more text
";
    let spans = split_sections("Primary Title Line Two", body, 0);
    assert_eq!(spans.len(), 3);

    // H1 matches title -> breadcrumb deduplicated.
    assert_eq!(spans[0].level, 1);
    assert_eq!(spans[0].heading, "Primary Title Line Two");
    assert_eq!(spans[0].breadcrumb, "Primary Title Line Two");
    assert_eq!((spans[0].start_line, spans[0].end_line), (1, 3));

    // Multi-line setext H2 with HardBreak joins lines with single space.
    assert_eq!(spans[1].level, 2);
    assert_eq!(spans[1].heading, "First Line Second Line");
    assert_eq!(
        spans[1].breadcrumb,
        "Primary Title Line Two > First Line Second Line"
    );
    assert_eq!((spans[1].start_line, spans[1].end_line), (5, 8));

    // CommonMark Rule 9: `para text\n---` is a setext H2.
    assert_eq!(spans[2].level, 2);
    assert_eq!(spans[2].heading, "para text");
    assert_eq!((spans[2].start_line, spans[2].end_line), (10, 12));
}

#[test]
fn blockquote_and_list_headings_ignored() {
    let body = "\
> ## Quoted Heading
> quoted body

- ## Bullet Heading
  bullet body

1. ### Ordered List Heading
   item body

## Real Top Heading
actual body
";
    let spans = split_sections("Container Test", body, 0);
    assert_eq!(spans.len(), 2);

    // Container headings do not split, so everything before `## Real Top Heading` is preamble.
    assert_eq!(spans[0].level, 0);
    assert_eq!(spans[0].heading, "");
    assert_eq!((spans[0].start_line, spans[0].end_line), (1, 8));

    assert_eq!(spans[1].level, 2);
    assert_eq!(spans[1].heading, "Real Top Heading");
    assert_eq!((spans[1].start_line, spans[1].end_line), (10, 11));
}

#[test]
fn h4_stays_inside_parent_and_breadcrumb_stack_pops() {
    let body = "\
## Architecture
intro
#### Deep Implementation Detail
h4 body
### Retry Queue
h3 body
## Monitoring
metrics
";
    let spans = split_sections("Gateway", body, 0);
    assert_eq!(spans.len(), 3);

    assert_eq!(spans[0].level, 2);
    assert_eq!(spans[0].heading, "Architecture");
    assert_eq!(spans[0].breadcrumb, "Gateway > Architecture");
    assert!(slice_text(body, &spans[0]).contains("#### Deep Implementation Detail"));
    assert_eq!((spans[0].start_line, spans[0].end_line), (1, 4));

    assert_eq!(spans[1].level, 3);
    assert_eq!(spans[1].heading, "Retry Queue");
    assert_eq!(spans[1].breadcrumb, "Gateway > Architecture > Retry Queue");
    assert_eq!((spans[1].start_line, spans[1].end_line), (5, 6));

    assert_eq!(spans[2].level, 2);
    assert_eq!(spans[2].heading, "Monitoring");
    assert_eq!(spans[2].breadcrumb, "Gateway > Monitoring");
    assert_eq!((spans[2].start_line, spans[2].end_line), (7, 8));
}

#[test]
fn crlf_and_empty_headings_handled_accurately() {
    let body = "## Alpha\r\nline one\r\n\r\n##\r\n##   \r\n## Beta\r\nend";
    let spans = split_sections("CRLF Note", body, 0);
    assert_eq!(spans.len(), 4);

    assert_eq!(spans[0].heading, "Alpha");
    assert_eq!((spans[0].start_line, spans[0].end_line), (1, 2));

    assert_eq!(spans[1].heading, "(untitled)");
    assert_eq!((spans[1].start_line, spans[1].end_line), (4, 4));

    assert_eq!(spans[2].heading, "(untitled)");
    assert_eq!((spans[2].start_line, spans[2].end_line), (5, 5));

    assert_eq!(spans[3].heading, "Beta");
    assert_eq!((spans[3].start_line, spans[3].end_line), (6, 7));
}

#[test]
fn h1_title_case_insensitive_dedup_and_distinct_h1() {
    let body = "\
# payment gateway
intro
## Idempotency
key handling
## Payment Gateway
h2 with same name as title is kept
# Appendix
extra
### Edge Cases
none
";
    let spans = split_sections("Payment Gateway", body, 0);
    assert_eq!(spans.len(), 5);

    assert_eq!(spans[0].level, 1);
    assert_eq!(spans[0].heading, "payment gateway");
    assert_eq!(spans[0].breadcrumb, "Payment Gateway");

    assert_eq!(spans[1].level, 2);
    assert_eq!(spans[1].breadcrumb, "Payment Gateway > Idempotency");

    assert_eq!(spans[2].level, 2);
    assert_eq!(spans[2].breadcrumb, "Payment Gateway > Payment Gateway");

    assert_eq!(spans[3].level, 1);
    assert_eq!(spans[3].breadcrumb, "Payment Gateway > Appendix");

    assert_eq!(spans[4].level, 3);
    assert_eq!(
        spans[4].breadcrumb,
        "Payment Gateway > Appendix > Edge Cases"
    );
}

#[test]
fn cjk_wikilinks_attrs_and_120_char_truncation() {
    let long_cjk = "分布式结算".repeat(30); // 150 chars
    let body = format!(
        "## 结算引擎 `v2` [[distributed-settlement|分布式结算网关]] [[retry-policy]] **核心** {{#core-v2}}\nbody\n## {long_cjk}\nmore\n"
    );
    let spans = split_sections("系统设计", &body, 0);
    assert_eq!(spans.len(), 2);

    assert_eq!(
        spans[0].heading,
        "结算引擎 v2 分布式结算网关 retry-policy 核心 {#core-v2}"
    );
    assert_eq!(
        spans[0].breadcrumb,
        "系统设计 > 结算引擎 v2 分布式结算网关 retry-policy 核心 {#core-v2}"
    );
    assert_eq!((spans[0].start_line, spans[0].end_line), (1, 2));

    assert_eq!(spans[1].heading.chars().count(), MAX_HEADING_CHARS);
    assert!(body.is_char_boundary(spans[1].byte_start));
    assert!(body.is_char_boundary(spans[1].byte_end));
    assert_eq!((spans[1].start_line, spans[1].end_line), (3, 4));
}

#[test]
fn max_sections_per_doc_caps_at_512_merging_remainder() {
    let mut body = String::from("Preamble before headings\n\n");
    for i in 1..=550 {
        body.push_str(&format!("## Section {i}\nLine for section {i}\n"));
    }

    let spans = split_sections("Large Spec", &body, 0);
    assert_eq!(spans.len(), MAX_SECTIONS_PER_DOC);

    for (idx, span) in spans.iter().enumerate() {
        assert_eq!(span.ord, idx as u32);
    }

    // ord 0 is preamble, ord 511 is Section 511 which absorbs Sections 512..=550.
    assert_eq!(spans[0].level, 0);
    let last = spans.last().unwrap();
    assert_eq!(last.ord, 511);
    assert_eq!(last.heading, "Section 511");
    assert_eq!(last.byte_end, body.len());
    let tail = slice_text(&body, last);
    assert!(tail.contains("## Section 550"));
    assert!(tail.contains("Line for section 550"));
    assert_eq!(last.end_line, body.lines().count() as u32);
}

#[test]
fn exact_file_line_math_with_frontmatter_offset_and_cjk() {
    let full_file = "\
---
title: 分布式网关设计
tags:
  - type/design
---
前置摘要段落

## 幂等性校验
第一行内容
第二行中文结尾

## 失败重试
最后一行无换行";

    let (fm, body) = parse_frontmatter(full_file);
    let title = fm["title"].as_str().unwrap();
    let offset = line_offset_for(full_file, body);
    assert_eq!(offset, 5);

    let spans = split_sections(title, body, offset);
    assert_eq!(spans.len(), 3);

    let file_lines: Vec<&str> = full_file.lines().collect();

    // Preamble: line 6
    assert_eq!((spans[0].start_line, spans[0].end_line), (6, 6));
    assert_eq!(
        &file_lines[(spans[0].start_line as usize - 1)..spans[0].end_line as usize],
        &["前置摘要段落"]
    );

    // ## 幂等性校验: lines 8..=10
    assert_eq!((spans[1].start_line, spans[1].end_line), (8, 10));
    assert_eq!(
        &file_lines[(spans[1].start_line as usize - 1)..spans[1].end_line as usize],
        &["## 幂等性校验", "第一行内容", "第二行中文结尾"]
    );

    // ## 失败重试: lines 12..=13
    assert_eq!((spans[2].start_line, spans[2].end_line), (12, 13));
    assert_eq!(
        &file_lines[(spans[2].start_line as usize - 1)..spans[2].end_line as usize],
        &["## 失败重试", "最后一行无换行"]
    );
}
