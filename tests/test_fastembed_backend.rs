#[cfg(feature = "fastembed")]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    use k0maru::vector::{
        default_embedding_engine, resolve_cache_dir, EmbeddingEngine, FastEmbedBackend, VectorError,
    };

    #[test]
    fn test_resolve_cache_dir_precedence() {
        let custom = PathBuf::from("/tmp/custom_models");
        assert_eq!(resolve_cache_dir(Some(custom.clone())), custom);

        let default_dir = resolve_cache_dir(None);
        assert!(default_dir.ends_with("models"));
        assert!(default_dir.to_string_lossy().contains(".k0maru"));
    }

    #[test]
    fn test_fastembed_error_handling_invalid_cache_path() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let not_a_dir = temp_dir.path().join("regular_file.txt");
        fs::write(&not_a_dir, "regular file content").expect("write file");

        // Attempting to create a directory inside a regular file must fail
        let invalid_cache_dir = not_a_dir.join("sub_models");
        let res = FastEmbedBackend::new(Some(invalid_cache_dir));
        assert!(res.is_err());
        match res.unwrap_err() {
            VectorError::ModelInitFailed(msg) => {
                assert!(
                    msg.contains("Failed to create model cache directory"),
                    "Unexpected message: {}",
                    msg
                );
            }
            other => panic!("Expected ModelInitFailed, got {:?}", other),
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_fastembed_error_handling_readonly_cache_path() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::tempdir().expect("tempdir");
        let readonly_parent = temp_dir.path().join("readonly_vault");
        fs::create_dir_all(&readonly_parent).expect("create readonly parent");
        fs::set_permissions(&readonly_parent, fs::Permissions::from_mode(0o444))
            .expect("set readonly");

        let bad_cache = readonly_parent.join("forbidden_models");
        let res = FastEmbedBackend::new(Some(bad_cache));
        assert!(res.is_err());
        match res.unwrap_err() {
            VectorError::ModelInitFailed(msg) => {
                assert!(
                    msg.contains("Failed to create model cache directory")
                        || msg.contains("Permission denied"),
                    "Unexpected message: {}",
                    msg
                );
            }
            other => panic!("Expected ModelInitFailed, got {:?}", other),
        }

        // Restore permissions so tempdir cleanup succeeds
        let _ = fs::set_permissions(&readonly_parent, fs::Permissions::from_mode(0o755));
    }

    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_default_embedding_engine_fallback_on_invalid_cache() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let file_path = temp_dir.path().join("blocker.txt");
        fs::write(&file_path, "blocker").expect("write blocker");
        let invalid_cache = file_path.join("models");

        // default_embedding_engine must catch ModelInitFailed and fallback to MockEmbeddingEngine
        let engine = default_embedding_engine(Some(invalid_cache))
            .expect("Fallback to MockEmbeddingEngine must succeed");
        assert_eq!(engine.dimension(), 384);
        assert_eq!(engine.model_name(), "mock-embedding-384");

        let vec = engine.embed("test fallback").expect("embed fallback");
        assert_eq!(vec.len(), 384);
    }

    #[test]
    fn test_default_embedding_engine_force_mock_env() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("K0MARU_FORCE_MOCK_EMBED", "1");
        let engine = default_embedding_engine(None).expect("engine with force mock");
        assert_eq!(engine.dimension(), 384);
        assert_eq!(engine.model_name(), "mock-embedding-384");
        std::env::remove_var("K0MARU_FORCE_MOCK_EMBED");
    }

    #[test]
    fn test_fastembed_real_inference_if_available() {
        let _guard = ENV_MUTEX.lock().unwrap();
        // Clear any force mock flag
        std::env::remove_var("K0MARU_FORCE_MOCK_EMBED");

        let cache_dir = resolve_cache_dir(None);
        let backend = match FastEmbedBackend::new(Some(cache_dir)) {
            Ok(b) => b,
            Err(VectorError::ModelInitFailed(e)) => {
                eprintln!(
                    "ℹ️ Skipping real ONNX inference test (model download unavailable in this environment): {}",
                    e
                );
                return;
            }
            Err(e) => panic!("Unexpected error during FastEmbedBackend::new: {:?}", e),
        };

        // Trait implementation and dimension checks
        assert_eq!(backend.dimension(), 384);
        assert_eq!(backend.model_name(), "all-MiniLM-L6-v2");

        // Object-safety: wrap in Arc<dyn EmbeddingEngine>
        let dyn_engine: Arc<dyn EmbeddingEngine> = Arc::new(backend);
        assert_eq!(dyn_engine.dimension(), 384);
        assert_eq!(dyn_engine.model_name(), "all-MiniLM-L6-v2");

        // Single embed
        let vec = dyn_engine
            .embed("K0maru local ONNX vector backend")
            .expect("single embed");
        assert_eq!(vec.len(), 384);
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(norm > 0.5, "Expected normalized vector, norm was: {}", norm);

        // Batch embed
        let texts = vec![
            "Rust memory safety",
            "Obsidian markdown second brain",
            "Reciprocal rank fusion search",
        ];
        let batch = dyn_engine.embed_batch(&texts).expect("batch embed");
        assert_eq!(batch.len(), 3);
        for v in &batch {
            assert_eq!(v.len(), 384);
        }

        // Empty batch returns empty Vec without calling ONNX session
        let empty = dyn_engine.embed_batch(&[]).expect("empty batch");
        assert!(empty.is_empty());

        // Thread safety: clone engine to another thread
        let engine_clone = dyn_engine.clone();
        let handle = std::thread::spawn(move || {
            let v = engine_clone
                .embed("Concurrent thread query")
                .expect("concurrent embed");
            assert_eq!(v.len(), 384);
        });
        handle.join().expect("thread join");
    }
}
