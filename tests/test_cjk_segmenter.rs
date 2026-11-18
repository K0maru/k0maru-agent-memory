use std::fs;
use std::sync::Arc;

use k0maru::adapters::ObsidianAdapter;
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::{HybridSearchEngine, SearchMode, SqliteStorage};
use k0maru::vector::MockEmbeddingEngine;
use k0maru::EmbeddingEngine;
use k0maru_core::text::{
    build_fts_query, fts_hash_for, segment_for_index, FtsQueryTier, FTS_SEGMENTER_VERSION,
};
use rusqlite::{params, Connection};
use tempfile::tempdir;

#[test]
fn test_fts_hash_for_includes_segmenter_version() {
    assert_eq!(FTS_SEGMENTER_VERSION, 1);
    assert_eq!(fts_hash_for("deadbeef"), "deadbeef:1");
}

#[test]
fn test_segment_for_index_cjk_bigrams_and_single_chars() {
    assert_eq!(segment_for_index(""), "");
    assert_eq!(segment_for_index("   \n\t  "), "");
    assert_eq!(segment_for_index("猫"), "猫");
    assert_eq!(segment_for_index("向量"), "向量");
    // Overlapping bigrams only, no tail unigram
    assert_eq!(
        segment_for_index("向量检索引擎"),
        "向量 量检 检索 索引 引擎"
    );
    // Kana and Hangul ranges
    assert_eq!(segment_for_index("ひらがな"), "ひら らが がな");
    assert_eq!(segment_for_index("ｶﾀｶﾅ"), "ｶﾀ ﾀｶ ｶﾅ");
    assert_eq!(segment_for_index("한글테스트"), "한글 글테 테스 스트");
    // Supplementary CJK Extension B
    assert_eq!(segment_for_index("𠀀𠀁𠀂"), "𠀀𠀁 𠀁𠀂");
}

#[test]
fn test_segment_for_index_cross_script_and_fullwidth_folding() {
    assert_eq!(segment_for_index("使用Rust"), "使用 Rust");
    assert_eq!(segment_for_index("Rust实现"), "Rust 实现");
    assert_eq!(segment_for_index("使用 Rust 实现"), "使用 Rust 实现");
    assert_eq!(segment_for_index("C语言Rust"), "C 语言 Rust");
    assert_eq!(segment_for_index("ＡＤＲ－０４２"), "ADR-042");
    assert_eq!(segment_for_index("向量\u{3000}检索"), "向量 检索");
    assert_eq!(segment_for_index("ADR-042"), "ADR-042");
    assert_eq!(
        segment_for_index("acquire_fenced_lease"),
        "acquire_fenced_lease"
    );
    assert_eq!(segment_for_index("pre-commit"), "pre-commit");
}

#[test]
fn test_build_fts_query_tiers_and_sanitization() {
    for tier in [
        FtsQueryTier::Strict,
        FtsQueryTier::AllTokens,
        FtsQueryTier::AnyTokens,
    ] {
        assert_eq!(build_fts_query("", tier), None);
        assert_eq!(build_fts_query("   \u{3000}\t ", tier), None);
        assert_eq!(build_fts_query("--- *** () \"\"\"", tier), None);
    }

    // Never emit unquoted FTS5 operators
    assert_eq!(
        build_fts_query("AND", FtsQueryTier::Strict).as_deref(),
        Some("\"AND\"")
    );
    assert_eq!(
        build_fts_query("foo AND bar OR NOT NEAR", FtsQueryTier::Strict).as_deref(),
        Some("\"foo\" \"AND\" \"bar\" \"OR\" \"NOT\" \"NEAR\"")
    );
    assert_eq!(
        build_fts_query("foo\"bar", FtsQueryTier::Strict).as_deref(),
        Some("\"foo\"\"bar\"")
    );

    // Strict tier
    assert_eq!(
        build_fts_query("猫", FtsQueryTier::Strict).as_deref(),
        Some("\"猫\"*")
    );
    assert_eq!(
        build_fts_query("向量检索", FtsQueryTier::Strict).as_deref(),
        Some("\"向量 量检 检索\"")
    );
    assert_eq!(
        build_fts_query("向量检索 引擎", FtsQueryTier::Strict).as_deref(),
        Some("\"向量 量检 检索\" \"引擎\"")
    );
    assert_eq!(
        build_fts_query("ADR-042", FtsQueryTier::Strict).as_deref(),
        Some("\"ADR-042\"")
    );
    assert_eq!(
        build_fts_query("acquire_fenced_lease", FtsQueryTier::Strict).as_deref(),
        Some("\"acquire_fenced_lease\"")
    );
    assert_eq!(
        build_fts_query("Rust实现", FtsQueryTier::Strict).as_deref(),
        Some("\"Rust 实现\"")
    );

    // AllTokens tier (deduplicated, handles reordering)
    assert_eq!(
        build_fts_query("向量 向量检索", FtsQueryTier::AllTokens).as_deref(),
        Some("\"向量\" AND \"量检\" AND \"检索\"")
    );
    assert_eq!(
        build_fts_query("猫 狗", FtsQueryTier::AllTokens).as_deref(),
        Some("\"猫\"* AND \"狗\"*")
    );
    assert_eq!(
        build_fts_query("Rust实现", FtsQueryTier::AllTokens).as_deref(),
        Some("\"Rust\" AND \"实现\"")
    );

    // AnyTokens tier (OR, capped at 8, preferring ASCII/identifiers)
    let long_query = "分布式结算系统在高并发环境下使用 distributed_lock 和 ADR-042 进行状态同步";
    let any = build_fts_query(long_query, FtsQueryTier::AnyTokens).expect("any query");
    let or_parts: Vec<&str> = any.split(" OR ").collect();
    assert_eq!(or_parts.len(), 8);
    assert!(or_parts.contains(&"\"distributed_lock\""));
    assert!(or_parts.contains(&"\"ADR-042\""));
}

#[test]
fn test_sqlite_fts5_contentless_symmetry_and_identifier_precision() {
    let conn = Connection::open_in_memory().expect("open in-memory");
    conn.execute_batch(
        r#"
        CREATE TABLE docs (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            body TEXT NOT NULL
        );
        CREATE VIRTUAL TABLE fts_docs USING fts5(
            body,
            content='',
            contentless_delete=1,
            tokenize='unicode61 remove_diacritics 2'
        );
        "#,
    )
    .expect("create tables");

    let samples = [
        ("gateway.md", "分布式结算与支付网关采用向量检索引擎。"),
        ("rust.md", "系统核心模块使用Rust实现高并发调度。"),
        (
            "adr042.md",
            "Architecture decision ADR-042 covers pre-commit and telemetry-system.",
        ),
        (
            "adr024.md",
            "Legacy decision ADR-024 referenced in issue 042.",
        ),
        (
            "lease.md",
            "Call acquire_fenced_lease before mutating state.",
        ),
        (
            "scattered.md",
            "First acquire lock, then check fenced buffer, finally renew lease.",
        ),
        ("cat.md", "一只猫咪在索引节点上休息。"),
    ];

    for (idx, (path, body)) in samples.iter().enumerate() {
        let id = (idx + 1) as i64;
        conn.execute(
            "INSERT INTO docs(id, path, body) VALUES (?1, ?2, ?3)",
            params![id, path, body],
        )
        .unwrap();
        let seg = segment_for_index(body);
        conn.execute(
            "INSERT INTO fts_docs(rowid, body) VALUES (?1, ?2)",
            params![id, seg],
        )
        .unwrap();
    }

    let query_paths = |user_q: &str, tier: FtsQueryTier| -> Vec<String> {
        let Some(fts_q) = build_fts_query(user_q, tier) else {
            return Vec::new();
        };
        let mut stmt = conn
            .prepare(
                "SELECT d.path FROM fts_docs f JOIN docs d ON d.id = f.rowid WHERE fts_docs MATCH ?1 ORDER BY f.rank",
            )
            .unwrap();
        stmt.query_map(params![fts_q], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };

    // CJK phrase & sub-term match
    assert_eq!(
        query_paths("向量检索", FtsQueryTier::Strict),
        vec!["gateway.md"]
    );
    assert_eq!(
        query_paths("分布式结算", FtsQueryTier::Strict),
        vec!["gateway.md"]
    );
    // Mixed script phrase adjacency
    assert_eq!(
        query_paths("Rust实现", FtsQueryTier::Strict),
        vec!["rust.md"]
    );
    assert_eq!(
        query_paths("使用Rust", FtsQueryTier::Strict),
        vec!["rust.md"]
    );
    // Reordered cross-script match in AllTokens
    assert_eq!(
        query_paths("实现 Rust", FtsQueryTier::AllTokens),
        vec!["rust.md"]
    );
    // Single CJK char prefix match
    assert_eq!(query_paths("猫", FtsQueryTier::Strict), vec!["cat.md"]);

    // Identifier precision: ADR-042 must NOT match ADR-024 + stray 042 in any tier
    for tier in [
        FtsQueryTier::Strict,
        FtsQueryTier::AllTokens,
        FtsQueryTier::AnyTokens,
    ] {
        assert_eq!(query_paths("ADR-042", tier), vec!["adr042.md"]);
    }

    // Compound underscore identifier precision vs individual sub-token match
    assert_eq!(
        query_paths("acquire_fenced_lease", FtsQueryTier::Strict),
        vec!["lease.md"]
    );
    let acquire_hits = query_paths("acquire", FtsQueryTier::Strict);
    assert!(acquire_hits.contains(&"lease.md".to_string()));
    assert!(acquire_hits.contains(&"scattered.md".to_string()));

    // Hyphenated identifiers
    assert_eq!(
        query_paths("pre-commit", FtsQueryTier::Strict),
        vec!["adr042.md"]
    );
    assert_eq!(
        query_paths("telemetry-system", FtsQueryTier::Strict),
        vec!["adr042.md"]
    );

    // Operator/quote injection inputs must never cause SQLite FTS5 syntax errors
    for hostile in [
        "AND",
        "OR",
        "NOT",
        "NEAR",
        "foo\"bar",
        "\"向量检索\"",
        "a*b^c(d)",
        "(ADR-042)",
    ] {
        for tier in [
            FtsQueryTier::Strict,
            FtsQueryTier::AllTokens,
            FtsQueryTier::AnyTokens,
        ] {
            let _ = query_paths(hostile, tier);
        }
    }
}

#[test]
fn test_hybrid_drops_ghost_vector_hits_and_fixes_snippets() {
    let dir = tempdir().expect("tempdir");
    let vault_root = dir.path();

    let utf8_expansion_body = format!("{} telemetry settlement target pipeline.", "İ".repeat(250));
    let cjk_fallback_body = format!(
        "{} 这里讨论分布式结算网关中的向量检索模块，但没有引擎两字。",
        "prefix ".repeat(40)
    );

    fs::write(
        vault_root.join("payment-gateway.md"),
        format!("# Payment Gateway\n{}", utf8_expansion_body),
    )
    .unwrap();
    fs::write(
        vault_root.join("settlement.md"),
        format!("# Settlement\n{}", cjk_fallback_body),
    )
    .unwrap();

    let adapter = ObsidianAdapter::new(vault_root);
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    let embedder = Arc::new(MockEmbeddingEngine::new(384));
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);

    let (sync_stats, vec_stats) = scanner
        .sync_vault_with_vector(vault_root, Some(embedder.clone()))
        .expect("sync");
    assert_eq!(sync_stats.added, 2);
    assert_eq!(vec_stats.embedded_count, 2);

    // Inject a ghost vector whose document row does not exist in `documents`.
    let ghost_query = "telemetry settlement";
    let ghost_emb = embedder.embed(ghost_query).expect("embed ghost query");
    storage
        .insert_vector("ghost-ignored-note.md", &ghost_emb, 42)
        .expect("insert ghost vector");

    let engine = HybridSearchEngine::new(&storage, Some(embedder.clone()));

    // 1. Vector mode must skip the ghost vector hit completely and render UTF-8-safe snippet
    let vec_results = engine
        .search(ghost_query, SearchMode::Vector, 10)
        .expect("vector search");
    assert!(
        vec_results
            .iter()
            .all(|r| r.path != "ghost-ignored-note.md"),
        "ghost vector hit leaked into SearchMode::Vector: {:?}",
        vec_results
    );
    let gw_res = vec_results
        .iter()
        .find(|r| r.path == "payment-gateway.md")
        .expect("payment-gateway present");
    assert!(
        gw_res.snippet.contains("telemetry settlement target"),
        "expected UTF-8 char-safe snippet centered on match, got: {}",
        gw_res.snippet
    );

    // 2. Hybrid mode must also skip the ghost vector hit
    let hybrid_results = engine
        .search(ghost_query, SearchMode::Hybrid, 10)
        .expect("hybrid search");
    assert!(
        hybrid_results
            .iter()
            .all(|r| r.path != "ghost-ignored-note.md"),
        "ghost vector hit leaked into SearchMode::Hybrid: {:?}",
        hybrid_results
    );

    // 3. CJK longest-bigram snippet fallback when exact query phrase isn't present
    let cjk_results = engine
        .search("向量检索引擎", SearchMode::Vector, 10)
        .expect("cjk vector search");
    let settle_res = cjk_results
        .iter()
        .find(|r| r.path == "settlement.md")
        .expect("settlement present");
    assert!(
        settle_res.snippet.contains("向量检索"),
        "expected snippet fallback to longest matching CJK sub-phrase/bigram, got: {}",
        settle_res.snippet
    );
}
