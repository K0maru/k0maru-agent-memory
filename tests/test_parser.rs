use k0maru::core::models::WikiLink;
use k0maru::parser::{extract_elements, extract_tags, extract_wikilinks, parse_frontmatter};
use serde_json::json;
use std::time::Instant;

#[test]
fn test_frontmatter_valid_yaml() {
    let content = r#"---
title: "Deep Module Design"
hierarchy: L3Evergreen
tags:
  - architecture
  - clean-code
verified: true
counter: 42
---
# Deep Module Design

This is the document body with some content.
"#;

    let (frontmatter, body) = parse_frontmatter(content);
    assert_eq!(frontmatter["title"], "Deep Module Design");
    assert_eq!(frontmatter["hierarchy"], "L3Evergreen");
    assert_eq!(frontmatter["tags"][0], "architecture");
    assert_eq!(frontmatter["tags"][1], "clean-code");
    assert_eq!(frontmatter["verified"], true);
    assert_eq!(frontmatter["counter"], 42);

    assert!(body.starts_with("# Deep Module Design"));
    assert!(body.contains("This is the document body with some content."));
}

#[test]
fn test_frontmatter_empty() {
    let content = "---\n---\n# Empty Frontmatter Body";
    let (frontmatter, body) = parse_frontmatter(content);
    assert_eq!(frontmatter, json!({}));
    assert_eq!(body, "# Empty Frontmatter Body");
}

#[test]
fn test_frontmatter_crlf() {
    let content = "---\r\ntitle: CRLF Note\r\nstatus: active\r\n---\r\n# Body with CRLF";
    let (frontmatter, body) = parse_frontmatter(content);
    assert_eq!(frontmatter["title"], "CRLF Note");
    assert_eq!(frontmatter["status"], "active");
    assert_eq!(body, "# Body with CRLF");
}

#[test]
fn test_frontmatter_missing() {
    let content = "# Title Without Frontmatter\n\nJust markdown body content.";
    let (frontmatter, body) = parse_frontmatter(content);
    assert_eq!(frontmatter, json!({}));
    assert_eq!(body, content);
}

#[test]
fn test_frontmatter_unclosed() {
    let content = "---\ntitle: Never closed\nbody starts here without closing";
    let (frontmatter, body) = parse_frontmatter(content);
    assert_eq!(frontmatter, json!({}));
    assert_eq!(body, content);
}

#[test]
fn test_frontmatter_invalid_yaml() {
    let content = "---\ninvalid: yaml: : : [unmatched\n---\nBody after invalid YAML";
    let (frontmatter, body) = parse_frontmatter(content);
    assert_eq!(frontmatter, json!({}));
    // Graceful fallback without panic
    assert!(body.contains("Body after invalid YAML") || body == content);
}

#[test]
fn test_frontmatter_hr_not_confused() {
    let content = "# First Heading\n\nSome text\n---\nMore text after horizontal rule";
    let (frontmatter, body) = parse_frontmatter(content);
    assert_eq!(frontmatter, json!({}));
    assert_eq!(body, content);
}

#[test]
fn test_extract_wikilinks_bare_and_aliased() {
    let content = "Check [[CardA]] and aliased [[CardB|Display B]] here.";
    let links = extract_wikilinks(content);
    assert_eq!(links.len(), 2);
    assert_eq!(
        links[0],
        WikiLink {
            target: "CardA".to_string(),
            alias: None,
            raw_text: "[[CardA]]".to_string(),
        }
    );
    assert_eq!(
        links[1],
        WikiLink {
            target: "CardB".to_string(),
            alias: Some("Display B".to_string()),
            raw_text: "[[CardB|Display B]]".to_string(),
        }
    );
}

#[test]
fn test_extract_wikilinks_paths_and_unicode() {
    let content = r#"Links with paths and unicode:
- [[01_AI_Logs/2026-09-28-log]]
- [[20_Cards/深度模块设计|深度模块]]
- [[Notes/Subfolder/Architecture.md]]
"#;
    let links = extract_wikilinks(content);
    assert_eq!(links.len(), 3);
    assert_eq!(links[0].target, "01_AI_Logs/2026-09-28-log");
    assert_eq!(links[0].alias, None);

    assert_eq!(links[1].target, "20_Cards/深度模块设计");
    assert_eq!(links[1].alias, Some("深度模块".to_string()));

    // .md suffix stripped from target
    assert_eq!(links[2].target, "Notes/Subfolder/Architecture");
    assert_eq!(links[2].alias, None);
}

#[test]
fn test_extract_wikilinks_whitespace_trimming() {
    let content = "[[  Target Card  |  Custom Alias  ]]";
    let links = extract_wikilinks(content);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "Target Card");
    assert_eq!(links[0].alias, Some("Custom Alias".to_string()));
    assert_eq!(links[0].raw_text, "[[  Target Card  |  Custom Alias  ]]");
}

#[test]
fn test_extract_wikilinks_ignore_code_blocks() {
    let content = r#"Here is a real link: [[RealCard1]].

```rust
// This is inside a fenced code block
let fake = "[[FakeInRustCode]]";
let another_fake = "[[FakeRust|Alias]]";
```

Outside again: [[RealCard2|Alias 2]].

```
Plain code block [[FakeInPlainBlock]]
```

Finally: [[RealCard3]].
"#;

    let links = extract_wikilinks(content);
    let targets: Vec<&str> = links.iter().map(|l| l.target.as_str()).collect();
    assert_eq!(targets, vec!["RealCard1", "RealCard2", "RealCard3"]);
}

#[test]
fn test_extract_wikilinks_ignore_inline_code() {
    let content = "Do not parse `[[FakeInlineLink]]` or `` `[[AnotherInline|Fake]]` `` inside inline code, but do parse [[ValidAfter]].";
    let links = extract_wikilinks(content);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "ValidAfter");
}

#[test]
fn test_extract_wikilinks_edge_cases() {
    let content = r#"
Ignore malformed:
- [SingleBracket]
- [[]]
- [[   ]]
- [[UnclosedBracket
- UnopenedBracket]]
- [[|OnlyAlias]]
- [[ValidOne]]
"#;
    let links = extract_wikilinks(content);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "ValidOne");
}

#[test]
fn test_extract_tags_simple_and_nested() {
    let content = "Testing #rust and #dev/backend and #topic/ai-ml tags.";
    let tags = extract_tags(content);
    assert_eq!(tags, vec!["#rust", "#dev/backend", "#topic/ai-ml"]);
}

#[test]
fn test_extract_tags_unicode() {
    let content = "中文标签：#知识库/架构设计 与 #原则/测试驱动开发";
    let tags = extract_tags(content);
    assert_eq!(tags, vec!["#知识库/架构设计", "#原则/测试驱动开发"]);
}

#[test]
fn test_extract_tags_ignore_markdown_headers() {
    let content = r#"# Header 1
## Header 2
### Header 3 with #valid_tag_in_header
#### Header 4
# 标题一
"#;
    let tags = extract_tags(content);
    assert_eq!(tags, vec!["#valid_tag_in_header"]);
}

#[test]
fn test_extract_tags_ignore_urls() {
    let content = r#"
Check this link: https://example.com#section-anchor and http://test.org/path#hash.
Markdown link: [documentation](https://docs.rs/pulldown-cmark#features)
Autolink: <https://github.com/foo/bar#readme>
Here is a #real_tag.
"#;
    let tags = extract_tags(content);
    assert_eq!(tags, vec!["#real_tag"]);
}

#[test]
fn test_extract_tags_ignore_code_blocks_and_inline() {
    let content = r#"
```python
# This is a python comment
#not_a_tag
class Foo: pass
```

And inline `#not_a_tag_either` here.
Real tags are #valid_tag1 and (#valid_tag2), plus #valid_tag3!
"#;
    let tags = extract_tags(content);
    assert_eq!(tags, vec!["#valid_tag1", "#valid_tag2", "#valid_tag3"]);
}

#[test]
fn test_extract_tags_punctuation_and_numbers() {
    let content =
        "Ignore pure numbers like #123 and #1 or empty #. But keep #tag1, #tag-two, #tag_three.";
    let tags = extract_tags(content);
    assert_eq!(tags, vec!["#tag1", "#tag-two", "#tag_three"]);
}

#[test]
fn test_parser_performance_1000_documents() {
    let sample_doc = r#"---
title: "Micro-bench Note"
hierarchy: L3Evergreen
tags:
  - test
  - benchmark
---
# Benchmark Heading

This document references [[CardOne]] and aliased [[CardTwo|Alias Two]].
It also has hierarchical tag #topic/subtopic/micro and #rust/perf.

```rust
// Fenced code block with fake link and fake tag
let fake = "[[FakeInsideCode]]";
// #fake_tag
```

Inline code: `[[FakeInline]]` and `#fake_inline`.
Final note link: [[CardThree/Subcard.md]].
"#;

    // Warm-up
    let _ = parse_frontmatter(sample_doc);
    let _ = extract_wikilinks(sample_doc);
    let _ = extract_tags(sample_doc);

    let start = Instant::now();
    for _ in 0..1000 {
        let (fm, body) = parse_frontmatter(sample_doc);
        assert!(!fm.is_null());
        let (links, tags) = extract_elements(body);
        assert_eq!(links.len(), 3);
        assert_eq!(tags.len(), 2);
    }
    let elapsed = start.elapsed();
    println!("Parsed 1,000 documents in {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 50,
        "Parsing 1,000 documents must complete in <50ms, took {:?}",
        elapsed
    );
}
