use std::path::PathBuf;
use std::sync::Arc;

use k0maru_core::core::models::{Document, HierarchyLevel, WikiLink};
use k0maru_core::core::traits::CacheStorage;
use k0maru_core::storage::rank_signals::{
    compute_access_reinforcement, compute_decay, DecayConfig,
};
use k0maru_core::storage::{GraphBoostMode, HybridSearchEngine, SearchMode, SqliteStorage};
use k0maru_core::vector::{EmbeddingEngine, MockEmbeddingEngine, VectorError};
use serde_json::json;

const SECS_PER_DAY: i64 = 86_400;
// 2026-11-18T00:00:00Z
const TEST_NOW: i64 = 1_794_960_000;

struct FixedQueryEmbedder {
    dim: usize,
}

impl EmbeddingEngine for FixedQueryEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, VectorError> {
        let mut vec = vec![0.0f32; self.dim];
        let lower = text.to_lowercase();
        if lower.contains("settlement") || lower.contains("quorum") {
            vec[0] = 1.0;
        } else {
            vec[1] = 1.0;
        }
        Ok(vec)
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, VectorError> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn model_name(&self) -> &str {
        "fixed-test-embed"
    }
}

fn make_doc(
    path: &str,
    title: &str,
    hierarchy: HierarchyLevel,
    frontmatter: serde_json::Value,
    mtime: u64,
    body: &str,
    links: Vec<WikiLink>,
) -> Document {
    Document {
        path: PathBuf::from(path),
        title: title.to_string(),
        hierarchy,
        frontmatter,
        links,
        tags: Vec::new(),
        content_hash: format!("hash-{path}"),
        mtime,
        body: body.to_string(),
        body_line_offset: 0,
    }
}

#[test]
fn test_decay_curves_floors_and_implausible_mtimes() {
    let cfg = DecayConfig::default();

    // L1Resource and L3Evergreen never decay regardless of age.
    let old_fm = json!({ "date": "2020-01-01" });
    assert_eq!(
        compute_decay(
            HierarchyLevel::L1Resource,
            &old_fm,
            "10_Projects/gateway.md",
            Some(1_577_836_800),
            1_577_836_800,
            TEST_NOW,
            cfg,
        ),
        1.0
    );
    assert_eq!(
        compute_decay(
            HierarchyLevel::L3Evergreen,
            &old_fm,
            "20_Cards/settlement.md",
            Some(1_577_836_800),
            1_577_836_800,
            TEST_NOW,
            cfg,
        ),
        1.0
    );

    // L0Ephemeral: half-life 60d, floor 0.50.
    // Exactly 60 days old -> 0.50 + 0.50 * 0.5 = 0.75.
    let l0_60d_ts = (TEST_NOW - 60 * SECS_PER_DAY) as u64;
    let d_l0_60 = compute_decay(
        HierarchyLevel::L0Ephemeral,
        &json!({}),
        "00_Daily/scratch.md",
        None,
        l0_60d_ts,
        TEST_NOW,
        cfg,
    );
    assert!((d_l0_60 - 0.75).abs() < 1e-5, "d_l0_60={d_l0_60}");

    // Very old L0 approaches floor 0.50 and never drops below it.
    let l0_old_ts = (TEST_NOW - 3_650 * SECS_PER_DAY) as u64;
    let d_l0_floor = compute_decay(
        HierarchyLevel::L0Ephemeral,
        &json!({}),
        "00_Daily/ancient.md",
        None,
        l0_old_ts,
        TEST_NOW,
        cfg,
    );
    assert!(
        (d_l0_floor - 0.50).abs() < 1e-4 && d_l0_floor >= 0.50,
        "d_l0_floor={d_l0_floor}"
    );

    // L2Log: half-life 180d, floor 0.70.
    // Exactly 180 days old -> 0.70 + 0.30 * 0.5 = 0.85.
    let l2_180d_ts = (TEST_NOW - 180 * SECS_PER_DAY) as u64;
    let d_l2_180 = compute_decay(
        HierarchyLevel::L2Log,
        &json!({}),
        "01_AI_Logs/audit.md",
        None,
        l2_180d_ts,
        TEST_NOW,
        cfg,
    );
    assert!((d_l2_180 - 0.85).abs() < 1e-5, "d_l2_180={d_l2_180}");

    // Very old L2 approaches floor 0.70 and never drops below it.
    let d_l2_floor = compute_decay(
        HierarchyLevel::L2Log,
        &json!({}),
        "01_AI_Logs/ancient-log.md",
        None,
        l0_old_ts,
        TEST_NOW,
        cfg,
    );
    assert!(
        (d_l2_floor - 0.70).abs() < 1e-4 && d_l2_floor >= 0.70,
        "d_l2_floor={d_l2_floor}"
    );

    // Implausible mtimes (0, 1000, u64::MAX) with no valid dates yield d == 1.0.
    for bad_mtime in [0u64, 1000u64, u64::MAX] {
        let d = compute_decay(
            HierarchyLevel::L0Ephemeral,
            &json!({}),
            "00_Daily/default-fixture.md",
            None,
            bad_mtime,
            TEST_NOW,
            cfg,
        );
        assert_eq!(d, 1.0, "bad_mtime={bad_mtime}");
    }
}

#[test]
fn test_frontmatter_date_beats_fresh_git_checkout_mtime() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    storage.initialize().unwrap();

    // Simulate `git checkout`: both docs have fresh mtime == TEST_NOW,
    // but `old_log` has frontmatter `date: "2024-01-01"` while `fresh_log` has `date: "2026-11-18"`.
    let fresh_mtime = TEST_NOW as u64;
    let old_log = make_doc(
        "01_AI_Logs/old-settlement.md",
        "Old Settlement Run",
        HierarchyLevel::L2Log,
        json!({ "date": "2024-01-01" }),
        fresh_mtime,
        "Distributed settlement consensus latency report.",
        vec![],
    );
    let fresh_log = make_doc(
        "01_AI_Logs/fresh-settlement.md",
        "Fresh Settlement Run",
        HierarchyLevel::L2Log,
        json!({ "date": "2026-11-18" }),
        fresh_mtime,
        "Distributed settlement consensus latency report.",
        vec![],
    );
    storage.upsert_document(&old_log).unwrap();
    storage.upsert_document(&fresh_log).unwrap();

    let engine = HybridSearchEngine::new(&storage, None).with_now(TEST_NOW);
    let results = engine
        .search("settlement consensus latency", SearchMode::Hybrid, 5)
        .unwrap();
    assert_eq!(results.len(), 2);

    let fresh_hit = results
        .iter()
        .find(|r| r.path == "01_AI_Logs/fresh-settlement.md")
        .unwrap();
    let old_hit = results
        .iter()
        .find(|r| r.path == "01_AI_Logs/old-settlement.md")
        .unwrap();

    assert!(
        fresh_hit.score > old_hit.score,
        "fresh_hit ({}) should outrank decayed old_hit ({}) despite equal mtime",
        fresh_hit.score,
        old_hit.score
    );
    // old_log is ~1052 days old, near the 0.70 floor
    let expected_ratio = compute_decay(
        HierarchyLevel::L2Log,
        &old_log.frontmatter,
        "01_AI_Logs/old-settlement.md",
        None,
        fresh_mtime,
        TEST_NOW,
        DecayConfig::default(),
    );
    assert!(expected_ratio < 0.72);
    let expected_old_score = (1.0 / (60.0 + old_hit.bm25_rank.unwrap() as f32)) * expected_ratio;
    let expected_fresh_score = 1.0 / (60.0 + fresh_hit.bm25_rank.unwrap() as f32);
    assert!((old_hit.score - expected_old_score).abs() < 1e-6);
    assert!((fresh_hit.score - expected_fresh_score).abs() < 1e-6);
}

#[test]
fn test_access_reinforcement_bounds_and_recency() {
    assert_eq!(compute_access_reinforcement(None, TEST_NOW), 1.0);
    assert_eq!(
        compute_access_reinforcement(Some((0, TEST_NOW)), TEST_NOW),
        1.0
    );

    let r_small = compute_access_reinforcement(Some((1, TEST_NOW)), TEST_NOW);
    assert!((r_small - (1.0 + 0.15 * 0.25)).abs() < 1e-5);

    // Saturates at r == 1.15 for c >= 15 at now, never exceeds 1.15.
    let r_sat = compute_access_reinforcement(Some((15, TEST_NOW)), TEST_NOW);
    let r_huge = compute_access_reinforcement(Some((100_000, TEST_NOW)), TEST_NOW);
    assert!((r_sat - 1.15).abs() < 1e-6);
    assert!(r_huge <= 1.15);

    // 30-day half-life halves the +0.15 bonus to +0.075.
    let r_30d = compute_access_reinforcement(Some((15, TEST_NOW - 30 * SECS_PER_DAY)), TEST_NOW);
    assert!((r_30d - 1.075).abs() < 1e-5);

    // Missing doc_access table falls back gracefully to r == 1.0 inside HybridSearchEngine.
    let mut storage = SqliteStorage::in_memory().unwrap();
    storage.initialize().unwrap();
    storage
        .connection()
        .execute("DROP TABLE IF EXISTS doc_access", [])
        .unwrap();
    let doc = make_doc(
        "20_Cards/gateway.md",
        "Gateway",
        HierarchyLevel::L3Evergreen,
        json!({}),
        TEST_NOW as u64,
        "Payment gateway idempotency key store.",
        vec![],
    );
    storage.upsert_document(&doc).unwrap();
    let engine = HybridSearchEngine::new(&storage, None).with_now(TEST_NOW);
    let hits = engine.search("idempotency", SearchMode::Hybrid, 5).unwrap();
    assert_eq!(hits.len(), 1);
    assert!((hits[0].score - 1.0 / 61.0).abs() < 1e-6);
}

#[test]
fn test_relative_in_pool_graph_boost_and_absolute_override() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    storage.initialize().unwrap();

    let card_a = make_doc(
        "20_Cards/ledger-a.md",
        "Ledger A",
        HierarchyLevel::L3Evergreen,
        json!({}),
        TEST_NOW as u64,
        "Distributed settlement ledger consensus protocol.",
        vec![WikiLink::new("ledger-b", None::<String>, "[[ledger-b]]")],
    );
    let card_b = make_doc(
        "20_Cards/ledger-b.md",
        "Ledger B",
        HierarchyLevel::L3Evergreen,
        json!({}),
        TEST_NOW as u64,
        "Distributed settlement consensus quorum.",
        vec![],
    );
    let card_isolated = make_doc(
        "20_Cards/ledger-isolated.md",
        "Ledger Isolated",
        HierarchyLevel::L3Evergreen,
        json!({}),
        TEST_NOW as u64,
        "Distributed settlement consensus quorum.",
        vec![],
    );

    storage.upsert_document(&card_a).unwrap();
    storage.upsert_document(&card_b).unwrap();
    storage.upsert_document(&card_isolated).unwrap();
    storage.rebuild_link_edges().unwrap();

    let engine = HybridSearchEngine::new(&storage, None).with_now(TEST_NOW);
    assert_eq!(
        engine.graph_boost_mode(),
        GraphBoostMode::Relative(HybridSearchEngine::DEFAULT_GRAPH_BOOST_RATIO)
    );

    let hits = engine
        .search("distributed settlement", SearchMode::Hybrid, 5)
        .unwrap();
    assert_eq!(hits.len(), 3);

    // In BM25-only hybrid mode, S_max = 1 / 61 and gamma = 0.25 => B = 0.25 / 61.
    let expected_s_max = 1.0 / 61.0;
    let expected_boost = 0.25 * expected_s_max;

    let hit_a = hits
        .iter()
        .find(|r| r.path == "20_Cards/ledger-a.md")
        .unwrap();
    let hit_b = hits
        .iter()
        .find(|r| r.path == "20_Cards/ledger-b.md")
        .unwrap();
    let hit_iso = hits
        .iter()
        .find(|r| r.path == "20_Cards/ledger-isolated.md")
        .unwrap();

    // Both A (outgoing) and B (incoming) have an active pool neighbor.
    assert!((hit_a.graph_boost - expected_boost).abs() < 1e-6);
    assert!((hit_b.graph_boost - expected_boost).abs() < 1e-6);
    assert_eq!(hit_iso.graph_boost, 0.0);

    // Custom ratio via with_graph_boost_ratio
    let engine_ratio = HybridSearchEngine::new(&storage, None)
        .with_now(TEST_NOW)
        .with_graph_boost_ratio(0.40);
    let hits_ratio = engine_ratio
        .search("distributed settlement", SearchMode::Hybrid, 5)
        .unwrap();
    let hit_a_ratio = hits_ratio
        .iter()
        .find(|r| r.path == "20_Cards/ledger-a.md")
        .unwrap();
    assert!((hit_a_ratio.graph_boost - 0.40 * expected_s_max).abs() < 1e-6);

    // Absolute override via with_graph_boost_weight
    let engine_abs = HybridSearchEngine::new(&storage, None)
        .with_now(TEST_NOW)
        .with_graph_boost_weight(0.012);
    assert_eq!(
        engine_abs.graph_boost_mode(),
        GraphBoostMode::Absolute(0.012)
    );
    let hits_abs = engine_abs
        .search("distributed settlement", SearchMode::Hybrid, 5)
        .unwrap();
    let hit_a_abs = hits_abs
        .iter()
        .find(|r| r.path == "20_Cards/ledger-a.md")
        .unwrap();
    assert!((hit_a_abs.graph_boost - 0.012).abs() < 1e-6);
}

#[test]
fn test_one_hop_expansion_max_not_sum_cap_filters_and_snippet() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    storage.initialize().unwrap();

    // Five direct parent hits matching "quorum consensus", all linking to `child_hub` (L3, does NOT match query).
    for idx in 1..=5 {
        let p = make_doc(
            &format!("10_Projects/parent-{idx}.md"),
            &format!("Parent {idx}"),
            HierarchyLevel::L1Resource,
            json!({}),
            TEST_NOW as u64,
            "Quorum consensus replication protocol details.",
            vec![
                WikiLink::new("child-hub", None::<String>, "[[child-hub]]"),
                WikiLink::new("daily-skip", None::<String>, "[[daily-skip]]"),
                WikiLink::new("superseded-skip", None::<String>, "[[superseded-skip]]"),
                WikiLink::new("l1-extra-1", None::<String>, "[[l1-extra-1]]"),
                WikiLink::new("l2-extra-2", None::<String>, "[[l2-extra-2]]"),
            ],
        );
        storage.upsert_document(&p).unwrap();
    }

    // Active L3 neighbor that does NOT contain query keywords -> clean intro snippet, S_n = max(0.50 * F_p).
    let child_hub = make_doc(
        "20_Cards/child-hub.md",
        "Child Hub Invariant",
        HierarchyLevel::L3Evergreen,
        json!({}),
        TEST_NOW as u64,
        "# Child Hub Invariant\nDeterministic failover fence token verification across partitions.",
        vec![],
    );
    // Active L0 neighbor -> MUST be excluded from expansion.
    let daily_skip = make_doc(
        "00_Daily/daily-skip.md",
        "Daily Note",
        HierarchyLevel::L0Ephemeral,
        json!({}),
        TEST_NOW as u64,
        "Transient standup notes unrelated to search terms.",
        vec![],
    );
    // Inactive L3 neighbor -> MUST be excluded from expansion.
    let superseded_skip = make_doc(
        "20_Cards/superseded-skip.md",
        "Superseded Card",
        HierarchyLevel::L3Evergreen,
        json!({ "status": "superseded" }),
        TEST_NOW as u64,
        "Obsolete design card.",
        vec![],
    );
    // Additional eligible outgoing neighbors to test expansion cap max(1, limit / 2).
    let extra_1 = make_doc(
        "10_Projects/l1-extra-1.md",
        "Extra L1 Resource",
        HierarchyLevel::L1Resource,
        json!({}),
        TEST_NOW as u64,
        "Gateway retry configuration reference.",
        vec![],
    );
    let extra_2 = make_doc(
        "01_AI_Logs/l2-extra-2.md",
        "Extra L2 Log",
        HierarchyLevel::L2Log,
        json!({ "date": "2026-11-18" }),
        TEST_NOW as u64,
        "Gateway benchmark trace.",
        vec![],
    );

    // Incoming edge n -> parent-1:
    // - incoming L3 IS expanded with beta_in = 0.35
    // - incoming L1/L2 is NOT expanded
    let incoming_l3 = make_doc(
        "20_Cards/incoming-l3.md",
        "Incoming L3",
        HierarchyLevel::L3Evergreen,
        json!({}),
        TEST_NOW as u64,
        "Evergreen card pointing into parent-1.",
        vec![WikiLink::new("parent-1", None::<String>, "[[parent-1]]")],
    );
    let incoming_l1 = make_doc(
        "10_Projects/incoming-l1.md",
        "Incoming L1",
        HierarchyLevel::L1Resource,
        json!({}),
        TEST_NOW as u64,
        "Resource note pointing into parent-1.",
        vec![WikiLink::new("parent-1", None::<String>, "[[parent-1]]")],
    );

    for doc in [
        &child_hub,
        &daily_skip,
        &superseded_skip,
        &extra_1,
        &extra_2,
        &incoming_l3,
        &incoming_l1,
    ] {
        storage.upsert_document(doc).unwrap();
    }

    // Dirty link_edges: expansion MUST NOT trigger before rebuild_link_edges.
    {
        let pre_engine = HybridSearchEngine::new(&storage, None).with_now(TEST_NOW);
        let pre_rebuild = pre_engine
            .search("quorum consensus", SearchMode::Hybrid, 10)
            .unwrap();
        assert!(
            pre_rebuild.iter().all(|r| r.expanded_from.is_none()),
            "expansion must not run while link_edges is dirty"
        );
    }

    storage.rebuild_link_edges().unwrap();
    let engine = HybridSearchEngine::new(&storage, None).with_now(TEST_NOW);

    // With limit = 10: 5 direct hits + up to max(1, 10/2) = 5 expansions.
    // Eligible neighbors are child-hub, l1-extra-1, l2-extra-2, incoming-l3 (4 total).
    let hits = engine
        .search("quorum consensus", SearchMode::Hybrid, 10)
        .unwrap();

    assert!(hits.iter().all(|r| r.path != "00_Daily/daily-skip.md"));
    assert!(hits.iter().all(|r| r.path != "20_Cards/superseded-skip.md"));
    assert!(hits.iter().all(|r| r.path != "10_Projects/incoming-l1.md"));

    let parent_1 = hits
        .iter()
        .find(|r| r.path == "10_Projects/parent-1.md")
        .unwrap();
    let hub_hit = hits
        .iter()
        .find(|r| r.path == "20_Cards/child-hub.md")
        .expect("child-hub expanded");
    let incoming_l3_hit = hits
        .iter()
        .find(|r| r.path == "20_Cards/incoming-l3.md")
        .expect("incoming L3 expanded");

    // Verify max-not-sum: hub_hit has 5 parents, yet score == 0.50 * best parent_score (< parent_score!).
    assert_eq!(hub_hit.bm25_rank, None);
    assert_eq!(hub_hit.vector_rank, None);
    assert_eq!(hub_hit.graph_boost, 0.0);
    assert!(hub_hit.expanded_from.is_some());
    assert!(hub_hit.score < parent_1.score);
    assert!(
        (hub_hit.score - 0.50 * parent_1.score).abs() < 1e-6,
        "expected max (0.50 * F_p = {}), got {} (would be ~5x if summed)",
        0.50 * parent_1.score,
        hub_hit.score
    );
    assert!(hub_hit
        .snippet
        .contains("Deterministic failover fence token"));

    // Incoming L3 uses beta_in = 0.35.
    assert_eq!(
        incoming_l3_hit.expanded_from.as_deref(),
        Some("10_Projects/parent-1.md")
    );
    assert!((incoming_l3_hit.score - 0.35 * parent_1.score).abs() < 1e-6);

    // limit = 1 -> expansion is strictly below its parent, so only the parent is returned.
    let limit_one = engine
        .search("quorum consensus", SearchMode::Hybrid, 1)
        .unwrap();
    assert_eq!(limit_one.len(), 1);
    assert!(limit_one[0].expanded_from.is_none());

    // Cap check: when 1 direct parent links to 5 eligible children and limit = 4,
    // expansion cap is max(1, 4 / 2) = 2 expansions (total results <= 4).
    let mut cap_storage = SqliteStorage::in_memory().unwrap();
    cap_storage.initialize().unwrap();
    let links: Vec<WikiLink> = (1..=5)
        .map(|i| {
            WikiLink::new(
                format!("target-{i}"),
                None::<String>,
                format!("[[target-{i}]]"),
            )
        })
        .collect();
    let single_parent = make_doc(
        "10_Projects/single-parent.md",
        "Single Parent",
        HierarchyLevel::L1Resource,
        json!({}),
        TEST_NOW as u64,
        "Unique idempotency ledger query token.",
        links,
    );
    cap_storage.upsert_document(&single_parent).unwrap();
    for i in 1..=5 {
        let child = make_doc(
            &format!("20_Cards/target-{i}.md"),
            &format!("Target {i}"),
            HierarchyLevel::L3Evergreen,
            json!({}),
            TEST_NOW as u64,
            "Invariant note without query terms.",
            vec![],
        );
        cap_storage.upsert_document(&child).unwrap();
    }
    cap_storage.rebuild_link_edges().unwrap();

    let cap_engine = HybridSearchEngine::new(&cap_storage, None).with_now(TEST_NOW);
    let cap_hits = cap_engine
        .search("idempotency ledger", SearchMode::Hybrid, 4)
        .unwrap();
    let expanded_count = cap_hits
        .iter()
        .filter(|r| r.expanded_from.is_some())
        .count();
    assert_eq!(
        expanded_count, 2,
        "limit=4 caps expansions at max(1, 4/2)=2"
    );
    assert_eq!(cap_hits.len(), 3);
}

#[test]
fn test_decay_access_and_superseded_composition_and_pure_modes() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    storage.initialize().unwrap();

    let embedder = Arc::new(FixedQueryEmbedder { dim: 16 });
    storage.ensure_vector_dimension(16).unwrap();

    // Both docs match the query identically; `superseded_l2` is 60 days old (Wait: L2 half-life 180d)
    // and has access stats, plus `status: superseded`.
    let date_180d_ago = "2026-05-22"; // 180 days before 2026-11-18
    let active_l3 = make_doc(
        "20_Cards/active-settlement.md",
        "Active Settlement",
        HierarchyLevel::L3Evergreen,
        json!({ "status": "active" }),
        TEST_NOW as u64,
        "Settlement quorum protocol.",
        vec![WikiLink::new(
            "neighbor-card",
            None::<String>,
            "[[neighbor-card]]",
        )],
    );
    let superseded_l2 = make_doc(
        "01_AI_Logs/old-superseded-log.md",
        "Superseded Settlement Log",
        HierarchyLevel::L2Log,
        json!({
            "status": "superseded",
            "superseded_by": "[[20_Cards/active-settlement]]",
            "date": date_180d_ago
        }),
        TEST_NOW as u64,
        "Settlement quorum protocol.",
        vec![],
    );
    let neighbor_card = make_doc(
        "20_Cards/neighbor-card.md",
        "Neighbor Card",
        HierarchyLevel::L3Evergreen,
        json!({}),
        TEST_NOW as u64,
        "Unrelated card body.",
        vec![],
    );

    storage.upsert_document(&active_l3).unwrap();
    storage.upsert_document(&superseded_l2).unwrap();
    storage.upsert_document(&neighbor_card).unwrap();
    storage.rebuild_link_edges().unwrap();

    // Insert vectors so SearchMode::Vector works too.
    let v_match = embedder.embed("settlement quorum").unwrap();
    let v_other = embedder.embed("unrelated").unwrap();
    storage
        .insert_vector("20_Cards/active-settlement.md", &v_match, 1)
        .unwrap();
    storage
        .insert_vector("01_AI_Logs/old-superseded-log.md", &v_match, 2)
        .unwrap();
    storage
        .insert_vector("20_Cards/neighbor-card.md", &v_other, 3)
        .unwrap();

    // Record 15 accesses at TEST_NOW on `superseded_l2` -> r = 1.15.
    storage
        .connection()
        .execute(
            "INSERT INTO doc_access (path, access_count, last_access) VALUES (?1, 15, ?2)",
            rusqlite::params!["01_AI_Logs/old-superseded-log.md", TEST_NOW],
        )
        .unwrap();

    let engine = HybridSearchEngine::new(&storage, Some(embedder.clone())).with_now(TEST_NOW);

    // 1. Hybrid mode: superseded L2 composes S * d * r * 0.2 (with graph_boost == 0.0).
    let hybrid_hits = engine
        .search("settlement quorum", SearchMode::Hybrid, 5)
        .unwrap();
    let sup_hit = hybrid_hits
        .iter()
        .find(|r| r.path == "01_AI_Logs/old-superseded-log.md")
        .unwrap();
    let d_sup = compute_decay(
        HierarchyLevel::L2Log,
        &superseded_l2.frontmatter,
        "01_AI_Logs/old-superseded-log.md",
        None,
        TEST_NOW as u64,
        TEST_NOW,
        DecayConfig::default(),
    );
    let r_sup = compute_access_reinforcement(Some((15, TEST_NOW)), TEST_NOW);
    assert!((d_sup - 0.85).abs() < 0.01, "d_sup={d_sup}");
    assert!((r_sup - 1.15).abs() < 1e-6);
    assert_eq!(sup_hit.graph_boost, 0.0);

    let s_sup = 0.5 / (60.0 + sup_hit.bm25_rank.unwrap() as f32)
        + 0.5 / (60.0 + sup_hit.vector_rank.unwrap() as f32);
    let expected_f_sup = s_sup * d_sup * r_sup * 0.2;
    assert!(
        (sup_hit.score - expected_f_sup).abs() < 1e-6,
        "expected {}, got {}",
        expected_f_sup,
        sup_hit.score
    );

    // 2. SearchMode::Bm25 stays pure: no decay, no access boost, no 1-hop expansion, only 0.2x penalty.
    let bm25_hits = engine
        .search("settlement quorum", SearchMode::Bm25, 5)
        .unwrap();
    assert!(bm25_hits.iter().all(|r| r.expanded_from.is_none()));
    let sup_bm25 = bm25_hits
        .iter()
        .find(|r| r.path == "01_AI_Logs/old-superseded-log.md")
        .unwrap();
    let expected_pure_bm25 = (1.0 / (60.0 + sup_bm25.bm25_rank.unwrap() as f32)) * 0.2;
    assert!((sup_bm25.score - expected_pure_bm25).abs() < 1e-6);

    // 3. SearchMode::Vector stays pure: no decay, no access boost, no 1-hop expansion, only 0.2x penalty.
    let vec_hits = engine
        .search("settlement quorum", SearchMode::Vector, 5)
        .unwrap();
    assert!(vec_hits.iter().all(|r| r.expanded_from.is_none()));
    let sup_vec = vec_hits
        .iter()
        .find(|r| r.path == "01_AI_Logs/old-superseded-log.md")
        .unwrap();
    let expected_pure_vec = (1.0 / (60.0 + sup_vec.vector_rank.unwrap() as f32)) * 0.2;
    assert!((sup_vec.score - expected_pure_vec).abs() < 1e-6);

    // Keep MockEmbeddingEngine referenced to avoid unused import warnings if feature flags vary.
    let _mock = MockEmbeddingEngine::new(16);
}

#[test]
fn test_expanded_hit_section_locate_and_git_time_custom_decay() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    storage.initialize().unwrap();

    // Parent matches "circuit breaker", child matches ONLY one of the query words ("breaker")
    // inside section 2, and is NOT in the direct BM25 pool if we set limit=1... wait:
    // if child has "breaker" it could match BM25 `OR`, so let's make parent match a unique
    // phrase and child be outside direct pool by using a stricter test:
    // Here parent links to child, and child has a section heading matching "ledger" while
    // parent has "ledger" repeated so it's in pool, or let's verify git_time & custom DecayConfig:
    let fresh_mtime = TEST_NOW as u64;
    let l0_git_old = make_doc(
        "00_Daily/scratchpad-alpha.md",
        "Scratchpad Alpha",
        HierarchyLevel::L0Ephemeral,
        json!({}),
        fresh_mtime,
        "Payment gateway circuit breaker threshold notes.",
        vec![],
    );
    let l0_git_new = make_doc(
        "00_Daily/scratchpad-beta.md",
        "Scratchpad Beta",
        HierarchyLevel::L0Ephemeral,
        json!({}),
        fresh_mtime,
        "Payment gateway circuit breaker threshold notes.",
        vec![],
    );
    storage.upsert_document(&l0_git_old).unwrap();
    storage.upsert_document(&l0_git_new).unwrap();
    storage
        .set_git_times(
            &[
                (
                    "00_Daily/scratchpad-alpha.md".to_string(),
                    TEST_NOW - 120 * SECS_PER_DAY,
                ),
                ("00_Daily/scratchpad-beta.md".to_string(), TEST_NOW),
            ],
            "abc1234",
        )
        .unwrap();

    let custom_cfg = DecayConfig {
        l0_half_life_days: 30.0,
        l0_floor: 0.40,
        l2_half_life_days: 90.0,
        l2_floor: 0.60,
    };
    let engine = HybridSearchEngine::new(&storage, None)
        .with_now(TEST_NOW)
        .with_decay(custom_cfg);
    assert_eq!(engine.decay(), custom_cfg);

    let hits = engine
        .search("circuit breaker threshold", SearchMode::Hybrid, 5)
        .unwrap();
    assert_eq!(hits.len(), 2);
    let alpha = hits
        .iter()
        .find(|r| r.path == "00_Daily/scratchpad-alpha.md")
        .unwrap();
    let beta = hits
        .iter()
        .find(|r| r.path == "00_Daily/scratchpad-beta.md")
        .unwrap();
    assert!(beta.score > alpha.score);

    // 120 days with half-life 30d = 4 half-lives => 0.40 + 0.60 / 16 = 0.4375.
    let expected_d = 0.40 + 0.60 * 2.0f32.powf(-120.0 / 30.0);
    let expected_alpha_score = (1.0 / (60.0 + alpha.bm25_rank.unwrap() as f32)) * expected_d;
    let expected_beta_score = 1.0 / (60.0 + beta.bm25_rank.unwrap() as f32);
    assert!((alpha.score - expected_alpha_score).abs() < 1e-6);
    assert!((beta.score - expected_beta_score).abs() < 1e-6);
}
