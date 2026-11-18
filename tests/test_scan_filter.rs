use std::fs;
use std::path::Path;

use k0maru::scanner::ScanFilter;
use k0maru::{
    CacheStorage, GenericWikiAdapter, IncrementalScanner, ObsidianAdapter, SqliteStorage,
    VaultAdapter,
};
use tempfile::tempdir;

#[test]
fn test_k0maruignore_round_trip_ignore_and_unignore() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    fs::create_dir_all(root.join("20_Cards")).unwrap();
    fs::create_dir_all(root.join("drafts")).unwrap();
    fs::create_dir_all(root.join("notes")).unwrap();

    fs::write(
        root.join("20_Cards/adr-001.md"),
        "# Payment Gateway Consensus\nUse idempotency keys.\n",
    )
    .unwrap();
    fs::write(
        root.join("drafts/wip.md"),
        "# Draft Settlement Flow\nUnfinished draft content.\n",
    )
    .unwrap();
    fs::write(
        root.join("notes/keep.md"),
        "# Kept Note\nImportant architectural detail.\n",
    )
    .unwrap();
    fs::write(
        root.join("notes/drop.md"),
        "# Dropped Note\nNoise to ignore.\n",
    )
    .unwrap();
    // Ensure .gitignore is NOT implicitly honored by ScanFilter (only .k0maruignore).
    fs::write(root.join(".gitignore"), "20_Cards/\n").unwrap();

    let adapter = ObsidianAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().unwrap();
    storage.initialize().unwrap();

    // 1. Initial sync indexes all 4 notes (ignoring .gitignore)
    let stats1 = IncrementalScanner::new(&adapter, &mut storage)
        .sync(false)
        .unwrap();
    assert_eq!(stats1.added, 4);
    assert_eq!(stats1.deleted, 0);
    assert_eq!(stats1.excluded, 0);
    assert!(!storage.search_fts("Unfinished", 10).unwrap().is_empty());

    // 2. Add .k0maruignore excluding drafts/ and notes/*.md except notes/keep.md
    fs::write(
        root.join(".k0maruignore"),
        "drafts/\nnotes/*.md\n!notes/keep.md\n",
    )
    .unwrap();

    let stats2 = IncrementalScanner::new(&adapter, &mut storage)
        .sync(false)
        .unwrap();
    assert_eq!(stats2.added, 0);
    assert_eq!(stats2.deleted, 2);
    assert_eq!(stats2.unchanged, 2);
    assert!(storage
        .get_document(Path::new("drafts/wip.md"))
        .unwrap()
        .is_none());
    assert!(storage
        .get_document(Path::new("notes/drop.md"))
        .unwrap()
        .is_none());
    assert!(storage
        .get_document(Path::new("20_Cards/adr-001.md"))
        .unwrap()
        .is_some());
    assert!(storage
        .get_document(Path::new("notes/keep.md"))
        .unwrap()
        .is_some());
    assert!(storage.search_fts("Unfinished", 10).unwrap().is_empty());

    // 3. Remove .k0maruignore: previously ignored files are re-added automatically
    fs::remove_file(root.join(".k0maruignore")).unwrap();
    let stats3 = IncrementalScanner::new(&adapter, &mut storage)
        .sync(false)
        .unwrap();
    assert_eq!(stats3.added, 2);
    assert_eq!(stats3.deleted, 0);
    assert_eq!(stats3.unchanged, 2);
    assert!(storage
        .get_document(Path::new("drafts/wip.md"))
        .unwrap()
        .is_some());
    assert!(!storage.search_fts("Unfinished", 10).unwrap().is_empty());
}

#[test]
fn test_excalidraw_exclusion_by_suffix_and_frontmatter() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    fs::create_dir_all(root.join("diagrams")).unwrap();
    fs::write(
        root.join("diagrams/flow.excalidraw.md"),
        "# Flow Drawing\n```compressed-json\nN4Ig\n```\n",
    )
    .unwrap();
    fs::write(
        root.join("diagrams/UPPER.EXCALIDRAW.MD"),
        "# Uppercase Suffix Drawing\n",
    )
    .unwrap();
    fs::write(
        root.join("diagrams/canvas_note.md"),
        "---\nexcalidraw-plugin: parsed\ntags:\n  - excalidraw\n---\n# Embedded Drawing\n",
    )
    .unwrap();
    fs::write(
        root.join("diagrams/architecture.md"),
        "# System Architecture\nReal documentation note.\n",
    )
    .unwrap();

    let adapter = ObsidianAdapter::new(root);
    let scanned = adapter.scan_files().unwrap();
    let scanned_strs: Vec<String> = scanned
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    // *.excalidraw.md is filtered at scan time; canvas_note.md is excluded post-read via frontmatter
    assert_eq!(
        scanned_strs,
        vec!["diagrams/architecture.md", "diagrams/canvas_note.md"]
    );

    let mut storage = SqliteStorage::in_memory().unwrap();
    storage.initialize().unwrap();

    let stats1 = IncrementalScanner::new(&adapter, &mut storage)
        .sync(false)
        .unwrap();
    assert_eq!(stats1.added, 1);
    assert_eq!(stats1.excluded, 1);
    assert!(storage
        .get_document(Path::new("diagrams/architecture.md"))
        .unwrap()
        .is_some());
    assert!(storage
        .get_document(Path::new("diagrams/canvas_note.md"))
        .unwrap()
        .is_none());

    // Subsequent sync does not add excluded files
    let stats2 = IncrementalScanner::new(&adapter, &mut storage)
        .sync(false)
        .unwrap();
    assert_eq!(stats2.added, 0);
    assert_eq!(stats2.unchanged, 1);
    assert_eq!(stats2.excluded, 1);
}

#[test]
fn test_packs_filtered_by_k0maruignore() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let core_pack = root.join(".k0maru/packs/core-pack");
    let noisy_pack = root.join(".k0maru/packs/noisy-pack");
    fs::create_dir_all(&core_pack).unwrap();
    fs::create_dir_all(&noisy_pack).unwrap();

    fs::write(root.join("Root.md"), "# Root\n").unwrap();
    fs::write(core_pack.join("SKILL.md"), "# Core Skill\n").unwrap();
    fs::write(core_pack.join("sketch.excalidraw.md"), "# Sketch\n").unwrap();
    fs::write(noisy_pack.join("README.md"), "# Noisy Pack\n").unwrap();

    fs::write(root.join(".k0maruignore"), ".k0maru/packs/noisy-pack/\n").unwrap();

    let adapter = ObsidianAdapter::new(root);
    let paths: Vec<String> = adapter
        .scan_files()
        .unwrap()
        .into_iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(paths, vec![".k0maru/packs/core-pack/SKILL.md", "Root.md"]);

    // Excluding .k0maru/packs/ entirely skips all pack markdown
    fs::write(root.join(".k0maruignore"), ".k0maru/packs/\n").unwrap();
    let paths_no_packs: Vec<String> = adapter
        .scan_files()
        .unwrap()
        .into_iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(paths_no_packs, vec!["Root.md"]);
}

#[test]
fn test_canonical_sibling_directive_and_builder() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    fs::create_dir_all(root.join("articles/post1/nested")).unwrap();
    fs::create_dir_all(root.join("articles/post2")).unwrap();

    fs::write(
        root.join(".k0maruignore"),
        "# k0maru:canonical-sibling = canonical.md\n",
    )
    .unwrap();
    fs::write(root.join("articles/post1/canonical.md"), "# Canonical\n").unwrap();
    fs::write(root.join("articles/post1/variant_a.md"), "# Variant A\n").unwrap();
    fs::write(root.join("articles/post1/variant_b.md"), "# Variant B\n").unwrap();
    fs::write(root.join("articles/post1/nested/sub.md"), "# Sub\n").unwrap();
    fs::write(root.join("articles/post2/standalone.md"), "# Standalone\n").unwrap();

    let obsidian = ObsidianAdapter::new(root);
    let obsidian_paths: Vec<String> = obsidian
        .scan_files()
        .unwrap()
        .into_iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        obsidian_paths,
        vec![
            "articles/post1/canonical.md",
            "articles/post1/nested/sub.md",
            "articles/post2/standalone.md",
        ]
    );

    let generic = GenericWikiAdapter::new(root);
    let generic_paths: Vec<String> = generic
        .scan_files()
        .unwrap()
        .into_iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(generic_paths, obsidian_paths);

    // Also verify ScanFilter::with_canonical_siblings builder directly
    let filter = ScanFilter::empty(root).with_canonical_siblings(["standalone.md"]);
    assert!(!filter.is_excluded(Path::new("articles/post2/standalone.md"), false));
    assert!(!filter.is_excluded(Path::new("articles/post1/variant_a.md"), false));
    fs::write(root.join("articles/post2/other.md"), "# Other\n").unwrap();
    assert!(filter.is_excluded(Path::new("articles/post2/other.md"), false));
}

#[test]
fn test_frontmatter_index_false_exclusion_lifecycle() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    fs::create_dir_all(root.join("notes")).unwrap();
    fs::write(
        root.join("notes/public.md"),
        "---\nindex: true\n---\n# Public Note\nDistributed settlement guide.\n",
    )
    .unwrap();
    fs::write(
        root.join("notes/hidden_str.md"),
        "---\nindex: \"false\"\n---\n# Hidden String Note\nShould not be indexed.\n",
    )
    .unwrap();

    let adapter = ObsidianAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().unwrap();
    storage.initialize().unwrap();

    // 1. Initial sync: public.md added, hidden_str.md excluded (not added)
    let stats1 = IncrementalScanner::new(&adapter, &mut storage)
        .sync(false)
        .unwrap();
    assert_eq!(stats1.added, 1);
    assert_eq!(stats1.excluded, 1);
    assert!(storage
        .get_document(Path::new("notes/public.md"))
        .unwrap()
        .is_some());
    assert!(storage
        .get_document(Path::new("notes/hidden_str.md"))
        .unwrap()
        .is_none());

    // 2. Modify public.md to set index: false -> purged from DB and counted as excluded
    fs::write(
        root.join("notes/public.md"),
        "---\nindex: false\n---\n# Public Note\nDistributed settlement guide.\n",
    )
    .unwrap();
    let stats2 = IncrementalScanner::new(&adapter, &mut storage)
        .sync(false)
        .unwrap();
    assert_eq!(stats2.added, 0);
    assert_eq!(stats2.modified, 0);
    assert_eq!(stats2.excluded, 2);
    assert!(storage
        .get_document(Path::new("notes/public.md"))
        .unwrap()
        .is_none());
    assert!(storage.search_fts("settlement", 10).unwrap().is_empty());

    // 3. Re-enable index: true on public.md -> re-added cleanly
    fs::write(
        root.join("notes/public.md"),
        "---\nindex: true\n---\n# Public Note\nDistributed settlement guide.\n",
    )
    .unwrap();
    let stats3 = IncrementalScanner::new(&adapter, &mut storage)
        .sync(false)
        .unwrap();
    assert_eq!(stats3.added, 1);
    assert_eq!(stats3.excluded, 1);
    assert!(storage
        .get_document(Path::new("notes/public.md"))
        .unwrap()
        .is_some());
    assert!(!storage.search_fts("settlement", 10).unwrap().is_empty());
}

#[test]
fn test_non_utf8_file_resilience_skips_bad_file() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    fs::write(root.join("valid_a.md"), "# Valid A\nHealthy markdown.\n").unwrap();
    fs::write(
        root.join("corrupt.md"),
        b"# Broken\n\xff\xfe\xfd invalid utf8",
    )
    .unwrap();
    fs::write(root.join("valid_b.md"), "# Valid B\nAlso healthy.\n").unwrap();

    let adapter = GenericWikiAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().unwrap();
    storage.initialize().unwrap();

    let stats = IncrementalScanner::new(&adapter, &mut storage)
        .sync(false)
        .unwrap();
    assert_eq!(stats.added, 2);
    assert!(storage
        .get_document(Path::new("valid_a.md"))
        .unwrap()
        .is_some());
    assert!(storage
        .get_document(Path::new("valid_b.md"))
        .unwrap()
        .is_some());
    assert!(storage
        .get_document(Path::new("corrupt.md"))
        .unwrap()
        .is_none());

    // Second sync also succeeds cleanly without aborting
    let stats2 = IncrementalScanner::new(&adapter, &mut storage)
        .sync(false)
        .unwrap();
    assert_eq!(stats2.added, 0);
    assert_eq!(stats2.unchanged, 2);
}

#[test]
fn test_body_line_offset_exact_math() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    // Case A: 6-line frontmatter -> body starts on 1-based line 7 -> body_line_offset == 6
    let six_line_fm = "\
---
title: Settlement Spec
tags:
  - architecture
status: active
---
# Settlement Spec
Line 8 content
";
    fs::write(root.join("six_lines.md"), six_line_fm).unwrap();

    // Case B: No frontmatter -> body_line_offset == 0
    let no_fm = "# Plain Note\nLine 2\n";
    fs::write(root.join("no_fm.md"), no_fm).unwrap();

    // Case C: CRLF frontmatter -> 3 lines -> body_line_offset == 3
    let crlf_fm = "---\r\ntitle: CRLF\r\n---\r\n# CRLF Body\r\n";
    fs::write(root.join("crlf.md"), crlf_fm).unwrap();

    // Case D: Empty frontmatter -> 2 lines -> body_line_offset == 2
    let empty_fm = "---\n---\n# After Empty FM\n";
    fs::write(root.join("empty_fm.md"), empty_fm).unwrap();

    // Case E: BOM + 3-line frontmatter -> body_line_offset == 3
    let bom_fm = "\u{FEFF}---\ntitle: BOM\n---\n# After BOM FM\n";
    fs::write(root.join("bom_fm.md"), bom_fm).unwrap();

    let obsidian = ObsidianAdapter::new(root);
    let generic = GenericWikiAdapter::new(root);

    for (file, expected_offset, expected_first_body_line) in [
        ("six_lines.md", 6usize, "# Settlement Spec"),
        ("no_fm.md", 0usize, "# Plain Note"),
        ("crlf.md", 3usize, "# CRLF Body"),
        ("empty_fm.md", 2usize, "# After Empty FM"),
        ("bom_fm.md", 3usize, "# After BOM FM"),
    ] {
        let doc_obs = obsidian.read_document(Path::new(file)).unwrap();
        let doc_gen = generic.read_document(Path::new(file)).unwrap();
        assert_eq!(
            doc_obs.body_line_offset, expected_offset,
            "obsidian {}",
            file
        );
        assert_eq!(
            doc_gen.body_line_offset, expected_offset,
            "generic {}",
            file
        );

        let raw = fs::read_to_string(root.join(file)).unwrap();
        let orig_lines: Vec<&str> = raw.lines().collect();
        // Body 1-based line 1 + body_line_offset == original 1-based line (0-indexed at body_line_offset)
        assert_eq!(
            orig_lines[doc_obs.body_line_offset].trim_start_matches('\u{FEFF}'),
            expected_first_body_line,
            "line math mismatch for {}",
            file
        );
        assert_eq!(
            doc_obs.body.lines().next().unwrap(),
            expected_first_body_line,
            "body first line mismatch for {}",
            file
        );
    }
}
