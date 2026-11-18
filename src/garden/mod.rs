//! Knowledge graph gardening: orphan detection, broken link auditing,
//! duplicate L3 card detection (cosine + MinHash-LSH), and safe idempotent `--weave`.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::{xxh3_64, xxh3_64_with_seed};

use k0maru_core::adapters::{is_structured_vault, GenericWikiAdapter, ObsidianAdapter};
use k0maru_core::core::models::{Document, HierarchyLevel};
use k0maru_core::graph::{LinkResolver, Resolution};
use k0maru_core::parser::markdown::{
    extract_managed_backlink_blocks, BACKLINK_BLOCK_END, BACKLINK_BLOCK_START,
};
use k0maru_core::scanner::{IncrementalScanner, ScanFilter};
use k0maru_core::storage::SqliteStorage;
use k0maru_core::text::{is_cjk, segment_for_index};

const MAX_WEAVE_FILE_BYTES: u64 = 1024 * 1024; // 1 MiB
const VECTOR_COSINE_DUP_THRESHOLD: f64 = 0.88;
const JACCARD_DUP_THRESHOLD: f64 = 0.60;
const MINHASH_NUM_HASHES: usize = 64;
const LSH_BANDS: usize = 16;
const LSH_ROWS_PER_BAND: usize = 4;

/// Options controlling `k0maru garden` analysis and weaving behavior.
#[derive(Debug, Clone, Default)]
pub struct GardenOptions {
    pub weave: bool,
    pub dry_run: bool,
    pub include_logs: bool,
    pub include_daily: bool,
    pub force: bool,
}

/// An active document with zero incoming and zero outgoing edges in `link_edges`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrphanReport {
    pub path: String,
    pub hierarchy: String,
}

/// A WikiLink row in `links` whose target resolves to `Resolution::Broken`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokenLinkReport {
    pub source: String,
    pub target: String,
    pub raw: String,
}

/// A pair of active L3 cards flagged as potential duplicates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuplicateReport {
    pub a: String,
    pub b: String,
    pub score: f64,
    pub method: String,
}

/// Planned managed-backlink update for a single L3 target card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeavePlanItem {
    pub target: String,
    pub add: Vec<String>,
}

/// Target file skipped during `--weave` due to safety or concurrency guards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaveSkipItem {
    pub path: String,
    pub reason: String,
}

/// Summary of `--weave` planning and writes.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WeaveReport {
    pub planned: Vec<WeavePlanItem>,
    pub applied: usize,
    pub skipped: Vec<WeaveSkipItem>,
}

/// Complete report returned by `k0maru garden`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GardenReport {
    pub orphans: Vec<OrphanReport>,
    pub broken_links: Vec<BrokenLinkReport>,
    pub duplicates: Vec<DuplicateReport>,
    pub weave: WeaveReport,
}

struct IndexedDoc {
    id: i64,
    path_str: String,
    doc: Document,
}

/// Runs incremental sync, graph edge rebuild, garden diagnostics, and optional backlink weaving.
pub fn run_garden(
    vault_path: &Path,
    storage: &mut SqliteStorage,
    opts: &GardenOptions,
) -> Result<GardenReport, Box<dyn std::error::Error>> {
    sync_and_rebuild_edges(vault_path, storage)?;

    let docs = load_indexed_docs(storage)?;
    let mut id_to_idx: HashMap<i64, usize> = HashMap::with_capacity(docs.len());
    for (idx, item) in docs.iter().enumerate() {
        id_to_idx.insert(item.id, idx);
    }

    let resolver = LinkResolver::from_docs(
        docs.iter()
            .map(|d| (d.id, d.path_str.clone(), d.doc.title.clone())),
    );

    let edges = load_link_edges(storage)?;
    let mut has_edge: HashSet<i64> = HashSet::with_capacity(edges.len() * 2);
    for &(src_id, dst_id) in &edges {
        has_edge.insert(src_id);
        has_edge.insert(dst_id);
    }

    // 1. Orphans: active {L1, L2, L3} (L0 only with --include-daily) with 0 in/out edges.
    let mut orphans = Vec::new();
    for item in &docs {
        if !item.doc.is_active() {
            continue;
        }
        if item.doc.hierarchy == HierarchyLevel::L0Ephemeral && !opts.include_daily {
            continue;
        }
        if !has_edge.contains(&item.id) {
            orphans.push(OrphanReport {
                path: item.path_str.clone(),
                hierarchy: item.doc.hierarchy.as_str().to_string(),
            });
        }
    }

    // 2. Broken links: `links` rows resolving to `Resolution::Broken` (Assets & SelfRefs skipped).
    let broken_links = detect_broken_links(storage, &resolver)?;

    // 3. Duplicate active L3 cards (excluding superseded pairs).
    let duplicates = detect_duplicates(storage, &docs, &resolver)?;

    // 4. Optional `--weave`: write sorted, idempotent managed backlink blocks into active L3 cards.
    let weave = if opts.weave {
        let report = run_weave(vault_path, storage, &docs, &id_to_idx, &edges, opts)?;
        if report.applied > 0 && !opts.dry_run {
            sync_and_rebuild_edges(vault_path, storage)?;
        }
        report
    } else {
        WeaveReport::default()
    };

    Ok(GardenReport {
        orphans,
        broken_links,
        duplicates,
        weave,
    })
}

fn sync_and_rebuild_edges(
    vault_path: &Path,
    storage: &mut SqliteStorage,
) -> Result<(), Box<dyn std::error::Error>> {
    if is_structured_vault(vault_path) {
        let adapter = ObsidianAdapter::new(vault_path);
        let mut scanner = IncrementalScanner::new(&adapter, storage);
        scanner.sync(false)?;
    } else {
        let adapter = GenericWikiAdapter::new(vault_path);
        let mut scanner = IncrementalScanner::new(&adapter, storage);
        scanner.sync(false)?;
    }
    storage.rebuild_link_edges_if_dirty()?;
    Ok(())
}

fn load_indexed_docs(
    storage: &SqliteStorage,
) -> Result<Vec<IndexedDoc>, Box<dyn std::error::Error>> {
    let conn = storage.connection();

    let mut tags_by_path: HashMap<String, Vec<String>> = HashMap::new();
    {
        let mut tag_stmt = conn.prepare("SELECT document_path, tag FROM tags")?;
        let tag_rows =
            tag_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in tag_rows {
            let (path, tag) = row?;
            tags_by_path.entry(path).or_default().push(tag);
        }
    }

    let mut stmt = conn.prepare(
        "SELECT id, path, title, hierarchy, frontmatter, content_hash, mtime, body, body_line_offset \
         FROM documents ORDER BY path ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
            r.get::<_, i64>(6)?,
            r.get::<_, String>(7)?,
            r.get::<_, i64>(8)?,
        ))
    })?;

    let mut docs = Vec::new();
    for row in rows {
        let (id, path_str, title, hierarchy_str, fm_str, content_hash, mtime, body, offset) = row?;
        let normalized_path = path_str.replace('\\', "/");
        let hierarchy = parse_hierarchy(&hierarchy_str);
        let frontmatter = serde_json::from_str(&fm_str).unwrap_or(serde_json::json!({}));
        let tags = tags_by_path.remove(&path_str).unwrap_or_default();

        docs.push(IndexedDoc {
            id,
            path_str: normalized_path.clone(),
            doc: Document {
                path: PathBuf::from(normalized_path),
                title,
                hierarchy,
                frontmatter,
                links: Vec::new(),
                tags,
                content_hash,
                mtime: mtime.max(0) as u64,
                body,
                body_line_offset: offset.max(0) as usize,
            },
        });
    }

    Ok(docs)
}

fn parse_hierarchy(s: &str) -> HierarchyLevel {
    match s {
        "L0Ephemeral" => HierarchyLevel::L0Ephemeral,
        "L1Resource" => HierarchyLevel::L1Resource,
        "L2Log" => HierarchyLevel::L2Log,
        "L3Evergreen" => HierarchyLevel::L3Evergreen,
        _ => HierarchyLevel::L0Ephemeral,
    }
}

fn load_link_edges(storage: &SqliteStorage) -> Result<Vec<(i64, i64)>, Box<dyn std::error::Error>> {
    let conn = storage.connection();
    let mut stmt = conn.prepare("SELECT src_doc_id, dst_doc_id FROM link_edges")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
    let mut edges = Vec::new();
    for row in rows {
        edges.push(row?);
    }
    Ok(edges)
}

fn detect_broken_links(
    storage: &SqliteStorage,
    resolver: &LinkResolver,
) -> Result<Vec<BrokenLinkReport>, Box<dyn std::error::Error>> {
    let conn = storage.connection();
    let mut stmt = conn.prepare(
        "SELECT source_path, target, raw_text FROM links ORDER BY source_path ASC, target ASC, raw_text ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;

    let mut broken = Vec::new();
    for row in rows {
        let (source_raw, target, raw) = row?;
        let source = source_raw.replace('\\', "/");
        if matches!(resolver.resolve(&source, &target), Resolution::Broken) {
            broken.push(BrokenLinkReport {
                source,
                target,
                raw,
            });
        }
    }
    Ok(broken)
}

fn collect_supersession_pairs(
    docs: &[IndexedDoc],
    l3_indices: &[usize],
    resolver: &LinkResolver,
) -> HashSet<(i64, i64)> {
    let mut pairs = HashSet::new();

    for &idx in l3_indices {
        let item = &docs[idx];
        let mut refs = item.doc.supersedes();
        if let Some(sb) = item.doc.superseded_by() {
            for part in sb.split(',') {
                let trimmed = part.trim();
                if !trimmed.is_empty() {
                    refs.push(trimmed.to_string());
                }
            }
        }

        for raw_ref in refs {
            let cleaned = strip_wikilink_syntax(&raw_ref);
            if cleaned.is_empty() {
                continue;
            }
            if let Resolution::Doc(other_id) = resolver.resolve(&item.path_str, cleaned) {
                if other_id != item.id {
                    let key = if item.id < other_id {
                        (item.id, other_id)
                    } else {
                        (other_id, item.id)
                    };
                    pairs.insert(key);
                }
            }
        }
    }

    pairs
}

fn strip_wikilink_syntax(raw: &str) -> &str {
    let s = raw.trim();
    let inner = s
        .strip_prefix("[[")
        .and_then(|rest| rest.strip_suffix("]]"))
        .unwrap_or(s);
    inner.split('|').next().unwrap_or(inner).trim()
}

fn load_document_vectors(
    storage: &SqliteStorage,
) -> Result<HashMap<String, Vec<f32>>, Box<dyn std::error::Error>> {
    let conn = storage.connection();
    let has_table: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'document_vectors'",
        [],
        |r| r.get(0),
    )?;
    if has_table == 0 {
        return Ok(HashMap::new());
    }

    let mut stmt = conn.prepare("SELECT id, embedding FROM document_vectors")?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
    })?;

    let mut vectors = HashMap::new();
    for row in rows {
        let (id, blob) = row?;
        if blob.is_empty() || blob.len() % 4 != 0 {
            continue;
        }
        let (chunks, _) = blob.as_chunks::<4>();
        let vec_f32: Vec<f32> = chunks.iter().map(|c| f32::from_le_bytes(*c)).collect();
        vectors.insert(id.replace('\\', "/"), vec_f32);
    }

    Ok(vectors)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;
    for (&x, &y) in a.iter().zip(b.iter()) {
        let xf = x as f64;
        let yf = y as f64;
        dot += xf * yf;
        norm_a += xf * xf;
        norm_b += yf * yf;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom <= 0.0 {
        0.0
    } else {
        dot / denom
    }
}

fn tokenize_for_jaccard(title: &str, body: &str) -> HashSet<String> {
    let combined = format!("{title}\n{body}");
    let segmented = segment_for_index(&combined);
    let mut tokens = HashSet::new();

    for raw_tok in segmented.split_whitespace() {
        let trimmed =
            raw_tok.trim_matches(|c: char| !(c.is_alphanumeric() || is_cjk(c) || c == '_'));
        if trimmed.is_empty() || !trimmed.chars().any(|c| c.is_alphanumeric() || is_cjk(c)) {
            continue;
        }
        tokens.insert(trimmed.to_lowercase());
    }

    tokens
}

fn compute_minhash_signature(tokens: &HashSet<String>) -> [u64; MINHASH_NUM_HASHES] {
    let mut sig = [u64::MAX; MINHASH_NUM_HASHES];
    for tok in tokens {
        let bytes = tok.as_bytes();
        for (idx, slot) in sig.iter_mut().enumerate() {
            let seed = (idx as u64)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                .wrapping_add(1);
            let h = xxh3_64_with_seed(bytes, seed);
            if h < *slot {
                *slot = h;
            }
        }
    }
    sig
}

fn exact_jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let (small, large) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    let inter = small.iter().filter(|tok| large.contains(*tok)).count();
    let union = a.len() + b.len() - inter;
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

fn detect_duplicates(
    storage: &SqliteStorage,
    docs: &[IndexedDoc],
    resolver: &LinkResolver,
) -> Result<Vec<DuplicateReport>, Box<dyn std::error::Error>> {
    let l3_indices: Vec<usize> = docs
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            item.doc.is_active() && item.doc.hierarchy == HierarchyLevel::L3Evergreen
        })
        .map(|(idx, _)| idx)
        .collect();

    if l3_indices.len() < 2 {
        return Ok(Vec::new());
    }

    let superseded_pairs = collect_supersession_pairs(docs, &l3_indices, resolver);
    let vectors_by_path = load_document_vectors(storage)?;

    let l3_vectors: Vec<Option<&Vec<f32>>> = l3_indices
        .iter()
        .map(|&idx| vectors_by_path.get(&docs[idx].path_str))
        .collect();

    let mut duplicates = Vec::new();
    let mut seen_pairs: HashSet<(usize, usize)> = HashSet::new();

    // 1. Vector cosine similarity for pairs where both cards have stored vectors.
    for i in 0..l3_indices.len() {
        let Some(vec_i) = l3_vectors[i] else {
            continue;
        };
        let doc_i = &docs[l3_indices[i]];
        for j in (i + 1)..l3_indices.len() {
            let Some(vec_j) = l3_vectors[j] else {
                continue;
            };
            let doc_j = &docs[l3_indices[j]];
            let id_pair = if doc_i.id < doc_j.id {
                (doc_i.id, doc_j.id)
            } else {
                (doc_j.id, doc_i.id)
            };
            if superseded_pairs.contains(&id_pair) {
                continue;
            }
            let cos = cosine_similarity(vec_i, vec_j);
            if cos > VECTOR_COSINE_DUP_THRESHOLD {
                seen_pairs.insert((i, j));
                let (a, b) = ordered_paths(&doc_i.path_str, &doc_j.path_str);
                duplicates.push(DuplicateReport {
                    a,
                    b,
                    score: cos,
                    method: "cosine".to_string(),
                });
            }
        }
    }

    // 2. MinHash-LSH fallback for any L3 card lacking a stored vector.
    let any_missing_vec = l3_vectors.iter().any(|v| v.is_none());
    if any_missing_vec {
        let token_sets: Vec<HashSet<String>> = l3_indices
            .iter()
            .map(|&idx| tokenize_for_jaccard(&docs[idx].doc.title, &docs[idx].doc.body))
            .collect();

        let mut band_buckets: Vec<HashMap<[u64; LSH_ROWS_PER_BAND], Vec<usize>>> =
            vec![HashMap::new(); LSH_BANDS];

        for (pos, tokens) in token_sets.iter().enumerate() {
            if tokens.is_empty() {
                continue;
            }
            let sig = compute_minhash_signature(tokens);
            for (band_idx, buckets) in band_buckets.iter_mut().enumerate() {
                let offset = band_idx * LSH_ROWS_PER_BAND;
                let key = [
                    sig[offset],
                    sig[offset + 1],
                    sig[offset + 2],
                    sig[offset + 3],
                ];
                buckets.entry(key).or_default().push(pos);
            }
        }

        let mut candidate_pairs: BTreeSet<(usize, usize)> = BTreeSet::new();
        for buckets in &band_buckets {
            for bucket in buckets.values() {
                if bucket.len() < 2 {
                    continue;
                }
                for p1 in 0..bucket.len() {
                    for p2 in (p1 + 1)..bucket.len() {
                        let i = bucket[p1];
                        let j = bucket[p2];
                        let (lo, hi) = if i < j { (i, j) } else { (j, i) };
                        if lo == hi {
                            continue;
                        }
                        if l3_vectors[lo].is_some() && l3_vectors[hi].is_some() {
                            continue;
                        }
                        candidate_pairs.insert((lo, hi));
                    }
                }
            }
        }

        for (i, j) in candidate_pairs {
            if seen_pairs.contains(&(i, j)) {
                continue;
            }
            let doc_i = &docs[l3_indices[i]];
            let doc_j = &docs[l3_indices[j]];
            let id_pair = if doc_i.id < doc_j.id {
                (doc_i.id, doc_j.id)
            } else {
                (doc_j.id, doc_i.id)
            };
            if superseded_pairs.contains(&id_pair) {
                continue;
            }

            let jaccard = exact_jaccard(&token_sets[i], &token_sets[j]);
            if jaccard >= JACCARD_DUP_THRESHOLD {
                let (a, b) = ordered_paths(&doc_i.path_str, &doc_j.path_str);
                duplicates.push(DuplicateReport {
                    a,
                    b,
                    score: jaccard,
                    method: "jaccard_lsh".to_string(),
                });
            }
        }
    }

    duplicates.sort_by(|x, y| {
        y.score
            .total_cmp(&x.score)
            .then_with(|| x.a.cmp(&y.a))
            .then_with(|| x.b.cmp(&y.b))
    });

    Ok(duplicates)
}

fn ordered_paths(p1: &str, p2: &str) -> (String, String) {
    if p1 <= p2 {
        (p1.to_string(), p2.to_string())
    } else {
        (p2.to_string(), p1.to_string())
    }
}

fn run_weave(
    vault_path: &Path,
    _storage: &SqliteStorage,
    docs: &[IndexedDoc],
    id_to_idx: &HashMap<i64, usize>,
    edges: &[(i64, i64)],
    opts: &GardenOptions,
) -> Result<WeaveReport, Box<dyn std::error::Error>> {
    let canon_vault = vault_path.canonicalize()?;
    let scan_filter = ScanFilter::from_vault_root(&canon_vault);

    // Map target doc_id -> list of active source doc indices.
    let mut sources_by_target: HashMap<i64, Vec<usize>> = HashMap::new();
    for &(src_id, dst_id) in edges {
        let (Some(&src_idx), Some(&dst_idx)) = (id_to_idx.get(&src_id), id_to_idx.get(&dst_id))
        else {
            continue;
        };
        if src_idx == dst_idx {
            continue;
        }
        let src_doc = &docs[src_idx].doc;
        let dst_doc = &docs[dst_idx].doc;
        if !dst_doc.is_active() || dst_doc.hierarchy != HierarchyLevel::L3Evergreen {
            continue;
        }
        if !src_doc.is_active() {
            continue;
        }
        let src_allowed = match src_doc.hierarchy {
            HierarchyLevel::L3Evergreen | HierarchyLevel::L1Resource => true,
            HierarchyLevel::L2Log => opts.include_logs,
            HierarchyLevel::L0Ephemeral => false,
        };
        if src_allowed {
            sources_by_target.entry(dst_id).or_default().push(src_idx);
        }
    }

    let mut report = WeaveReport::default();

    for target_item in docs {
        if !target_item.doc.is_active() || target_item.doc.hierarchy != HierarchyLevel::L3Evergreen
        {
            continue;
        }

        let src_indices = sources_by_target
            .get(&target_item.id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let has_existing_block = target_item.doc.body.contains(BACKLINK_BLOCK_START);

        if src_indices.is_empty() && !has_existing_block {
            continue;
        }

        // Deduplicate and sort source entries deterministically.
        let mut source_entries: Vec<(String, String, String)> = src_indices
            .iter()
            .map(|&s_idx| {
                let s = &docs[s_idx];
                let path_no_ext = strip_md_ext(&s.path_str).to_string();
                let clean_title = sanitize_link_title(&s.doc.title, &path_no_ext);
                (s.path_str.clone(), path_no_ext, clean_title)
            })
            .collect();
        source_entries.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        source_entries.dedup_by(|a, b| a.1 == b.1);

        // Safety checks per target file.
        let raw_target = vault_path.join(&target_item.path_str);
        let sym_meta = match fs::symlink_metadata(&raw_target) {
            Ok(m) => m,
            Err(err) => {
                report.skipped.push(WeaveSkipItem {
                    path: target_item.path_str.clone(),
                    reason: format!("unreadable metadata: {err}"),
                });
                continue;
            }
        };
        if sym_meta.file_type().is_symlink() {
            report.skipped.push(WeaveSkipItem {
                path: target_item.path_str.clone(),
                reason: "symlink".to_string(),
            });
            continue;
        }

        let canon_target = match raw_target.canonicalize() {
            Ok(p) => p,
            Err(err) => {
                report.skipped.push(WeaveSkipItem {
                    path: target_item.path_str.clone(),
                    reason: format!("canonicalize failed: {err}"),
                });
                continue;
            }
        };
        if !canon_target.starts_with(&canon_vault) || !canon_target.is_file() {
            report.skipped.push(WeaveSkipItem {
                path: target_item.path_str.clone(),
                reason: "outside vault".to_string(),
            });
            continue;
        }

        if sym_meta.len() > MAX_WEAVE_FILE_BYTES {
            report.skipped.push(WeaveSkipItem {
                path: target_item.path_str.clone(),
                reason: "exceeds 1 MiB".to_string(),
            });
            continue;
        }

        if target_item.doc.is_index_excluded()
            || scan_filter.is_excluded(Path::new(&target_item.path_str), false)
        {
            report.skipped.push(WeaveSkipItem {
                path: target_item.path_str.clone(),
                reason: "index excluded".to_string(),
            });
            continue;
        }

        let bytes = match fs::read(&canon_target) {
            Ok(b) => b,
            Err(err) => {
                report.skipped.push(WeaveSkipItem {
                    path: target_item.path_str.clone(),
                    reason: format!("read failed: {err}"),
                });
                continue;
            }
        };

        let disk_hash = format!("{:016x}", xxh3_64(&bytes));
        if disk_hash != target_item.doc.content_hash {
            report.skipped.push(WeaveSkipItem {
                path: target_item.path_str.clone(),
                reason: "changed since scan".to_string(),
            });
            continue;
        }

        let text = match String::from_utf8(bytes) {
            Ok(t) => t,
            Err(_) => {
                report.skipped.push(WeaveSkipItem {
                    path: target_item.path_str.clone(),
                    reason: "non-utf8".to_string(),
                });
                continue;
            }
        };

        let new_text = rewrite_managed_backlinks(&text, &source_entries);
        if new_text == text {
            continue;
        }

        if !opts.force {
            match crate::git::is_path_dirty(&canon_vault, &target_item.path_str) {
                Ok(true) => {
                    report.skipped.push(WeaveSkipItem {
                        path: target_item.path_str.clone(),
                        reason: "dirty git file".to_string(),
                    });
                    continue;
                }
                Err(err) => {
                    report.skipped.push(WeaveSkipItem {
                        path: target_item.path_str.clone(),
                        reason: format!("git status error: {err}"),
                    });
                    continue;
                }
                Ok(false) => {}
            }
        }

        report.planned.push(WeavePlanItem {
            target: target_item.path_str.clone(),
            add: source_entries.iter().map(|(p, _, _)| p.clone()).collect(),
        });

        if !opts.dry_run {
            atomic_write_preserving_perms(
                &canon_target,
                new_text.as_bytes(),
                sym_meta.permissions(),
            )?;
            report.applied += 1;
        }
    }

    Ok(report)
}

fn strip_md_ext(path: &str) -> &str {
    if path.len() >= 3 && path[path.len() - 3..].eq_ignore_ascii_case(".md") {
        &path[..path.len() - 3]
    } else {
        path
    }
}

fn sanitize_link_title(title: &str, fallback: &str) -> String {
    let cleaned = title
        .replace('\r', "")
        .replace('\n', " ")
        .replace('|', "/")
        .trim()
        .to_string();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn rewrite_managed_backlinks(text: &str, sources: &[(String, String, String)]) -> String {
    let uses_crlf = text.contains("\r\n");
    let nl = if uses_crlf { "\r\n" } else { "\n" };
    let had_trailing_nl = text.ends_with('\n');
    let trailing = if had_trailing_nl { nl } else { "" };

    let block_ranges = extract_managed_backlink_blocks(text);

    // Strip any existing managed blocks to isolate base content.
    let base_owned: String = if block_ranges.is_empty() {
        text.trim_end_matches(['\r', '\n']).to_string()
    } else if block_ranges.len() == 1 && text[block_ranges[0].1..].trim().is_empty() {
        text[..block_ranges[0].0]
            .trim_end_matches(['\r', '\n'])
            .to_string()
    } else {
        let mut rebuilt = String::with_capacity(text.len());
        let mut cursor = 0;
        for (start, end) in block_ranges {
            let segment = text[cursor..start].trim_end_matches(['\r', '\n']);
            if !segment.is_empty() {
                if !rebuilt.is_empty() {
                    rebuilt.push_str(nl);
                    rebuilt.push_str(nl);
                }
                rebuilt.push_str(segment);
            }
            cursor = end;
        }
        let tail = text[cursor..].trim_matches(['\r', '\n']);
        if !tail.is_empty() {
            if !rebuilt.is_empty() {
                rebuilt.push_str(nl);
                rebuilt.push_str(nl);
            }
            rebuilt.push_str(tail);
        }
        rebuilt
    };

    if sources.is_empty() {
        if base_owned.is_empty() {
            return String::new();
        }
        return format!("{base_owned}{trailing}");
    }

    let mut block = String::from(BACKLINK_BLOCK_START);
    for (_, path_no_ext, title) in sources {
        block.push_str(nl);
        block.push_str(&format!("- [[{path_no_ext}|{title}]]"));
    }
    block.push_str(nl);
    block.push_str(BACKLINK_BLOCK_END);

    if base_owned.is_empty() {
        format!("{block}{trailing}")
    } else {
        format!("{base_owned}{nl}{nl}{block}{trailing}")
    }
}

fn atomic_write_preserving_perms(
    target: &Path,
    bytes: &[u8],
    perms: fs::Permissions,
) -> Result<(), Box<dyn std::error::Error>> {
    let parent = target
        .parent()
        .ok_or("target file has no parent directory")?;
    let file_name = target
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("target file has invalid UTF-8 name")?;
    let tmp_path = parent.join(format!(".{file_name}.k0maru-weave.tmp"));

    let write_res = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::set_permissions(&tmp_path, perms)?;
        fs::rename(&tmp_path, target)?;
        Ok(())
    })();

    if write_res.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }

    write_res
}

/// Formats a `GardenReport` as human-readable terminal output.
pub fn format_garden_report(report: &GardenReport, dry_run: bool) -> String {
    let mut out = String::new();
    out.push_str("Garden Report\n");
    out.push_str(&format!("  Orphans:      {}\n", report.orphans.len()));
    for item in &report.orphans {
        out.push_str(&format!("    - {} ({})\n", item.path, item.hierarchy));
    }

    out.push_str(&format!("  Broken links: {}\n", report.broken_links.len()));
    for item in &report.broken_links {
        out.push_str(&format!("    - {} -> {}\n", item.source, item.raw));
    }

    out.push_str(&format!("  Duplicates:   {}\n", report.duplicates.len()));
    for item in &report.duplicates {
        out.push_str(&format!(
            "    - {} <-> {} [{}: {:.3}]\n",
            item.a, item.b, item.method, item.score
        ));
    }

    if !report.weave.planned.is_empty()
        || report.weave.applied > 0
        || !report.weave.skipped.is_empty()
    {
        let label = if dry_run { "Weave (dry-run)" } else { "Weave" };
        out.push_str(&format!(
            "  {label}: planned {}, applied {}, skipped {}\n",
            report.weave.planned.len(),
            report.weave.applied,
            report.weave.skipped.len()
        ));
        for item in &report.weave.planned {
            out.push_str(&format!(
                "    * {} ({} backlink(s))\n",
                item.target,
                item.add.len()
            ));
        }
        for item in &report.weave.skipped {
            out.push_str(&format!("    ! {} (skipped: {})\n", item.path, item.reason));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_indexed(id: i64, path: &str, title: &str, content: &str) -> IndexedDoc {
        let hash = format!("{:016x}", xxh3_64(content.as_bytes()));
        IndexedDoc {
            id,
            path_str: path.to_string(),
            doc: Document {
                path: PathBuf::from(path),
                title: title.to_string(),
                hierarchy: HierarchyLevel::L3Evergreen,
                frontmatter: serde_json::json!({ "title": title }),
                links: Vec::new(),
                tags: Vec::new(),
                content_hash: hash,
                mtime: 1_780_000_000,
                body: content.to_string(),
                body_line_offset: 0,
            },
        }
    }

    #[test]
    fn test_run_weave_toctou_skips_symlink_outside_and_changed_since_scan() {
        let tmp = tempdir().unwrap();
        let vault = tmp.path().join("vault");
        let outside = tmp.path().join("outside");
        fs::create_dir_all(vault.join("20_Cards")).unwrap();
        fs::create_dir_all(&outside).unwrap();

        let src_content = "# Source\nLinks to targets.\n";
        fs::write(vault.join("20_Cards/Source.md"), src_content).unwrap();

        let changed_initial = "# Changed\nInitial.";
        fs::write(
            vault.join("20_Cards/Changed.md"),
            "# Changed\nModified on disk.",
        )
        .unwrap();

        let no_nl_content = "# NoNewline\nBody without trailing newline";
        fs::write(vault.join("20_Cards/NoNewline.md"), no_nl_content).unwrap();

        let outside_content = "# External\nOutside vault.\n";
        fs::write(outside.join("External.md"), outside_content).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.join("External.md"), vault.join("20_Cards/Sym.md"))
            .unwrap();

        let mut docs = vec![
            make_indexed(1, "20_Cards/Source.md", "Source", src_content),
            make_indexed(2, "20_Cards/Changed.md", "Changed", changed_initial),
            make_indexed(3, "20_Cards/NoNewline.md", "NoNewline", no_nl_content),
        ];
        #[cfg(unix)]
        docs.push(make_indexed(4, "20_Cards/Sym.md", "Sym", outside_content));

        let mut id_to_idx = HashMap::new();
        for (i, d) in docs.iter().enumerate() {
            id_to_idx.insert(d.id, i);
        }

        let mut edges = vec![(1, 2), (1, 3)];
        #[cfg(unix)]
        edges.push((1, 4));

        let storage = SqliteStorage::in_memory().unwrap();
        let report = run_weave(
            &vault,
            &storage,
            &docs,
            &id_to_idx,
            &edges,
            &GardenOptions {
                weave: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(report.applied, 1);
        assert!(report
            .skipped
            .iter()
            .any(|s| s.path == "20_Cards/Changed.md" && s.reason == "changed since scan"));
        #[cfg(unix)]
        assert!(report
            .skipped
            .iter()
            .any(|s| s.path == "20_Cards/Sym.md" && s.reason == "symlink"));

        // Verify no-trailing-newline preservation and no leftover temp files.
        let written = fs::read_to_string(vault.join("20_Cards/NoNewline.md")).unwrap();
        assert!(!written.ends_with('\n'));
        assert!(written.ends_with(BACKLINK_BLOCK_END));
        assert!(!vault
            .join("20_Cards/.NoNewline.md.k0maru-weave.tmp")
            .exists());
    }
}
