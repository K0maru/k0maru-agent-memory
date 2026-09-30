use k0maru::core::models::{Document, HierarchyLevel, WikiLink};
use k0maru::core::traits::CacheStorage;
use k0maru::storage::SqliteStorage;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tempfile::NamedTempFile;

fn create_sample_document(
    path: &str,
    title: &str,
    hierarchy: HierarchyLevel,
    body: &str,
    links: Vec<WikiLink>,
    tags: Vec<&str>,
) -> Document {
    Document {
        path: PathBuf::from(path),
        title: title.to_string(),
        hierarchy,
        frontmatter: json!({
            "title": title,
            "tags": tags,
        }),
        links,
        tags: tags.into_iter().map(|s| s.to_string()).collect(),
        content_hash: format!("hash_{}", path),
        mtime: 1774843200,
        body: body.to_string(),
    }
}

#[test]
fn test_sqlite_storage_init_and_in_memory() {
    let mut storage = SqliteStorage::in_memory().expect("Failed to create in-memory SqliteStorage");
    storage.initialize().expect("Failed to initialize schema");

    // Idempotent initialization
    storage
        .initialize()
        .expect("Re-initialization should be idempotent");

    let outgoing = storage
        .get_outgoing_links(Path::new("dummy.md"))
        .expect("Querying empty storage should succeed");
    assert!(outgoing.is_empty());
}

#[test]
fn test_sqlite_storage_open_file_and_persistence() {
    let tmp_file = NamedTempFile::new().expect("Failed to create temp file");
    let db_path = tmp_file.path().to_path_buf();

    let doc = create_sample_document(
        "20_Cards/Arch.md",
        "Arch Title",
        HierarchyLevel::L3Evergreen,
        "Body content for architecture",
        vec![WikiLink::new(
            "TargetNote",
            Some("Alias"),
            "[[TargetNote|Alias]]",
        )],
        vec!["arch", "core"],
    );

    // Write in first connection session
    {
        let mut storage = SqliteStorage::open(&db_path).expect("Failed to open sqlite db");
        storage.initialize().expect("Failed to initialize");
        storage.upsert_document(&doc).expect("Failed to upsert doc");
    }

    // Reopen in second connection session
    {
        let storage = SqliteStorage::open(&db_path).expect("Failed to reopen sqlite db");
        let outgoing = storage
            .get_outgoing_links(Path::new("20_Cards/Arch.md"))
            .expect("Failed to get outgoing links");
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].target, "TargetNote");
        assert_eq!(outgoing[0].alias.as_deref(), Some("Alias"));

        let results = storage
            .search_fts("architecture", 10)
            .expect("FTS search failed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Arch Title");
    }
}

#[test]
fn test_upsert_document_and_links_and_tags() {
    let mut storage = SqliteStorage::in_memory().expect("in_memory failed");
    storage.initialize().expect("initialize failed");

    let doc_path = PathBuf::from("10_Projects/proj.md");
    let doc1 = create_sample_document(
        "10_Projects/proj.md",
        "Project Alpha",
        HierarchyLevel::L1Resource,
        "Initial project description referencing [[NoteOne]] and [[NoteTwo]].",
        vec![
            WikiLink::new("NoteOne", None::<String>, "[[NoteOne]]"),
            WikiLink::new("NoteTwo", None::<String>, "[[NoteTwo]]"),
        ],
        vec!["project", "alpha"],
    );

    storage
        .upsert_document(&doc1)
        .expect("Initial upsert failed");

    let outgoing = storage
        .get_outgoing_links(&doc_path)
        .expect("get_outgoing_links failed");
    assert_eq!(outgoing.len(), 2);
    assert_eq!(outgoing[0].target, "NoteOne");
    assert_eq!(outgoing[1].target, "NoteTwo");

    let backlinks_one = storage
        .get_backlinks("NoteOne")
        .expect("get_backlinks failed");
    assert_eq!(backlinks_one, vec![doc_path.clone()]);

    // Overwrite document with new links and new tags
    let doc1_updated = create_sample_document(
        "10_Projects/proj.md",
        "Project Alpha v2",
        HierarchyLevel::L1Resource,
        "Updated body referencing only [[NoteThree]].",
        vec![WikiLink::new(
            "NoteThree",
            Some("Three"),
            "[[NoteThree|Three]]",
        )],
        vec!["project", "v2"],
    );

    storage
        .upsert_document(&doc1_updated)
        .expect("Update upsert failed");

    let outgoing_updated = storage
        .get_outgoing_links(&doc_path)
        .expect("get_outgoing_links after update failed");
    assert_eq!(outgoing_updated.len(), 1);
    assert_eq!(outgoing_updated[0].target, "NoteThree");

    // Old backlinks should no longer point here
    let backlinks_old = storage
        .get_backlinks("NoteOne")
        .expect("get_backlinks failed");
    assert!(backlinks_old.is_empty());

    let backlinks_new = storage
        .get_backlinks("NoteThree")
        .expect("get_backlinks failed");
    assert_eq!(backlinks_new, vec![doc_path]);
}

#[test]
fn test_delete_document_cascade() {
    let mut storage = SqliteStorage::in_memory().expect("in_memory failed");
    storage.initialize().expect("initialize failed");

    let doc = create_sample_document(
        "20_Cards/ToDelete.md",
        "To Delete",
        HierarchyLevel::L3Evergreen,
        "Some secret uniquephrasehere to delete",
        vec![WikiLink::new(
            "LinkedCard",
            None::<String>,
            "[[LinkedCard]]",
        )],
        vec!["temporary"],
    );

    storage.upsert_document(&doc).expect("upsert failed");
    assert_eq!(storage.get_outgoing_links(&doc.path).unwrap().len(), 1);
    assert_eq!(storage.get_backlinks("LinkedCard").unwrap().len(), 1);
    assert_eq!(storage.search_fts("uniquephrasehere", 10).unwrap().len(), 1);

    storage
        .delete_document(&doc.path)
        .expect("delete_document failed");

    assert!(storage.get_outgoing_links(&doc.path).unwrap().is_empty());
    assert!(storage.get_backlinks("LinkedCard").unwrap().is_empty());
    assert!(storage
        .search_fts("uniquephrasehere", 10)
        .unwrap()
        .is_empty());
}

#[test]
fn test_fts5_search_and_bm25_ranking() {
    let mut storage = SqliteStorage::in_memory().expect("in_memory failed");
    storage.initialize().expect("initialize failed");

    let doc_low = create_sample_document(
        "20_Cards/LowRelevance.md",
        "General Systems",
        HierarchyLevel::L3Evergreen,
        "This note mentions memory once among many other things like storage disk network.",
        vec![],
        vec!["systems"],
    );

    let doc_high = create_sample_document(
        "20_Cards/HighRelevance.md",
        "Memory Architecture",
        HierarchyLevel::L3Evergreen,
        "Memory memory memory is central to agent memory hub design. Fast memory access.",
        vec![],
        vec!["memory", "core"],
    );

    let doc_unrelated = create_sample_document(
        "00_Daily/Cooking.md",
        "Cooking Recipes",
        HierarchyLevel::L0Ephemeral,
        "Pasta with tomato sauce and basil leaves.",
        vec![],
        vec!["cooking"],
    );

    storage.upsert_document(&doc_low).unwrap();
    storage.upsert_document(&doc_high).unwrap();
    storage.upsert_document(&doc_unrelated).unwrap();

    let search_results = storage.search_fts("memory", 10).expect("FTS search failed");
    assert_eq!(search_results.len(), 2);
    // BM25 ranking: doc_high has title match, tag match, and high frequency in body
    assert_eq!(search_results[0].title, "Memory Architecture");
    assert_eq!(search_results[1].title, "General Systems");

    // Search by tag
    let tag_results = storage
        .search_fts("cooking", 5)
        .expect("FTS tag search failed");
    assert_eq!(tag_results.len(), 1);
    assert_eq!(tag_results[0].title, "Cooking Recipes");

    // Search empty query
    let empty_results = storage.search_fts("", 5).expect("Empty query failed");
    assert!(empty_results.is_empty());

    let spaces_results = storage.search_fts("   ", 5).expect("Spaces query failed");
    assert!(spaces_results.is_empty());
}

#[test]
fn test_backlinks_case_insensitivity_and_extension_tolerance() {
    let mut storage = SqliteStorage::in_memory().expect("in_memory failed");
    storage.initialize().expect("initialize failed");

    let doc_a = create_sample_document(
        "00_Daily/Day1.md",
        "Day 1",
        HierarchyLevel::L0Ephemeral,
        "Mentioning [[Architecture_Decisions]]",
        vec![WikiLink::new(
            "Architecture_Decisions",
            None::<String>,
            "[[Architecture_Decisions]]",
        )],
        vec![],
    );

    let doc_b = create_sample_document(
        "01_AI_Logs/Log1.md",
        "Log 1",
        HierarchyLevel::L2Log,
        "Mentioning [[architecture_decisions.md]] with lower case and md",
        vec![WikiLink::new(
            "architecture_decisions.md",
            None::<String>,
            "[[architecture_decisions.md]]",
        )],
        vec![],
    );

    let doc_c = create_sample_document(
        "10_Projects/Proj.md",
        "Proj",
        HierarchyLevel::L1Resource,
        "Mentioning [[20_Cards/Architecture_Decisions]] with path prefix",
        vec![WikiLink::new(
            "20_Cards/Architecture_Decisions",
            None::<String>,
            "[[20_Cards/Architecture_Decisions]]",
        )],
        vec![],
    );

    let doc_d = create_sample_document(
        "20_Cards/Unrelated.md",
        "Unrelated",
        HierarchyLevel::L3Evergreen,
        "Mentioning [[OtherTarget]]",
        vec![WikiLink::new(
            "OtherTarget",
            None::<String>,
            "[[OtherTarget]]",
        )],
        vec![],
    );

    storage.upsert_document(&doc_a).unwrap();
    storage.upsert_document(&doc_b).unwrap();
    storage.upsert_document(&doc_c).unwrap();
    storage.upsert_document(&doc_d).unwrap();

    let query_cases = [
        "Architecture_Decisions",
        "architecture_decisions",
        "Architecture_Decisions.md",
        "20_Cards/Architecture_Decisions.md",
    ];

    for target in query_cases {
        let mut backlinks = storage
            .get_backlinks(target)
            .unwrap_or_else(|_| panic!("Failed to get backlinks for {}", target));
        backlinks.sort();

        let mut expected = vec![
            PathBuf::from("00_Daily/Day1.md"),
            PathBuf::from("01_AI_Logs/Log1.md"),
            PathBuf::from("10_Projects/Proj.md"),
        ];
        expected.sort();

        assert_eq!(backlinks, expected, "Failed for query target: {}", target);
    }
}

#[test]
fn test_wipe_and_rebuild_performance() {
    let mut storage = SqliteStorage::in_memory().expect("in_memory failed");
    storage.initialize().expect("initialize failed");

    // Populate with 100 documents
    for i in 0..100 {
        let doc = create_sample_document(
            &format!("cards/doc_{}.md", i),
            &format!("Title {}", i),
            HierarchyLevel::L3Evergreen,
            &format!("Body content {} with [[link_{}]]", i, i),
            vec![WikiLink::new(
                format!("link_{}", i),
                None::<String>,
                format!("[[link_{}]]", i),
            )],
            vec!["batch", "perf"],
        );
        storage.upsert_document(&doc).unwrap();
    }

    assert_eq!(storage.search_fts("batch", 200).unwrap().len(), 100);

    // Wipe and rebuild
    let start = Instant::now();
    storage.wipe_and_rebuild().expect("wipe_and_rebuild failed");
    let elapsed = start.elapsed();

    // Invariant: wipe_and_rebuild takes < 10ms
    assert!(
        elapsed.as_millis() < 10,
        "wipe_and_rebuild took too long: {}ms (must be < 10ms)",
        elapsed.as_millis()
    );

    // Verify completely empty
    assert!(storage.search_fts("batch", 10).unwrap().is_empty());
    assert!(storage
        .get_outgoing_links(Path::new("cards/doc_0.md"))
        .unwrap()
        .is_empty());

    // Verify schema is completely operational after wipe
    let fresh_doc = create_sample_document(
        "fresh.md",
        "Fresh Doc",
        HierarchyLevel::L0Ephemeral,
        "Fresh body",
        vec![],
        vec!["fresh"],
    );
    storage
        .upsert_document(&fresh_doc)
        .expect("Upsert after wipe failed");
    assert_eq!(storage.search_fts("fresh", 10).unwrap().len(), 1);
}

#[test]
fn test_fts_syntax_and_special_characters() {
    let mut storage = SqliteStorage::in_memory().expect("in_memory failed");
    storage.initialize().expect("initialize failed");

    let doc = create_sample_document(
        "code.md",
        "Rust Code Snippet",
        HierarchyLevel::L1Resource,
        "fn main() -> Result<(), Box<dyn std::error::Error>> { println!(\"Hello World: 123\"); }",
        vec![],
        vec!["rust", "code-snippet"],
    );
    storage.upsert_document(&doc).unwrap();

    // Test search queries with punctuation and quotes
    let queries = [
        "Result",
        "\"Hello World\"",
        "println!",
        "std::error",
        "code-snippet",
        "unmatched \" quote",
        "nested (parens) and [brackets]",
    ];

    for q in queries {
        let results = storage.search_fts(q, 10);
        assert!(
            results.is_ok(),
            "Search with query '{}' should not fail with syntax error: {:?}",
            q,
            results.err()
        );
    }
}
