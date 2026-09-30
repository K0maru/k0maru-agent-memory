mod common;

use k0maru::core::models::{Document, HierarchyLevel, SyncStats, WikiLink};
use k0maru::core::traits::{CacheStorage, VaultAdapter};
use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[test]
fn test_hierarchy_level_variants_and_serde() {
    let levels = vec![
        HierarchyLevel::L0Ephemeral,
        HierarchyLevel::L1Resource,
        HierarchyLevel::L2Log,
        HierarchyLevel::L3Evergreen,
    ];

    for level in &levels {
        let serialized = serde_json::to_string(level).expect("Failed to serialize HierarchyLevel");
        let deserialized: HierarchyLevel =
            serde_json::from_str(&serialized).expect("Failed to deserialize HierarchyLevel");
        assert_eq!(*level, deserialized);
    }

    assert_eq!(
        serde_json::to_string(&HierarchyLevel::L0Ephemeral).unwrap(),
        "\"L0Ephemeral\""
    );
    assert_eq!(
        serde_json::to_string(&HierarchyLevel::L1Resource).unwrap(),
        "\"L1Resource\""
    );
    assert_eq!(
        serde_json::to_string(&HierarchyLevel::L2Log).unwrap(),
        "\"L2Log\""
    );
    assert_eq!(
        serde_json::to_string(&HierarchyLevel::L3Evergreen).unwrap(),
        "\"L3Evergreen\""
    );

    // Test HashSet compatibility (Hash + Eq)
    let mut set = HashSet::new();
    set.insert(HierarchyLevel::L0Ephemeral);
    set.insert(HierarchyLevel::L3Evergreen);
    assert!(set.contains(&HierarchyLevel::L0Ephemeral));
    assert!(!set.contains(&HierarchyLevel::L1Resource));
}

#[test]
fn test_wikilink_models_and_serde() {
    let bare_link = WikiLink {
        target: "Architecture_Decisions".to_string(),
        alias: None,
        raw_text: "[[Architecture_Decisions]]".to_string(),
    };

    let aliased_link = WikiLink {
        target: "k0maru-memory".to_string(),
        alias: Some("K0maru Memory Hub".to_string()),
        raw_text: "[[k0maru-memory|K0maru Memory Hub]]".to_string(),
    };

    let json_bare = serde_json::to_string(&bare_link).expect("Serialize bare link failed");
    let deser_bare: WikiLink =
        serde_json::from_str(&json_bare).expect("Deserialize bare link failed");
    assert_eq!(bare_link, deser_bare);
    assert_eq!(deser_bare.target, "Architecture_Decisions");
    assert_eq!(deser_bare.alias, None);
    assert_eq!(deser_bare.raw_text, "[[Architecture_Decisions]]");

    let json_alias = serde_json::to_string(&aliased_link).expect("Serialize aliased link failed");
    let deser_alias: WikiLink =
        serde_json::from_str(&json_alias).expect("Deserialize aliased link failed");
    assert_eq!(aliased_link, deser_alias);
    assert_eq!(deser_alias.alias.as_deref(), Some("K0maru Memory Hub"));
}

#[test]
fn test_document_model_serialization_roundtrip() {
    let link = WikiLink {
        target: "Architecture_Decisions".to_string(),
        alias: Some("Core Architecture".to_string()),
        raw_text: "[[Architecture_Decisions|Core Architecture]]".to_string(),
    };

    let doc = Document {
        path: PathBuf::from("20_Cards/Architecture_Decisions.md"),
        title: "Architecture Decisions".to_string(),
        hierarchy: HierarchyLevel::L3Evergreen,
        frontmatter: json!({
            "status": "evergreen",
            "priority": 1,
            "tags": ["core", "architecture"]
        }),
        links: vec![link],
        tags: vec!["core".to_string(), "architecture".to_string()],
        content_hash: "abcd1234ef567890".to_string(),
        mtime: 1774843200,
        body: "# Architecture Decisions\nInvariants here.".to_string(),
    };

    let serialized = serde_json::to_string_pretty(&doc).expect("Failed to serialize Document");
    let deserialized: Document =
        serde_json::from_str(&serialized).expect("Failed to deserialize Document");

    assert_eq!(doc, deserialized);
    assert_eq!(deserialized.title, "Architecture Decisions");
    assert_eq!(deserialized.hierarchy, HierarchyLevel::L3Evergreen);
    assert_eq!(deserialized.links.len(), 1);
    assert_eq!(deserialized.tags.len(), 2);
    assert_eq!(deserialized.content_hash, "abcd1234ef567890");
    assert_eq!(deserialized.mtime, 1774843200);
}

#[test]
fn test_sync_stats_default_and_serde() {
    let stats = SyncStats {
        added: 5,
        modified: 2,
        deleted: 1,
        unchanged: 42,
        duration_ms: 125,
    };

    let serialized = serde_json::to_string(&stats).expect("Failed to serialize SyncStats");
    let deserialized: SyncStats =
        serde_json::from_str(&serialized).expect("Failed to deserialize SyncStats");

    assert_eq!(stats, deserialized);
    assert_eq!(deserialized.added, 5);
    assert_eq!(deserialized.modified, 2);
    assert_eq!(deserialized.deleted, 1);
    assert_eq!(deserialized.unchanged, 42);
    assert_eq!(deserialized.duration_ms, 125);

    let default_stats = SyncStats::default();
    assert_eq!(default_stats.added, 0);
    assert_eq!(default_stats.modified, 0);
    assert_eq!(default_stats.deleted, 0);
    assert_eq!(default_stats.unchanged, 0);
    assert_eq!(default_stats.duration_ms, 0);
}

// Dummy implementations to verify trait contracts compile and execute cleanly
struct DummyAdapter;
impl VaultAdapter for DummyAdapter {
    fn scan_files(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        Ok(vec![PathBuf::from("dummy.md")])
    }
    fn read_document(&self, relative_path: &Path) -> Result<Document, Box<dyn std::error::Error>> {
        Ok(Document {
            path: relative_path.to_path_buf(),
            title: "Dummy".to_string(),
            hierarchy: HierarchyLevel::L0Ephemeral,
            frontmatter: json!({}),
            links: vec![],
            tags: vec![],
            content_hash: "dummyhash".to_string(),
            mtime: 0,
            body: String::new(),
        })
    }
    fn write_card(
        &self,
        title: &str,
        _content: &str,
        _hierarchy: HierarchyLevel,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        Ok(PathBuf::from(format!("20_Cards/{}.md", title)))
    }
}

struct DummyCache;
impl CacheStorage for DummyCache {
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn wipe_and_rebuild(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn upsert_document(&mut self, _doc: &Document) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn delete_document(&mut self, _path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn search_fts(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }
    fn get_outgoing_links(
        &self,
        _path: &Path,
    ) -> Result<Vec<WikiLink>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }
    fn get_backlinks(
        &self,
        _target_title: &str,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }
}

#[test]
fn test_traits_contract_invocation() {
    let adapter = DummyAdapter;
    let files = adapter.scan_files().expect("scan_files failed");
    assert_eq!(files, vec![PathBuf::from("dummy.md")]);

    let doc = adapter
        .read_document(&files[0])
        .expect("read_document failed");
    assert_eq!(doc.title, "Dummy");

    let card_path = adapter
        .write_card("NewCard", "content", HierarchyLevel::L3Evergreen)
        .expect("write_card failed");
    assert_eq!(card_path, PathBuf::from("20_Cards/NewCard.md"));

    let mut cache = DummyCache;
    assert!(cache.initialize().is_ok());
    assert!(cache.wipe_and_rebuild().is_ok());
    assert!(cache.upsert_document(&doc).is_ok());
    assert!(cache.delete_document(&doc.path).is_ok());
    assert!(cache.search_fts("test", 10).unwrap().is_empty());
    assert!(cache.get_outgoing_links(&doc.path).unwrap().is_empty());
    assert!(cache.get_backlinks("Dummy").unwrap().is_empty());
}

#[test]
fn test_mock_obsidian_vault_fixture() {
    let temp_vault = common::fixtures::mock_obsidian_vault();
    let root = temp_vault.path();

    assert!(root.join("00_Daily/2026-09-29.md").exists());
    assert!(root.join("01_AI_Logs/2026-09-29-eval.md").exists());
    assert!(root.join("10_Projects/k0maru-memory.md").exists());
    assert!(root.join("20_Cards/Architecture_Decisions.md").exists());

    let card_content = std::fs::read_to_string(root.join("20_Cards/Architecture_Decisions.md"))
        .expect("Failed to read card");
    assert!(card_content.contains("Architecture Decisions"));
    assert!(card_content.contains("[[k0maru-memory|K0maru Memory Project]]"));
}

#[test]
fn test_mock_karpathy_wiki_fixture() {
    let temp_wiki = common::fixtures::mock_karpathy_wiki();
    let root = temp_wiki.path();

    assert!(root.join("index.md").exists());
    assert!(root.join("rust-memory.md").exists());
    assert!(root.join("sqlite-fts5.md").exists());

    let index_content =
        std::fs::read_to_string(root.join("index.md")).expect("Failed to read index");
    assert!(index_content.contains("[[rust-memory]]"));
    assert!(index_content.contains("[[sqlite-fts5|SQLite FTS5 Search]]"));
}
