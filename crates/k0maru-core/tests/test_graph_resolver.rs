//! Integration tests for WikiLink graph resolution and pure-Rust civil date parsing.

use k0maru_core::core::dates::{
    days_from_civil, extract_reference_timestamp, is_plausible_timestamp,
    parse_first_iso_date_unix, MIN_PLAUSIBLE_TIMESTAMP,
};
use k0maru_core::graph::{LinkResolver, Resolution};
use k0maru_core::parser::extract_wikilinks;
use serde_json::json;

fn sample_resolver() -> LinkResolver {
    LinkResolver::from_docs(vec![
        (
            1,
            "10_Projects/payment-gateway.md".to_string(),
            "Payment Gateway Architecture".to_string(),
        ),
        (
            2,
            "20_Cards/distributed-settlement.md".to_string(),
            "Distributed Settlement".to_string(),
        ),
        (
            3,
            "10_Projects/shared.md".to_string(),
            "Project Shared Notes".to_string(),
        ),
        (
            4,
            "a/shared.md".to_string(),
            "Short Shared Notes".to_string(),
        ),
        (
            5,
            "archive/deep/shared.md".to_string(),
            "Archived Shared Notes".to_string(),
        ),
        (
            6,
            "team/alpha/sub/ledger.md".to_string(),
            "Alpha Ledger".to_string(),
        ),
        (
            7,
            "team/bravo/sub/ledger.md".to_string(),
            "Bravo Ledger".to_string(),
        ),
        (
            8,
            "20_Cards/adr-042.v2.md".to_string(),
            "ADR-042 Revision 2".to_string(),
        ),
        (
            9,
            "10_Projects/alpha-title-only.md".to_string(),
            "Gateway".to_string(),
        ),
        (
            10,
            "20_Cards/gateway.md".to_string(),
            "Gateway Card".to_string(),
        ),
        (11, "root_only.md".to_string(), "Root Note".to_string()),
        (
            12,
            "sub/root_only.md".to_string(),
            "Sub Root Note".to_string(),
        ),
    ])
}

#[test]
fn resolver_exact_path_and_normalization() {
    let r = sample_resolver();
    let src = "20_Cards/distributed-settlement.md";

    assert_eq!(
        r.resolve(src, "10_Projects/payment-gateway"),
        Resolution::Doc(1)
    );
    assert_eq!(
        r.resolve(src, "10_Projects/payment-gateway.md"),
        Resolution::Doc(1)
    );
    assert_eq!(
        r.resolve(src, "  10_PROJECTS\\Payment-Gateway.MD  "),
        Resolution::Doc(1)
    );
    assert_eq!(
        r.resolve(src, "./10_Projects/payment-gateway"),
        Resolution::Doc(1)
    );
    assert_eq!(
        r.resolve(src, "/10_Projects/payment-gateway"),
        Resolution::Doc(1)
    );
}

#[test]
fn resolver_subpath_suffix_and_stem_priority() {
    let r = sample_resolver();
    let src = "20_Cards/distributed-settlement.md";

    // Subpath suffix distinguishes alpha/sub/ledger from bravo/sub/ledger
    assert_eq!(r.resolve(src, "alpha/sub/ledger"), Resolution::Doc(6));
    assert_eq!(r.resolve(src, "bravo/sub/ledger.md"), Resolution::Doc(7));

    // Segment boundary check: "xalpha/sub/ledger" must NOT match "team/alpha/sub/ledger" via suffix,
    // so it falls through to by_stem["ledger"] and tie-breaks lexicographically (alpha < bravo).
    assert_eq!(r.resolve(src, "xalpha/sub/ledger"), Resolution::Doc(6));

    // Exact root path beats same-parent stem match (step 3 before step 5)
    assert_eq!(r.resolve("sub/other.md", "root_only"), Resolution::Doc(11));
    assert_eq!(
        r.resolve("sub/other.md", "sub/root_only"),
        Resolution::Doc(12)
    );

    // Stem match beats title match ("20_Cards/gateway.md" vs title "Gateway")
    assert_eq!(r.resolve(src, "gateway"), Resolution::Doc(10));

    // Title compatibility fallback
    assert_eq!(
        r.resolve(src, "Payment Gateway Architecture"),
        Resolution::Doc(1)
    );
    assert_eq!(
        r.resolve(src, "payment gateway architecture"),
        Resolution::Doc(1)
    );
}

#[test]
fn resolver_ambiguity_tie_breakers() {
    let r = sample_resolver();

    // Same parent dir ("10_Projects") beats shorter path ("a/shared.md")
    assert_eq!(
        r.resolve("10_Projects/payment-gateway.md", "shared"),
        Resolution::Doc(3)
    );

    // Unrelated parent dir falls back to shortest path ("a/shared" len 8 < "10_Projects/shared" < "archive/deep/shared")
    assert_eq!(
        r.resolve("20_Cards/distributed-settlement.md", "shared"),
        Resolution::Doc(4)
    );

    // Equal parent-dir mismatch and equal path length: lexicographic ("team/alpha/sub/ledger" < "team/bravo/sub/ledger")
    assert_eq!(
        r.resolve("20_Cards/distributed-settlement.md", "ledger"),
        Resolution::Doc(6)
    );

    // Same parent dir wins for ledger too
    assert_eq!(
        r.resolve("team/bravo/sub/notes.md", "ledger"),
        Resolution::Doc(7)
    );
}

#[test]
fn resolver_fragments_blocks_and_self_refs() {
    let r = sample_resolver();
    let src = "10_Projects/payment-gateway.md";

    // Heading & block fragment-only targets are self-references
    assert_eq!(r.resolve(src, "#Overview"), Resolution::SelfRef);
    assert_eq!(r.resolve(src, "^block-abc"), Resolution::SelfRef);
    assert_eq!(r.resolve(src, "  #Section 1  "), Resolution::SelfRef);

    // Doc targets with fragments strip `#...` and `^...`
    assert_eq!(
        r.resolve(src, "distributed-settlement#Consistency"),
        Resolution::Doc(2)
    );
    assert_eq!(
        r.resolve(src, "20_Cards/distributed-settlement.md#Consistency"),
        Resolution::Doc(2)
    );
    assert_eq!(
        r.resolve(src, "distributed-settlement^blk-9"),
        Resolution::Doc(2)
    );
    assert_eq!(
        r.resolve(src, "distributed-settlement#Heading^blk-9"),
        Resolution::Doc(2)
    );

    // Resolving to self doc ID returns SelfRef across path, stem, and title
    assert_eq!(
        r.resolve(src, "10_Projects/payment-gateway"),
        Resolution::SelfRef
    );
    assert_eq!(r.resolve(src, "payment-gateway"), Resolution::SelfRef);
    assert_eq!(r.resolve(src, "payment-gateway#API"), Resolution::SelfRef);
    assert_eq!(
        r.resolve(src, "Payment Gateway Architecture"),
        Resolution::SelfRef
    );

    // Ambiguous stem where same-parent tie-break picks self -> SelfRef
    assert_eq!(
        r.resolve("10_Projects/shared.md", "shared"),
        Resolution::SelfRef
    );
}

#[test]
fn resolver_assets_and_broken_links() {
    let r = sample_resolver();
    let src = "10_Projects/payment-gateway.md";

    let asset_targets = [
        "diagram.png",
        "photo.JPG",
        "scan.jpeg",
        "anim.gif",
        "hero.webp",
        "logo.svg",
        "docs/whitepaper.pdf#page=2",
        "boards/architecture.canvas",
        "sketches/flow.excalidraw",
        "sketches/flow.excalidraw.md",
        "demo.mp4",
        "clip.webm",
        "audio.mp3",
        "recording.wav",
        "bundle.zip",
        "metrics/latency.csv",
    ];
    for target in asset_targets {
        assert_eq!(
            r.resolve(src, target),
            Resolution::Asset,
            "expected Asset for {target}"
        );
    }

    // Dotted markdown doc names are NOT treated as assets
    assert_eq!(r.resolve(src, "adr-042.v2"), Resolution::Doc(8));
    assert_eq!(r.resolve(src, "adr-042.v2.md"), Resolution::Doc(8));
    assert_eq!(r.resolve(src, "node.js"), Resolution::Broken);

    // Missing / empty targets are Broken
    assert_eq!(r.resolve(src, "nonexistent-card"), Resolution::Broken);
    assert_eq!(r.resolve(src, ""), Resolution::Broken);
    assert_eq!(r.resolve(src, "   "), Resolution::Broken);
}

#[test]
fn resolver_integrates_with_extract_wikilinks() {
    let r = sample_resolver();
    let md = "\
See [[20_Cards/distributed-settlement#Flow|Settlement Flow]], \
![[assets/topology.png]], [[#Local Section]], [[missing-note]].";
    let links = extract_wikilinks(md);
    let resolutions: Vec<Resolution> = links
        .iter()
        .map(|l| r.resolve("10_Projects/payment-gateway.md", &l.target))
        .collect();

    assert_eq!(
        resolutions,
        vec![
            Resolution::Doc(2),
            Resolution::Asset,
            Resolution::SelfRef,
            Resolution::Broken,
        ]
    );
}

#[test]
fn dates_days_from_civil_and_iso_parsing() {
    assert_eq!(days_from_civil(1970, 1, 1), 0);
    assert_eq!(days_from_civil(1969, 12, 31), -1);
    assert_eq!(
        days_from_civil(2000, 1, 1) * 86_400,
        MIN_PLAUSIBLE_TIMESTAMP
    );

    // Leap-year calendar arithmetic
    assert_eq!(
        days_from_civil(2024, 3, 1) - days_from_civil(2024, 2, 28),
        2
    );
    assert_eq!(
        days_from_civil(2000, 3, 1) - days_from_civil(2000, 2, 28),
        2
    );
    assert_eq!(
        days_from_civil(1900, 3, 1) - days_from_civil(1900, 2, 28),
        1
    );

    let expected_sep29 = days_from_civil(2026, 9, 29) * 86_400;
    assert_eq!(
        parse_first_iso_date_unix("2026-09-29"),
        Some(expected_sep29)
    );
    assert_eq!(
        parse_first_iso_date_unix("[[2026-09-29]]"),
        Some(expected_sep29)
    );
    assert_eq!(
        parse_first_iso_date_unix("[[00_Daily/2026/2026-11-18]]"),
        Some(days_from_civil(2026, 11, 18) * 86_400)
    );
    assert_eq!(
        parse_first_iso_date_unix("2026-10-01-phase4-notes"),
        Some(days_from_civil(2026, 10, 1) * 86_400)
    );
    assert_eq!(
        parse_first_iso_date_unix("2026-09-29T18:30:00Z"),
        Some(expected_sep29)
    );

    // Invalid dates & leap-year boundaries
    assert_eq!(parse_first_iso_date_unix("2026-02-29"), None);
    assert_eq!(parse_first_iso_date_unix("2100-02-29"), None);
    assert_eq!(parse_first_iso_date_unix("2026-13-01"), None);
    assert_eq!(parse_first_iso_date_unix("2026-00-10"), None);
    assert_eq!(parse_first_iso_date_unix("2026-04-31"), None);
    assert_eq!(parse_first_iso_date_unix("2026-01-00"), None);
    assert_eq!(parse_first_iso_date_unix("12345-01-01"), None);
    assert_eq!(parse_first_iso_date_unix("2026-01-015"), None);

    // Valid leap days
    assert_eq!(
        parse_first_iso_date_unix("2024-02-29"),
        Some(days_from_civil(2024, 2, 29) * 86_400)
    );
    assert_eq!(
        parse_first_iso_date_unix("2000-02-29"),
        Some(days_from_civil(2000, 2, 29) * 86_400)
    );

    // Skips invalid candidate and finds first valid date
    assert_eq!(
        parse_first_iso_date_unix("bad 2026-02-29 then 2024-02-29"),
        Some(days_from_civil(2024, 2, 29) * 86_400)
    );
}

#[test]
fn dates_plausibility_and_reference_timestamp_priority() {
    let now = days_from_civil(2026, 11, 18) * 86_400;

    // Plausibility checks
    assert!(!is_plausible_timestamp(0, now));
    assert!(!is_plausible_timestamp(1000, now));
    assert!(!is_plausible_timestamp(-1, now));
    assert!(!is_plausible_timestamp(MIN_PLAUSIBLE_TIMESTAMP - 1, now));
    assert!(is_plausible_timestamp(MIN_PLAUSIBLE_TIMESTAMP, now));
    assert!(is_plausible_timestamp(now + 86_400, now));
    assert!(!is_plausible_timestamp(now + 86_401, now));

    let ts_date = days_from_civil(2026, 9, 29) * 86_400;
    let ts_created = days_from_civil(2026, 8, 1) * 86_400;
    let ts_stem = days_from_civil(2026, 7, 1) * 86_400;
    let ts_git = days_from_civil(2026, 6, 1) * 86_400;
    let ts_mtime = (days_from_civil(2026, 11, 18) * 86_400) as u64;

    // 1. frontmatter["date"] wins over created, stem, git_time, and fresh checkout mtime
    assert_eq!(
        extract_reference_timestamp(
            &json!({ "date": "[[2026-09-29]]", "created": "2026-08-01" }),
            "2026-07-01-settlement",
            Some(ts_git),
            ts_mtime,
            now,
        ),
        Some(ts_date)
    );

    // 2. frontmatter["created"] wins when date is missing or implausible
    assert_eq!(
        extract_reference_timestamp(
            &json!({ "date": "1995-01-01", "created": "2026-08-01" }),
            "2026-07-01-settlement",
            Some(ts_git),
            ts_mtime,
            now,
        ),
        Some(ts_created)
    );

    // 3. file_stem wins when frontmatter has no valid date/created
    assert_eq!(
        extract_reference_timestamp(
            &json!({}),
            "2026-07-01-settlement",
            Some(ts_git),
            ts_mtime,
            now,
        ),
        Some(ts_stem)
    );

    // 4. git_time wins over mtime when stem has no ISO date
    assert_eq!(
        extract_reference_timestamp(&json!({}), "settlement-card", Some(ts_git), ts_mtime, now),
        Some(ts_git)
    );

    // 5. mtime fallback when git_time is None or implausible sentinel
    assert_eq!(
        extract_reference_timestamp(&json!({}), "settlement-card", Some(-1), ts_mtime, now),
        Some(ts_mtime as i64)
    );

    // Sentinel rejections: 0, 1000, u64::MAX, -1 -> None
    assert_eq!(
        extract_reference_timestamp(&json!({}), "settlement-card", Some(-1), 0, now),
        None
    );
    assert_eq!(
        extract_reference_timestamp(&json!({}), "settlement-card", Some(0), 1000, now),
        None
    );
    assert_eq!(
        extract_reference_timestamp(&json!({}), "settlement-card", Some(1000), u64::MAX, now),
        None
    );
}
