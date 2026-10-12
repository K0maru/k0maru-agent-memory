use k0maru_core::core::traits::CacheStorage;
use k0maru_core::storage::SqliteStorage;
use tempfile::NamedTempFile;

#[test]
fn test_initial_vector_dimension_is_384() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    storage.initialize().expect("initialize");

    let dim = storage.get_vector_dimension().expect("get dim");
    assert_eq!(dim, Some(384));

    // Calling ensure_vector_dimension with same dimension returns false (no migration)
    let migrated = storage.ensure_vector_dimension(384).expect("ensure dim");
    assert!(!migrated);
    assert_eq!(storage.get_vector_dimension().expect("dim"), Some(384));
}

#[test]
fn test_dimension_migration_from_384_to_512() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    storage.initialize().expect("initialize");

    // Insert 384d vector
    let emb_384 = vec![0.1f32; 384];
    storage
        .insert_vector("doc_1", &emb_384, 111)
        .expect("insert 384d");
    assert_eq!(storage.vector_count().expect("count"), 1);
    assert_eq!(
        storage.get_vector_content_hash("doc_1").expect("hash"),
        Some(111)
    );

    // Migrate to 512d (BGE-ZH)
    let migrated = storage.ensure_vector_dimension(512).expect("migrate 512");
    assert!(migrated, "Should indicate migration was performed");
    assert_eq!(storage.get_vector_dimension().expect("dim"), Some(512));

    // Old vectors should have been wiped
    assert_eq!(storage.vector_count().expect("count after migration"), 0);
    assert_eq!(
        storage.get_vector_content_hash("doc_1").expect("hash"),
        None
    );

    // Inserting 384d vector should now fail with descriptive error
    let err_384 = storage.insert_vector("doc_old", &emb_384, 222).unwrap_err();
    assert!(
        err_384.to_string().contains("expected 512, got 384"),
        "Error message should mention expected 512, got 384: {}",
        err_384
    );

    // Inserting 512d vector should succeed
    let mut emb_512_a = vec![0.0f32; 512];
    emb_512_a[0] = 1.0;
    let mut emb_512_b = vec![0.0f32; 512];
    emb_512_b[1] = 1.0;

    storage
        .insert_vector("doc_zh_a", &emb_512_a, 333)
        .expect("insert 512d");
    storage
        .insert_vector("doc_zh_b", &emb_512_b, 444)
        .expect("insert 512d");
    assert_eq!(storage.vector_count().expect("count after 512d"), 2);

    // Search with 512d vector
    let results = storage.search_vectors(&emb_512_a, 5).expect("search 512d");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, "doc_zh_a");

    // Search with mismatched 384d vector should fail
    let err_search = storage.search_vectors(&emb_384, 5).unwrap_err();
    assert!(
        err_search.to_string().contains("expected 512, got 384"),
        "Search error should mention expected 512, got 384: {}",
        err_search
    );
}

#[test]
fn test_disk_persistence_with_dimension_migration() {
    let tmp_file = NamedTempFile::new().expect("tempfile");
    let db_path = tmp_file.path().to_path_buf();

    // Session 1: init at 384d
    {
        let mut storage = SqliteStorage::open(&db_path).expect("open");
        storage.initialize().expect("init");
        assert_eq!(storage.get_vector_dimension().expect("dim"), Some(384));
        let emb_384 = vec![0.5f32; 384];
        storage
            .insert_vector("doc_base", &emb_384, 123)
            .expect("insert");
    }

    // Session 2: reopen and migrate to 768d (BGE-Base)
    {
        let mut storage = SqliteStorage::open(&db_path).expect("reopen");
        assert_eq!(storage.get_vector_dimension().expect("dim"), Some(384));
        let migrated = storage.ensure_vector_dimension(768).expect("migrate 768");
        assert!(migrated);
        assert_eq!(storage.get_vector_dimension().expect("dim"), Some(768));

        let emb_768 = vec![0.1f32; 768];
        storage
            .insert_vector("doc_base_768", &emb_768, 456)
            .expect("insert 768");
    }

    // Session 3: reopen and verify 768d persisted
    {
        let storage = SqliteStorage::open(&db_path).expect("reopen 3");
        assert_eq!(storage.get_vector_dimension().expect("dim"), Some(768));
        let emb_768 = vec![0.1f32; 768];
        let results = storage.search_vectors(&emb_768, 5).expect("search 768");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "doc_base_768");
    }
}
