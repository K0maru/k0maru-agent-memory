mod common;

use k0maru::core::models::{Document, HierarchyLevel, WikiLink};
use k0maru::core::traits::CacheStorage;
use k0maru::storage::SqliteStorage;
use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
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
            Some("Alias".to_string()),
            "[[TargetNote|Alias]]",
        )],
        vec!["architecture", "core"],
    );

    {
        let mut storage = SqliteStorage::open(&db_path).expect("Failed to open SqliteStorage");
        storage.initialize().expect("Failed to initialize schema");
        storage
            .upsert_document(&doc)
            .expect("Upsert should succeed");

        let retrieved = storage
            .get_document(Path::new("20_Cards/Arch.md"))
            .expect("Get document should succeed")
            .expect("Document should exist");
        assert_eq!(retrieved.title, "Arch Title");
    }

    {
        let storage = SqliteStorage::open(&db_path).expect("Failed to re-open SqliteStorage");
        let retrieved = storage
            .get_document(Path::new("20_Cards/Arch.md"))
            .expect("Get document should succeed")
            .expect("Document should persist");
        assert_eq!(retrieved.title, "Arch Title");
    }
}

#[test]
fn test_sqlite_storage_document_crud_and_fts5() {
    let mut storage = SqliteStorage::in_memory().expect("Failed to create in-memory storage");
    storage.initialize().expect("Failed to initialize schema");

    let doc1 = create_sample_document(
        "10_Projects/Alpha.md",
        "Alpha Project",
        HierarchyLevel::L1Resource,
        "Distributed database replication and consensus protocol.",
        vec![],
        vec!["database", "distributed"],
    );
    let doc2 = create_sample_document(
        "20_Cards/Raft.md",
        "Raft Consensus",
        HierarchyLevel::L3Evergreen,
        "Leader election, log replication, and cluster membership changes.",
        vec![],
        vec!["consensus", "raft"],
    );

    storage.upsert_document(&doc1).expect("Upsert doc1");
    storage.upsert_document(&doc2).expect("Upsert doc2");

    let fts_results = storage
        .search_fts("replication", 10)
        .expect("FTS search failed");
    assert_eq!(fts_results.len(), 2);

    let fts_raft = storage.search_fts("leader", 10).expect("FTS search failed");
    assert_eq!(fts_raft.len(), 1);
    assert_eq!(fts_raft[0].title, "Raft Consensus");

    // Deletion
    storage
        .delete_document(Path::new("10_Projects/Alpha.md"))
        .expect("Delete doc1");
    assert!(storage
        .get_document(Path::new("10_Projects/Alpha.md"))
        .unwrap()
        .is_none());
    let fts_after_del = storage
        .search_fts("replication", 10)
        .expect("FTS search after delete");
    assert_eq!(fts_after_del.len(), 1);
}

#[test]
fn test_sqlite_storage_wikilink_forward_and_backlinks() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    storage.initialize().expect("initialize");

    let doc_a = create_sample_document(
        "20_Cards/NoteA.md",
        "Note A",
        HierarchyLevel::L3Evergreen,
        "Note A links to Note B.",
        vec![WikiLink::new("NoteB", None::<String>, "[[NoteB]]")],
        vec![],
    );
    let doc_b = create_sample_document(
        "20_Cards/NoteB.md",
        "Note B",
        HierarchyLevel::L3Evergreen,
        "Note B body.",
        vec![],
        vec![],
    );

    storage.upsert_document(&doc_a).expect("upsert doc_a");
    storage.upsert_document(&doc_b).expect("upsert doc_b");

    let outgoing = storage
        .get_outgoing_links(Path::new("20_Cards/NoteA.md"))
        .expect("outgoing");
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].target, "NoteB");

    let incoming = storage.get_backlinks("NoteB").expect("incoming");
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0], PathBuf::from("20_Cards/NoteA.md"));
}

#[test]
fn test_sqlite_storage_wipe_and_rebuild() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    storage.initialize().expect("initialize");

    let doc = create_sample_document(
        "00_Daily/Temp.md",
        "Temp Note",
        HierarchyLevel::L0Ephemeral,
        "Temporary scratchpad",
        vec![],
        vec!["temp"],
    );
    storage.upsert_document(&doc).expect("upsert doc");
    assert_eq!(storage.search_fts("Temporary", 10).unwrap().len(), 1);

    storage.wipe_and_rebuild().expect("wipe_and_rebuild");
    assert_eq!(storage.search_fts("Temporary", 10).unwrap().len(), 0);
}

#[test]
fn test_vector_storage_insert_and_cosine_search() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    storage.initialize().expect("initialize");

    let mut vec_a = vec![0.0f32; 384];
    vec_a[0] = 1.0;
    let mut vec_b = vec![0.0f32; 384];
    vec_b[1] = 1.0;

    storage
        .insert_vector("doc_a", &vec_a, 1001)
        .expect("insert vec_a");
    storage
        .insert_vector("doc_b", &vec_b, 1002)
        .expect("insert vec_b");

    assert_eq!(storage.vector_count().unwrap(), 2);
    assert_eq!(
        storage.get_vector_content_hash("doc_a").unwrap(),
        Some(1001)
    );

    let results = storage.search_vectors(&vec_a, 5).expect("search vectors");
    assert!(!results.is_empty());
    assert_eq!(results[0].0, "doc_a");
    assert!(results[0].1 < 0.001);
}

#[test]
fn test_vector_storage_dynamic_dimension_migration() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    storage.initialize().expect("initialize");

    assert_eq!(storage.get_vector_dimension().unwrap(), Some(384));

    let vec_384 = vec![0.5f32; 384];
    storage
        .insert_vector("doc_1", &vec_384, 111)
        .expect("insert 384d");
    assert_eq!(storage.vector_count().unwrap(), 1);

    // Dynamic dimension migration to 512d (BGE-ZH)
    let migrated = storage
        .ensure_vector_dimension(512)
        .expect("migrate to 512");
    assert!(migrated);
    assert_eq!(storage.get_vector_dimension().unwrap(), Some(512));
    assert_eq!(storage.vector_count().unwrap(), 0);

    let mut vec_512 = vec![0.0f32; 512];
    vec_512[0] = 1.0;
    storage
        .insert_vector("doc_2", &vec_512, 222)
        .expect("insert 512d");
    assert_eq!(storage.vector_count().unwrap(), 1);

    let search_res = storage.search_vectors(&vec_512, 5).expect("search 512d");
    assert_eq!(search_res[0].0, "doc_2");
}

#[test]
fn test_domain_models_serialization_and_metadata() {
    let levels = vec![
        HierarchyLevel::L0Ephemeral,
        HierarchyLevel::L1Resource,
        HierarchyLevel::L2Log,
        HierarchyLevel::L3Evergreen,
    ];

    for level in &levels {
        let ser = serde_json::to_string(level).unwrap();
        let de: HierarchyLevel = serde_json::from_str(&ser).unwrap();
        assert_eq!(*level, de);
    }

    let mut set = HashSet::new();
    set.insert(HierarchyLevel::L0Ephemeral);
    set.insert(HierarchyLevel::L3Evergreen);
    assert!(set.contains(&HierarchyLevel::L0Ephemeral));
    assert!(!set.contains(&HierarchyLevel::L1Resource));

    let link = WikiLink::new(
        "TargetNote",
        Some("Alias".to_string()),
        "[[TargetNote|Alias]]",
    );
    let ser_link = serde_json::to_string(&link).unwrap();
    let de_link: WikiLink = serde_json::from_str(&ser_link).unwrap();
    assert_eq!(link, de_link);
}
