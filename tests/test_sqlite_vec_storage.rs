use k0maru::core::traits::CacheStorage;
use k0maru::storage::SqliteStorage;
use tempfile::NamedTempFile;

fn create_embedding_with_axis(axis: usize, val: f32) -> Vec<f32> {
    let mut vec = vec![0.0f32; 384];
    vec[axis] = val;
    vec
}

fn create_custom_embedding(vals: &[(usize, f32)]) -> Vec<f32> {
    let mut vec = vec![0.0f32; 384];
    for &(idx, val) in vals {
        vec[idx] = val;
    }
    vec
}

#[test]
fn test_vector_schema_initialization() {
    let mut storage = SqliteStorage::in_memory().expect("in_memory failed");
    storage.initialize().expect("initialize failed");

    // Idempotent schema initialization
    storage
        .initialize()
        .expect("re-initialize should be idempotent");

    let count: i64 = storage
        .connection()
        .query_row("SELECT count(*) FROM vector_metadata", [], |row| row.get(0))
        .expect("vector_metadata table should exist");
    assert_eq!(count, 0);

    let vec_count: i64 = storage
        .connection()
        .query_row("SELECT count(*) FROM document_vectors", [], |row| {
            row.get(0)
        })
        .expect("document_vectors table should exist");
    assert_eq!(vec_count, 0);
}

#[test]
fn test_insert_and_get_vector_content_hash() {
    let mut storage = SqliteStorage::in_memory().expect("in_memory failed");
    storage.initialize().expect("initialize failed");

    let doc_id = "20_Cards/Arch.md";
    let hash = 0x1234_5678_90AB_CDEF_u64;
    let emb = create_embedding_with_axis(0, 1.0);

    // Initial check: None
    let initial_hash = storage
        .get_vector_content_hash(doc_id)
        .expect("get_vector_content_hash should succeed");
    assert_eq!(initial_hash, None);

    // Insert
    storage
        .insert_vector(doc_id, &emb, hash)
        .expect("insert_vector should succeed");

    // Fetch hash
    let fetched_hash = storage
        .get_vector_content_hash(doc_id)
        .expect("get_vector_content_hash should succeed");
    assert_eq!(fetched_hash, Some(hash));

    // Update with new hash and new embedding
    let new_hash = 0xFEDC_BA09_8765_4321_u64;
    let new_emb = create_embedding_with_axis(1, 1.0);
    storage
        .insert_vector(doc_id, &new_emb, new_hash)
        .expect("insert_vector update should succeed");

    let updated_hash = storage
        .get_vector_content_hash(doc_id)
        .expect("get_vector_content_hash should succeed");
    assert_eq!(updated_hash, Some(new_hash));
}

#[test]
fn test_search_vectors_cosine_distance_ordering() {
    let mut storage = SqliteStorage::in_memory().expect("in_memory failed");
    storage.initialize().expect("initialize failed");

    // doc_a is strictly aligned with axis 0
    let emb_a = create_embedding_with_axis(0, 1.0);
    // doc_b is orthogonal (axis 1)
    let emb_b = create_embedding_with_axis(1, 1.0);
    // doc_c is at 45 degrees between axis 0 and 1
    let val_45 = (2.0f32).sqrt() / 2.0; // ~0.7071
    let emb_c = create_custom_embedding(&[(0, val_45), (1, val_45)]);

    storage
        .insert_vector("doc_a", &emb_a, 100)
        .expect("insert doc_a failed");
    storage
        .insert_vector("doc_b", &emb_b, 200)
        .expect("insert doc_b failed");
    storage
        .insert_vector("doc_c", &emb_c, 300)
        .expect("insert doc_c failed");

    // Query with vector matching doc_a
    let query_a = create_embedding_with_axis(0, 1.0);
    let results = storage
        .search_vectors(&query_a, 3)
        .expect("search_vectors failed");

    assert_eq!(results.len(), 3);
    // doc_a should be closest (cosine distance ~ 0.0)
    assert_eq!(results[0].0, "doc_a");
    assert!(
        results[0].1 < 1e-4,
        "Expected distance close to 0, got {}",
        results[0].1
    );

    // doc_c should be second closest (cosine distance 1 - cos(45 deg) = 1 - 0.7071 = ~0.2929)
    assert_eq!(results[1].0, "doc_c");
    assert!(
        (results[1].1 - 0.2929).abs() < 1e-2,
        "Expected distance ~0.2929, got {}",
        results[1].1
    );

    // doc_b should be furthest (orthogonal, cosine distance ~ 1.0)
    assert_eq!(results[2].0, "doc_b");
    assert!(
        (results[2].1 - 1.0).abs() < 1e-2,
        "Expected distance ~1.0, got {}",
        results[2].1
    );

    // Test limit constraint
    let top_1 = storage
        .search_vectors(&query_a, 1)
        .expect("search with limit 1 failed");
    assert_eq!(top_1.len(), 1);
    assert_eq!(top_1[0].0, "doc_a");

    let top_0 = storage
        .search_vectors(&query_a, 0)
        .expect("search with limit 0 failed");
    assert!(top_0.is_empty());
}

#[test]
fn test_delete_vector() {
    let mut storage = SqliteStorage::in_memory().expect("in_memory failed");
    storage.initialize().expect("initialize failed");

    let emb = create_embedding_with_axis(0, 1.0);
    storage
        .insert_vector("doc_to_delete", &emb, 42)
        .expect("insert failed");

    assert!(storage
        .get_vector_content_hash("doc_to_delete")
        .unwrap()
        .is_some());

    let results = storage.search_vectors(&emb, 5).expect("search failed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "doc_to_delete");

    // Delete
    storage
        .delete_vector("doc_to_delete")
        .expect("delete_vector failed");

    assert_eq!(
        storage.get_vector_content_hash("doc_to_delete").unwrap(),
        None
    );

    let results_after = storage.search_vectors(&emb, 5).expect("search failed");
    assert!(results_after.is_empty());

    // Deleting again should succeed idempotently
    storage
        .delete_vector("doc_to_delete")
        .expect("idempotent delete should succeed");
}

#[test]
fn test_wipe_and_rebuild_with_vectors() {
    let mut storage = SqliteStorage::in_memory().expect("in_memory failed");
    storage.initialize().expect("initialize failed");

    let emb = create_embedding_with_axis(0, 1.0);
    storage
        .insert_vector("doc_rebuild", &emb, 999)
        .expect("insert failed");

    assert!(storage
        .get_vector_content_hash("doc_rebuild")
        .unwrap()
        .is_some());

    storage
        .wipe_and_rebuild()
        .expect("wipe_and_rebuild should succeed");

    assert_eq!(
        storage.get_vector_content_hash("doc_rebuild").unwrap(),
        None
    );
    let search = storage.search_vectors(&emb, 5).expect("search failed");
    assert!(search.is_empty());

    // Verify we can insert again after wipe
    storage
        .insert_vector("doc_rebuild", &emb, 1000)
        .expect("insert after wipe failed");
    assert_eq!(
        storage.get_vector_content_hash("doc_rebuild").unwrap(),
        Some(1000)
    );
}

#[test]
fn test_disk_persistence_with_vectors() {
    let tmp_file = NamedTempFile::new().expect("Failed to create temp file");
    let db_path = tmp_file.path().to_path_buf();

    let emb = create_embedding_with_axis(0, 1.0);
    let hash = 0xABCD_EF01_2345_6789_u64;

    // Session 1: open and write
    {
        let mut storage = SqliteStorage::open(&db_path).expect("open failed");
        storage.initialize().expect("init failed");
        storage
            .insert_vector("persisted_doc", &emb, hash)
            .expect("insert_vector failed");
    }

    // Session 2: reopen and read
    {
        let storage = SqliteStorage::open(&db_path).expect("reopen failed");
        let read_hash = storage
            .get_vector_content_hash("persisted_doc")
            .expect("get_vector_content_hash failed");
        assert_eq!(read_hash, Some(hash));

        let results = storage.search_vectors(&emb, 1).expect("search failed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "persisted_doc");
    }
}
