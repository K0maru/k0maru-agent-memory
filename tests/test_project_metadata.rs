use k0maru::core::models::{Document, HierarchyLevel};
use k0maru::loadout::metadata::{
    extract_derived_aliases, extract_project_metadata, is_project_match, normalize_identifier,
};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn test_normalize_identifier() {
    assert_eq!(
        normalize_identifier("okx-funding-arbitrage"),
        "okx funding arbitrage"
    );
    assert_eq!(
        normalize_identifier("okx_funding_arbitrage"),
        "okx funding arbitrage"
    );
    assert_eq!(
        normalize_identifier("OKX: Funding Arbitrage!"),
        "okx funding arbitrage"
    );
    assert_eq!(
        normalize_identifier("OKX资金费率套利与跨生态对冲系统"),
        "okx资金费率套利与跨生态对冲系统"
    );
    assert_eq!(
        normalize_identifier("  topic/funding-arbitrage/  "),
        "topic funding arbitrage"
    );
}

#[test]
fn test_extract_derived_aliases() {
    let source_stem = "2026-09-07-okx-funding-arbitrage-phase-5-live-tui-monitor";
    let derived = extract_derived_aliases(source_stem);
    assert!(
        derived.iter().any(|a| a == "okx-funding-arbitrage"),
        "Derived aliases should contain 'okx-funding-arbitrage', got: {:?}",
        derived
    );

    let simple_log = "2026-10-01-quant-trader-audit";
    let derived_simple = extract_derived_aliases(simple_log);
    assert!(
        derived_simple.iter().any(|a| a == "quant-trader"),
        "Derived aliases should contain 'quant-trader', got: {:?}",
        derived_simple
    );
}

#[test]
fn test_extract_project_metadata() {
    let doc = Document {
        path: PathBuf::from("10_Projects/OKX资金费率套利与跨生态对冲系统.md"),
        title: "OKX资金费率套利与跨生态对冲系统".to_string(),
        hierarchy: HierarchyLevel::L1Resource,
        frontmatter: json!({
            "status": "active",
            "aliases": ["funding-arbitrage", "okx-arb"],
            "slug": "okx-funding-arbitrage",
            "repo": "k0maru3/okx-funding-arbitrage",
            "tags": ["topic/funding-arbitrage", "project/crypto"]
        }),
        links: vec![],
        tags: vec![
            "topic/funding-arbitrage".to_string(),
            "project/crypto".to_string(),
        ],
        content_hash: "hash123".to_string(),
        mtime: 1774843200,
        body: "# OKX资金费率套利与跨生态对冲系统\n> 跨生态期现套利系统".to_string(),
    };

    let meta = extract_project_metadata(&doc, &["okx-funding-arbitrage".to_string()]);
    assert_eq!(meta.stem, "OKX资金费率套利与跨生态对冲系统");
    assert_eq!(meta.title, "OKX资金费率套利与跨生态对冲系统");
    assert_eq!(meta.slug.as_deref(), Some("okx-funding-arbitrage"));
    assert!(meta.aliases.contains(&"funding-arbitrage".to_string()));
    assert!(meta.aliases.contains(&"okx-arb".to_string()));
    assert!(meta.tags.contains(&"funding-arbitrage".to_string()));
    assert!(meta.tags.contains(&"crypto".to_string()));
    assert!(meta
        .derived_aliases
        .contains(&"okx-funding-arbitrage".to_string()));
}

#[test]
fn test_is_project_match_kebab_against_chinese() {
    let doc = Document {
        path: PathBuf::from("10_Projects/OKX资金费率套利与跨生态对冲系统.md"),
        title: "OKX资金费率套利与跨生态对冲系统".to_string(),
        hierarchy: HierarchyLevel::L1Resource,
        frontmatter: json!({
            "status": "active",
            "aliases": ["funding-arbitrage"],
            "slug": "okx-funding-arbitrage",
            "tags": ["topic/funding-arbitrage"]
        }),
        links: vec![],
        tags: vec!["topic/funding-arbitrage".to_string()],
        content_hash: "hash123".to_string(),
        mtime: 1774843200,
        body: "OKX funding arbitrage engine".to_string(),
    };

    let meta = extract_project_metadata(&doc, &["okx-funding-arbitrage".to_string()]);

    // 1. Slug query
    assert!(is_project_match("okx-funding-arbitrage", &meta));

    // 2. English alias query
    assert!(is_project_match("funding-arbitrage", &meta));

    // 3. Partial keyword
    assert!(is_project_match("OKX", &meta));
    assert!(is_project_match("资金费率", &meta));

    // 4. Snake case variant
    assert!(is_project_match("okx_funding_arbitrage", &meta));

    // 5. Negative queries should NOT match
    assert!(!is_project_match("binance-futures", &meta));
    assert!(!is_project_match("arbitrage-doge", &meta));
}
