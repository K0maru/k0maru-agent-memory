use k0maru_core::vector::{resolve_embedding_engine, EmbeddingModelType, VectorError};

#[test]
fn test_embedding_model_type_dimensions_and_canonical_names() {
    assert_eq!(EmbeddingModelType::AllMiniLML6V2.dimension(), 384);
    assert_eq!(
        EmbeddingModelType::AllMiniLML6V2.canonical_name(),
        "all-minilm-l6-v2"
    );

    assert_eq!(EmbeddingModelType::BgeSmallEnV15.dimension(), 384);
    assert_eq!(
        EmbeddingModelType::BgeSmallEnV15.canonical_name(),
        "bge-small-en-v1.5"
    );

    assert_eq!(EmbeddingModelType::BgeSmallZhV15.dimension(), 512);
    assert_eq!(
        EmbeddingModelType::BgeSmallZhV15.canonical_name(),
        "bge-small-zh-v1.5"
    );

    assert_eq!(EmbeddingModelType::BgeBaseEnV15.dimension(), 768);
    assert_eq!(
        EmbeddingModelType::BgeBaseEnV15.canonical_name(),
        "bge-base-en-v1.5"
    );

    assert_eq!(EmbeddingModelType::ParaphraseMlMiniLmL12V2.dimension(), 384);
    assert_eq!(
        EmbeddingModelType::ParaphraseMlMiniLmL12V2.canonical_name(),
        "paraphrase-multilingual-minilm-l12-v2"
    );

    assert_eq!(EmbeddingModelType::Mock.dimension(), 384);
    assert_eq!(EmbeddingModelType::Mock.canonical_name(), "mock");
}

#[test]
fn test_embedding_model_type_from_str_and_aliases() {
    // Canonical names
    assert_eq!(
        EmbeddingModelType::from_str("all-minilm-l6-v2").unwrap(),
        EmbeddingModelType::AllMiniLML6V2
    );
    assert_eq!(
        EmbeddingModelType::from_str("bge-small-en-v1.5").unwrap(),
        EmbeddingModelType::BgeSmallEnV15
    );
    assert_eq!(
        EmbeddingModelType::from_str("bge-small-zh-v1.5").unwrap(),
        EmbeddingModelType::BgeSmallZhV15
    );
    assert_eq!(
        EmbeddingModelType::from_str("bge-base-en-v1.5").unwrap(),
        EmbeddingModelType::BgeBaseEnV15
    );
    assert_eq!(
        EmbeddingModelType::from_str("paraphrase-multilingual-minilm-l12-v2").unwrap(),
        EmbeddingModelType::ParaphraseMlMiniLmL12V2
    );
    assert_eq!(
        EmbeddingModelType::from_str("mock").unwrap(),
        EmbeddingModelType::Mock
    );

    // Aliases
    assert_eq!(
        EmbeddingModelType::from_str("minilm").unwrap(),
        EmbeddingModelType::AllMiniLML6V2
    );
    assert_eq!(
        EmbeddingModelType::from_str("minilm-l6").unwrap(),
        EmbeddingModelType::AllMiniLML6V2
    );
    assert_eq!(
        EmbeddingModelType::from_str("bge-zh").unwrap(),
        EmbeddingModelType::BgeSmallZhV15
    );
    assert_eq!(
        EmbeddingModelType::from_str("bge-small-zh").unwrap(),
        EmbeddingModelType::BgeSmallZhV15
    );
    assert_eq!(
        EmbeddingModelType::from_str("bge-en").unwrap(),
        EmbeddingModelType::BgeSmallEnV15
    );
    assert_eq!(
        EmbeddingModelType::from_str("bge-small-en").unwrap(),
        EmbeddingModelType::BgeSmallEnV15
    );
    assert_eq!(
        EmbeddingModelType::from_str("bge-base-en").unwrap(),
        EmbeddingModelType::BgeBaseEnV15
    );
    assert_eq!(
        EmbeddingModelType::from_str("paraphrase-multilingual").unwrap(),
        EmbeddingModelType::ParaphraseMlMiniLmL12V2
    );
    assert_eq!(
        EmbeddingModelType::from_str("paraphrase-minilm").unwrap(),
        EmbeddingModelType::ParaphraseMlMiniLmL12V2
    );

    // Case insensitivity
    assert_eq!(
        EmbeddingModelType::from_str("BGE-ZH").unwrap(),
        EmbeddingModelType::BgeSmallZhV15
    );
    assert_eq!(
        EmbeddingModelType::from_str("All-MiniLM-L6-V2").unwrap(),
        EmbeddingModelType::AllMiniLML6V2
    );
    assert_eq!(
        EmbeddingModelType::from_str("MOCK").unwrap(),
        EmbeddingModelType::Mock
    );

    // Invalid model
    let err = EmbeddingModelType::from_str("nonexistent-model").unwrap_err();
    match err {
        VectorError::ModelNotFound(msg) => {
            assert!(msg.contains("nonexistent-model"));
        }
        other => panic!("Expected ModelNotFound, got {:?}", other),
    }
}

static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_resolve_embedding_engine_with_mock() {
    let engine = resolve_embedding_engine(Some("mock"), None).expect("mock engine");
    assert_eq!(engine.dimension(), 384);
    assert_eq!(engine.model_name(), "mock-embedding-384");
}

#[test]
fn test_resolve_embedding_engine_force_mock_respects_dimension() {
    let _guard = ENV_MUTEX.lock().unwrap();
    std::env::set_var("K0MARU_FORCE_MOCK_EMBED", "1");
    let engine = resolve_embedding_engine(Some("bge-small-zh-v1.5"), None).expect("bge zh mock");
    assert_eq!(engine.dimension(), 512);
    assert_eq!(engine.model_name(), "mock-embedding-512");

    let engine_base =
        resolve_embedding_engine(Some("bge-base-en-v1.5"), None).expect("bge base mock");
    assert_eq!(engine_base.dimension(), 768);
    assert_eq!(engine_base.model_name(), "mock-embedding-768");
    std::env::remove_var("K0MARU_FORCE_MOCK_EMBED");
}

#[test]
fn test_resolve_embedding_engine_env_var() {
    let _guard = ENV_MUTEX.lock().unwrap();
    std::env::set_var("K0MARU_FORCE_MOCK_EMBED", "1");
    std::env::set_var("K0MARU_EMBED_MODEL", "bge-small-zh");
    let engine = resolve_embedding_engine(None, None).expect("engine from env");
    assert_eq!(engine.dimension(), 512);
    assert_eq!(engine.model_name(), "mock-embedding-512");
    std::env::remove_var("K0MARU_EMBED_MODEL");
    std::env::remove_var("K0MARU_FORCE_MOCK_EMBED");
}

#[test]
fn test_resolve_embedding_engine_fallback_on_error() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let file_path = temp_dir.path().join("blocker.txt");
    std::fs::write(&file_path, "blocker").expect("write blocker");
    let invalid_cache = file_path.join("models");

    let engine = resolve_embedding_engine(Some("all-minilm-l6-v2"), Some(invalid_cache))
        .expect("engine fallback");
    assert_eq!(engine.dimension(), 384);
}
