use std::fs;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::adapters::GenericWikiAdapter;
use crate::convention::{generate_filename, slugify, NamingStyle};
use crate::scanner::IncrementalScanner;
use crate::storage::SqliteStorage;

/// Represents a Linda tuple in the local `~/.k0maru/tuples/` space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LindaTuple {
    pub id: String,
    pub kind: String,
    pub domain: String,
    pub task_id: String,
    pub payload: String,
    pub created_at: String,
}

/// Parsed structure of typical task payload within Linda tuples.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskPayload {
    pub status: Option<String>,
    pub commit: Option<String>,
    pub summary: Option<String>,
    pub title: Option<String>,
    pub kdsl: Option<String>,
    pub invariants: Option<Vec<String>>,
    pub deliverables: Option<Vec<String>>,
}

/// Result of an individual tuple crystallization action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizeResult {
    pub tuple_id: String,
    pub task_id: String,
    pub domain: String,
    pub relative_path: PathBuf,
    pub absolute_path: PathBuf,
    pub consumed: bool,
    pub dry_run: bool,
}

/// Resolves the default Linda tuple directory (`~/.k0maru/tuples/`).
pub fn resolve_tuple_dir(custom_path: Option<&Path>) -> PathBuf {
    if let Some(p) = custom_path {
        return p.to_path_buf();
    }
    if let Ok(env_dir) = std::env::var("K0MARU_TUPLE_DIR") {
        if !env_dir.trim().is_empty() {
            return PathBuf::from(env_dir.trim());
        }
    }
    if let Some(home) = dirs::home_dir() {
        return home.join(".k0maru").join("tuples");
    }
    PathBuf::from(".k0maru/tuples")
}

/// Scans the tuple space for matching tuples without modification.
pub fn find_tuples(
    tuple_dir: &Path,
    kind_filter: Option<&str>,
    domain_filter: Option<&str>,
    task_id_filter: Option<&str>,
) -> Vec<(PathBuf, LindaTuple)> {
    let mut results = Vec::new();
    if !tuple_dir.is_dir() {
        return results;
    }

    let entries = match fs::read_dir(tuple_dir) {
        Ok(e) => e,
        Err(_) => return results,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(tuple) = serde_json::from_str::<LindaTuple>(&content) {
                    if let Some(kf) = kind_filter {
                        if !tuple.kind.eq_ignore_ascii_case(kf) {
                            continue;
                        }
                    }
                    if let Some(df) = domain_filter {
                        if !tuple.domain.eq_ignore_ascii_case(df) {
                            continue;
                        }
                    }
                    if let Some(tf) = task_id_filter {
                        if !tuple.task_id.eq_ignore_ascii_case(tf) {
                            continue;
                        }
                    }
                    results.push((path, tuple));
                }
            }
        }
    }

    results.sort_by(|a, b| a.1.created_at.cmp(&b.1.created_at));
    results
}

/// Autonomous Tuple Crystallizer engine.
pub struct TupleCrystallizer {
    vault_root: PathBuf,
    tuple_dir: PathBuf,
}

impl TupleCrystallizer {
    pub fn new(vault_root: impl Into<PathBuf>, tuple_dir: Option<PathBuf>) -> Self {
        let vr = vault_root.into();
        let td = resolve_tuple_dir(tuple_dir.as_deref());
        Self {
            vault_root: vr,
            tuple_dir: td,
        }
    }

    pub fn tuple_dir(&self) -> &Path {
        &self.tuple_dir
    }

    pub fn vault_root(&self) -> &Path {
        &self.vault_root
    }

    /// Crystallizes a single Linda tuple into `01_AI_Logs/` within the vault.
    pub fn crystallize_tuple(
        &self,
        tuple_path: &Path,
        tuple: &LindaTuple,
        consume: bool,
        dry_run: bool,
    ) -> Result<CrystallizeResult, Box<dyn std::error::Error>> {
        let payload: TaskPayload =
            serde_json::from_str(&tuple.payload).unwrap_or_else(|_| TaskPayload {
                summary: Some(tuple.payload.clone()),
                ..Default::default()
            });

        let today = Local::now().format("%Y-%m-%d").to_string();
        let year = Local::now().format("%Y").to_string();
        let project_name = match tuple.domain.to_uppercase().as_str() {
            "MEMORY" => "K0maru-Agent-Memory",
            "ROUTER" => "K0maru-Agent-Router",
            "VERIFIER" => "K0maru-Agent-Verifier",
            "WORKSPACE" => "K0maru-Agent-Workspace",
            _ => &tuple.domain,
        };

        let title = payload.title.as_deref().unwrap_or(tuple.task_id.as_str());

        // Target directory: 01_AI_Logs
        let log_dir = self.vault_root.join("01_AI_Logs");
        if !dry_run && !log_dir.exists() {
            fs::create_dir_all(&log_dir)?;
        }

        // Generate filename
        let base_slug = slugify(&format!("task-{}", tuple.task_id));
        let filename = generate_filename(&base_slug, &NamingStyle::DateSlug, None);
        let dest_path = log_dir.join(&filename);

        // Collision avoidance
        let mut final_path = dest_path;
        let mut counter = 2;
        while final_path.exists() && !dry_run {
            let stem = format!("{}-task-{}-v{}", today, slugify(&tuple.task_id), counter);
            final_path = log_dir.join(format!("{}.md", stem));
            counter += 1;
        }

        let relative_path = final_path
            .strip_prefix(&self.vault_root)
            .unwrap_or(&final_path)
            .to_path_buf();

        // Build Markdown content
        let mut md = String::new();
        md.push_str("---\n");
        md.push_str(&format!("date: {}\n", today));
        md.push_str("tags:\n");
        md.push_str("  - type/ai-log\n");
        md.push_str("  - status/completed\n");
        md.push_str(&format!("project: \"[[10_Projects/{}]]\"\n", project_name));
        md.push_str(&format!("task_id: \"{}\"\n", tuple.task_id));
        md.push_str(&format!("domain: \"{}\"\n", tuple.domain));
        if let Some(ref c) = payload.commit {
            md.push_str(&format!("commit: \"{}\"\n", c));
        }
        md.push_str("---\n\n");

        md.push_str(&format!("# 🚀 任务自主结晶报告: {}\n\n", title));

        md.push_str("## 📌 任务元数据\n");
        md.push_str(&format!("- **任务编号 (Task ID)**: `{}`\n", tuple.task_id));
        md.push_str(&format!("- **目标领域 (Domain)**: `{}`\n", tuple.domain));
        md.push_str(&format!(
            "- **执行状态 (Status)**: `{}`\n",
            payload.status.as_deref().unwrap_or("COMPLETED")
        ));
        if let Some(ref c) = payload.commit {
            md.push_str(&format!("- **最终提交 (Commit)**: `{}`\n", c));
        }
        md.push_str(&format!(
            "- **完成时间 (Timestamp)**: `{}`\n\n",
            tuple.created_at
        ));

        md.push_str("## 📋 交付物与执行总结\n");
        if let Some(ref summary) = payload.summary {
            md.push_str(&format!("{}\n\n", summary));
        }
        if let Some(ref deliverables) = payload.deliverables {
            for d in deliverables {
                md.push_str(&format!("- [x] {}\n", d));
            }
            md.push('\n');
        }

        if let Some(ref kdsl) = payload.kdsl {
            md.push_str("## ⚡ 机器原生通信协议 (K-DSL Trace)\n```text\n");
            md.push_str(kdsl);
            md.push_str("\n```\n\n");
        }

        if let Some(ref invs) = payload.invariants {
            md.push_str("## 🛡️ 物理验证与架构不变量 (Verified Invariants)\n");
            for inv in invs {
                md.push_str(&format!("- ✓ `{}`\n", inv));
            }
            md.push('\n');
        }

        md.push_str("---\n\n## 🔗 关联双链 (WikiLinks)\n");
        md.push_str(&format!("- [[00_Daily/{}/{}]]\n", year, today));
        md.push_str(&format!("- [[10_Projects/{}]]\n", project_name));

        if !dry_run {
            fs::write(&final_path, &md)?;

            // Instant cache synchronization
            let cache_dir = self.vault_root.join(".k0maru");
            let cache_path = cache_dir.join("cache.sqlite");
            if let Ok(mut storage) = SqliteStorage::open(&cache_path) {
                let adapter = GenericWikiAdapter::new(&self.vault_root);
                let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
                let _ = scanner.sync(false);
            }

            // Consume tuple if requested
            if consume {
                let _ = fs::remove_file(tuple_path);
            }
        }

        Ok(CrystallizeResult {
            tuple_id: tuple.id.clone(),
            task_id: tuple.task_id.clone(),
            domain: tuple.domain.clone(),
            relative_path,
            absolute_path: final_path,
            consumed: consume && !dry_run,
            dry_run,
        })
    }

    /// Runs one pass over all matching tuples.
    pub fn crystallize_all(
        &self,
        kind_filter: Option<&str>,
        domain_filter: Option<&str>,
        task_id_filter: Option<&str>,
        consume: bool,
        dry_run: bool,
    ) -> Result<Vec<CrystallizeResult>, Box<dyn std::error::Error>> {
        let tuples = find_tuples(&self.tuple_dir, kind_filter, domain_filter, task_id_filter);

        let mut results = Vec::new();
        for (path, tuple) in tuples {
            let res = self.crystallize_tuple(&path, &tuple, consume, dry_run)?;
            results.push(res);
        }

        Ok(results)
    }

    /// Continuously watches for new tuples and crystallizes them.
    pub fn watch(
        &self,
        kind_filter: Option<&str>,
        domain_filter: Option<&str>,
        task_id_filter: Option<&str>,
        consume: bool,
        interval: Duration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let _ =
                self.crystallize_all(kind_filter, domain_filter, task_id_filter, consume, false);
            sleep(interval);
        }
    }
}
