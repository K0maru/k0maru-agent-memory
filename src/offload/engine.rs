use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Result of evaluating a stream of logs against the offload threshold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OffloadResult {
    /// Unique identifier for the offloaded node (e.g. `node_1a2b3c4d`).
    pub node_id: String,
    /// Whether the log stream exceeded threshold and was offloaded.
    pub truncated: bool,
    /// Total number of lines processed.
    pub total_lines: usize,
    /// Path to the persisted log file if truncated.
    pub log_path: Option<PathBuf>,
    /// Compact Mermaid state machine diagram representing the offloaded execution.
    pub mermaid_graph: String,
    /// Summary statistics and inspection advice for agent context.
    pub summary_text: String,
}

/// Offload engine for intercepting verbose command logs, externalizing long outputs
/// into disk references, and generating compact symbolic Mermaid diagrams.
#[derive(Debug, Clone)]
pub struct OffloadEngine {
    /// Maximum line count threshold before offloading is triggered (default 50).
    pub threshold: usize,
    /// Optional task identifier prefix for offloaded file names.
    pub task_id: Option<String>,
    /// Directory where offloaded raw log files are saved.
    pub refs_dir: PathBuf,
}

impl Default for OffloadEngine {
    fn default() -> Self {
        Self {
            threshold: 50,
            task_id: None,
            refs_dir: PathBuf::from(".scratch/refs"),
        }
    }
}

impl OffloadEngine {
    /// Create a new OffloadEngine targeting the specified refs directory.
    pub fn new(refs_dir: impl Into<PathBuf>) -> Self {
        Self {
            threshold: 50,
            task_id: None,
            refs_dir: refs_dir.into(),
        }
    }

    /// Set a custom line count threshold.
    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set a task identifier prefix.
    pub fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    /// Process an input stream of log lines from a reader (such as `std::io::stdin`).
    pub fn process_stream<R: BufRead>(
        &self,
        mut reader: R,
    ) -> Result<OffloadResult, Box<dyn std::error::Error>> {
        let mut lines = Vec::new();
        let mut line_buf = String::new();

        loop {
            line_buf.clear();
            match reader.read_line(&mut line_buf) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line_buf.trim_end_matches(['\r', '\n']);
                    lines.push(trimmed.to_string());
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::BrokenPipe {
                        break;
                    }
                    return Err(Box::new(e));
                }
            }
        }

        let total_lines = lines.len();

        if total_lines <= self.threshold {
            let summary_text = if lines.is_empty() {
                String::new()
            } else {
                lines.join("\n")
            };

            return Ok(OffloadResult {
                node_id: String::new(),
                truncated: false,
                total_lines,
                log_path: None,
                mermaid_graph: String::new(),
                summary_text,
            });
        }

        // Long logs: persist and generate Mermaid state diagram
        let raw_content = lines.join("\n");
        let hash = xxhash_rust::xxh3::xxh3_64(raw_content.as_bytes());
        let short_hash = format!("{:08x}", (hash as u32));
        let node_id = format!("node_{}", short_hash);

        if !self.refs_dir.exists() {
            fs::create_dir_all(&self.refs_dir)?;
        }

        let file_name = match &self.task_id {
            Some(tid) if !tid.is_empty() => format!("{}_{}.log", tid, node_id),
            _ => format!("{}.log", node_id),
        };

        let log_path = self.refs_dir.join(&file_name);
        let mut file_content = raw_content;
        if !file_content.ends_with('\n') {
            file_content.push('\n');
        }
        fs::write(&log_path, file_content)?;

        let task_label = match &self.task_id {
            Some(t) if !t.is_empty() => {
                let sanitized: String = t
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '_' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();
                format!("Task_{}", sanitized)
            }
            _ => "Running".to_string(),
        };

        let mermaid_graph = format!(
            "stateDiagram-v2\n    direction TB\n    [*] --> {task_label}\n    {task_label} --> Offloaded : >{threshold} lines ({total_lines} lines total)\n    state Offloaded {{\n        [*] --> Captured\n        Captured : Total {total_lines} lines captured\n        Captured : NodeID {node_id}\n        Captured : k0maru inspect {node_id}\n    }}\n    Offloaded --> [*]",
            threshold = self.threshold,
            total_lines = total_lines,
            task_label = task_label,
            node_id = node_id,
        );

        let summary_text = format!(
            "[Offloaded] {} lines exceeded threshold ({}) -> saved to {}\n💡 Inspect trace: k0maru inspect {}",
            total_lines,
            self.threshold,
            log_path.display(),
            node_id
        );

        Ok(OffloadResult {
            node_id,
            truncated: true,
            total_lines,
            log_path: Some(log_path),
            mermaid_graph,
            summary_text,
        })
    }

    /// Helper to process an in-memory string directly.
    pub fn offload_text(&self, text: &str) -> Result<OffloadResult, Box<dyn std::error::Error>> {
        self.process_stream(std::io::Cursor::new(text.as_bytes()))
    }
}

/// Look up and read back the full offloaded raw log content from refs_dir for a given node_id.
pub fn inspect_node(refs_dir: &Path, node_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let clean_id = node_id.strip_suffix(".log").unwrap_or(node_id).trim();
    if clean_id.is_empty() {
        return Err("Node ID cannot be empty".into());
    }

    // Direct path candidates
    let direct_with_ext = refs_dir.join(format!("{}.log", clean_id));
    if direct_with_ext.is_file() {
        return Ok(fs::read_to_string(&direct_with_ext)?);
    }

    let direct_exact = refs_dir.join(node_id);
    if direct_exact.is_file() {
        return Ok(fs::read_to_string(&direct_exact)?);
    }

    if !refs_dir.exists() {
        return Err(format!("Refs directory '{}' does not exist", refs_dir.display()).into());
    }

    // Scan refs_dir for files matching _{clean_id}.log or {clean_id}.log
    let mut matches = Vec::new();
    for entry in fs::read_dir(refs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                let stem = fname.strip_suffix(".log").unwrap_or(fname);
                if stem == clean_id
                    || fname.ends_with(&format!("_{}.log", clean_id))
                    || stem.ends_with(&format!("_{}", clean_id))
                {
                    matches.push(path);
                }
            }
        }
    }

    // Substring fallback if exact/suffix match not found
    if matches.is_empty() {
        for entry in fs::read_dir(refs_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                    if fname.contains(clean_id) && fname.ends_with(".log") {
                        matches.push(path);
                    }
                }
            }
        }
    }

    if let Some(matched_path) = matches.first() {
        return Ok(fs::read_to_string(matched_path)?);
    }

    Err(format!("Log node '{}' not found in {}", node_id, refs_dir.display()).into())
}
