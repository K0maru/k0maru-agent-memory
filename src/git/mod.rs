//! Git working tree and diff inspection module.
//!
//! Provides zero-dependency Git change inspection and unified diff stream parsing
//! for change-aware dynamic loadout activation.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use k0maru_core::storage::migrate::META_GIT_TIME_HEAD;
use k0maru_core::storage::SqliteStorage;

/// Status of a modified file detected in the Git index or working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileChangeStatus {
    Modified,
    Added,
    Deleted,
    Untracked,
    Renamed,
    Unknown,
}

impl FileChangeStatus {
    pub fn from_status_char(c: char) -> Self {
        match c {
            'M' => FileChangeStatus::Modified,
            'A' => FileChangeStatus::Added,
            'D' => FileChangeStatus::Deleted,
            '?' => FileChangeStatus::Untracked,
            'R' => FileChangeStatus::Renamed,
            _ => FileChangeStatus::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            FileChangeStatus::Modified => "modified",
            FileChangeStatus::Added => "added",
            FileChangeStatus::Deleted => "deleted",
            FileChangeStatus::Untracked => "untracked",
            FileChangeStatus::Renamed => "renamed",
            FileChangeStatus::Unknown => "unknown",
        }
    }
}

/// Represents a modified file detected in a Git repository or diff stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifiedFile {
    pub path: PathBuf,
    pub status: FileChangeStatus,
}

impl ModifiedFile {
    pub fn new(path: impl Into<PathBuf>, status: FileChangeStatus) -> Self {
        Self {
            path: path.into(),
            status,
        }
    }

    /// Extracts domain and module keywords from the file path.
    ///
    /// For example: `src/storage/sqlite.rs` -> `["storage", "sqlite"]`
    /// `crates/k0maru-core/src/vector/embedding.rs` -> `["k0maru-core", "vector", "embedding"]`
    pub fn extract_keywords(&self) -> Vec<String> {
        let mut keywords = Vec::new();
        let mut seen = HashSet::new();

        // Standard build/layout folders to skip
        let ignore_dirs = [
            "src",
            "crates",
            "tests",
            "target",
            "benches",
            "examples",
            "docs",
            "doc",
            "lib",
            ".github",
            ".git",
            ".cargo",
            "node_modules",
        ];

        for component in self.path.components() {
            let seg = component.as_os_str().to_string_lossy();
            let seg_str = seg.trim();

            if seg_str.is_empty() || ignore_dirs.contains(&seg_str) {
                continue;
            }

            // If it's a file with extension, extract stem
            let clean_stem = if let Some(idx) = seg_str.rfind('.') {
                &seg_str[..idx]
            } else {
                seg_str
            };

            let lower = clean_stem.to_lowercase();
            // Split by underscores and dashes
            let sub_parts: Vec<&str> = lower.split(['_', '-']).filter(|p| p.len() >= 2).collect();

            if sub_parts.len() > 1 && !seen.contains(&lower) && lower.len() >= 2 {
                seen.insert(lower.clone());
                keywords.push(lower.clone());
            }

            for part in sub_parts {
                let part_str = part.to_string();
                if part_str.len() >= 2 && !seen.contains(&part_str) {
                    seen.insert(part_str.clone());
                    keywords.push(part_str);
                }
            }
        }

        keywords
    }
}

/// Zero-dependency Git inspector using `std::process::Command::new("git")`.
pub struct GitInspector;

impl GitInspector {
    /// Checks whether the given directory is inside a Git repository.
    pub fn is_git_repo(repo_root: &Path) -> bool {
        Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .current_dir(repo_root)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Inspects the Git repository for working tree, staged, untracked, and branch diffs.
    ///
    /// Gracefully returns an empty `Vec` if the directory is not a Git repository
    /// or if `git` is unavailable.
    pub fn inspect_diff(
        repo_root: &Path,
        branch: Option<&str>,
    ) -> Result<Vec<ModifiedFile>, Box<dyn std::error::Error>> {
        if !Self::is_git_repo(repo_root) {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        let mut seen_paths = HashSet::new();

        // 1. Branch comparison diff if branch is specified
        if let Some(target_branch) = branch {
            // First try triple dot: target_branch...HEAD
            let branch_diff_arg = format!("{}...HEAD", target_branch);
            let mut output = Command::new("git")
                .arg("diff")
                .arg("--name-status")
                .arg(&branch_diff_arg)
                .current_dir(repo_root)
                .output();

            // Fallback to direct diff if triple-dot fails
            if output.as_ref().map(|o| !o.status.success()).unwrap_or(true) {
                output = Command::new("git")
                    .arg("diff")
                    .arg("--name-status")
                    .arg(target_branch)
                    .current_dir(repo_root)
                    .output();
            }

            if let Ok(out) = output {
                if out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    for item in parse_diff_stream(text.as_bytes())? {
                        if seen_paths.insert(item.path.clone()) {
                            files.push(item);
                        }
                    }
                }
            }
        }

        // 2. Working tree status (staged, unstaged, untracked) via porcelain
        let status_output = Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .arg("-uall")
            .current_dir(repo_root)
            .output();

        if let Ok(out) = status_output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                for item in parse_diff_stream(text.as_bytes())? {
                    if seen_paths.insert(item.path.clone()) {
                        files.push(item);
                    }
                }
            }
        }

        // 3. Unstaged working tree diff (--name-status)
        let unstaged_output = Command::new("git")
            .arg("diff")
            .arg("--name-status")
            .current_dir(repo_root)
            .output();

        if let Ok(out) = unstaged_output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                for item in parse_diff_stream(text.as_bytes())? {
                    if seen_paths.insert(item.path.clone()) {
                        files.push(item);
                    }
                }
            }
        }

        // 4. Staged diff (--cached --name-status)
        let staged_output = Command::new("git")
            .arg("diff")
            .arg("--cached")
            .arg("--name-status")
            .current_dir(repo_root)
            .output();

        if let Ok(out) = staged_output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                for item in parse_diff_stream(text.as_bytes())? {
                    if seen_paths.insert(item.path.clone()) {
                        files.push(item);
                    }
                }
            }
        }

        Ok(files)
    }
}

/// Cleans raw Git paths: removes surrounding quotes and strips "a/" or "b/" prefix.
fn clean_git_path(raw: &str) -> String {
    let trimmed = raw.trim();
    let unquoted = if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    if let Some(stripped) = unquoted.strip_prefix("b/") {
        stripped.to_string()
    } else if let Some(stripped) = unquoted.strip_prefix("a/") {
        stripped.to_string()
    } else {
        unquoted.to_string()
    }
}

/// Parses `diff --git <partA> <partB>`, handling paths with quotes and spaces.
fn parse_git_diff_header(rest: &str) -> Option<String> {
    let rest = rest.trim();
    let (a_raw, b_raw) = if rest.starts_with('"') {
        let mut in_escape = false;
        let mut close_quote_idx = None;
        for (i, c) in rest.char_indices().skip(1) {
            if in_escape {
                in_escape = false;
            } else if c == '\\' {
                in_escape = true;
            } else if c == '"' {
                close_quote_idx = Some(i);
                break;
            }
        }
        let end_a = close_quote_idx?;
        (&rest[..=end_a], rest[end_a + 1..].trim())
    } else {
        let (a, b) = rest.split_once(' ')?;
        (a, b.trim())
    };

    let target_b = clean_git_path(b_raw);
    if !target_b.is_empty() && target_b != "/dev/null" {
        Some(target_b)
    } else {
        let target_a = clean_git_path(a_raw);
        if !target_a.is_empty() && target_a != "/dev/null" {
            Some(target_a)
        } else {
            None
        }
    }
}

/// Parses a diff stream (unified diff, name-status, porcelain status, or raw path list)
/// from any reader (such as stdin or a patch file).
pub fn parse_diff_stream<R: BufRead>(
    reader: R,
) -> Result<Vec<ModifiedFile>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    let mut seen_paths = HashSet::new();

    for line_res in reader.lines() {
        let line = line_res?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Format A: Unified diff header: `diff --git a/path b/path`
        if let Some(rest) = trimmed.strip_prefix("diff --git ") {
            if let Some(target) = parse_git_diff_header(rest) {
                let path = PathBuf::from(target);
                if seen_paths.insert(path.clone()) {
                    files.push(ModifiedFile::new(path, FileChangeStatus::Modified));
                }
            }
            continue;
        }

        // Format B: Unified diff file target: `+++ b/path`
        if let Some(rest) = trimmed.strip_prefix("+++ ") {
            let target = clean_git_path(rest);
            if !target.is_empty() && target != "/dev/null" {
                let path = PathBuf::from(target);
                if seen_paths.insert(path.clone()) {
                    files.push(ModifiedFile::new(path, FileChangeStatus::Modified));
                }
            }
            continue;
        }

        // Format C: Git status porcelain: `XY path` or `?? path`
        if let Some(stripped) = trimmed.strip_prefix("?? ") {
            let target = clean_git_path(stripped);
            if !target.is_empty() {
                let path = PathBuf::from(target);
                if seen_paths.insert(path.clone()) {
                    files.push(ModifiedFile::new(path, FileChangeStatus::Untracked));
                }
            }
            continue;
        }

        // Check 2-character porcelain status code (e.g. ` M path`, `M  path`, `A  path`, `D  path`)
        if line.len() >= 4 {
            let first_two = &line[..2];
            let is_porcelain = match first_two {
                " M" | "M " | "MM" | "AM" => Some(FileChangeStatus::Modified),
                " A" | "A " | "AA" => Some(FileChangeStatus::Added),
                " D" | "D " => Some(FileChangeStatus::Deleted),
                " R" | "R " => Some(FileChangeStatus::Renamed),
                _ => None,
            };

            if let Some(status) = is_porcelain {
                if line.as_bytes().get(2) == Some(&b' ') {
                    let rest = line[3..].trim();
                    let target_raw = if let Some((_old, new)) = rest.split_once(" -> ") {
                        new.trim()
                    } else {
                        rest
                    };
                    let target = clean_git_path(target_raw);
                    if !target.is_empty() {
                        let path = PathBuf::from(target);
                        if seen_paths.insert(path.clone()) {
                            files.push(ModifiedFile::new(path, status));
                        }
                        continue;
                    }
                }
            }
        }

        // Format D: Tab-separated name-status: `M\tpath` or `A\tpath` or `R100\told\tnew`
        if let Some((status_part, rest)) = trimmed.split_once('\t') {
            let status_char = status_part.chars().next().unwrap_or('?');
            let status = FileChangeStatus::from_status_char(status_char);
            let target_raw = if status == FileChangeStatus::Renamed {
                // R100\told\tnew
                rest.split('\t').next_back().unwrap_or(rest).trim()
            } else {
                rest.trim()
            };
            let target = clean_git_path(target_raw);
            if !target.is_empty() {
                let path = PathBuf::from(target);
                if seen_paths.insert(path.clone()) {
                    files.push(ModifiedFile::new(path, status));
                }
                continue;
            }
        }

        // Ignore diff body lines, hunk lines, and git metadata
        // Unified diff context lines start with ' ' and must never leak into modified file lists
        if line.starts_with(' ')
            || line.starts_with('\t')
            || line.starts_with('+')
            || line.starts_with('-')
            || line.starts_with('@')
            || line.starts_with('\\')
            || trimmed.starts_with('#')
            || trimmed.starts_with("index ")
            || trimmed.starts_with("old mode")
            || trimmed.starts_with("new mode")
            || trimmed.starts_with("deleted file mode")
            || trimmed.starts_with("new file mode")
            || trimmed.starts_with("similarity index")
            || trimmed.starts_with("rename from")
            || trimmed.starts_with("rename to")
            || trimmed.starts_with("Binary files")
            || trimmed.starts_with("Subproject commit")
            || trimmed.starts_with("GIT binary patch")
            || trimmed.contains(" changed, ")
            || trimmed.ends_with(';')
            || trimmed.ends_with('{')
            || trimmed.ends_with('}')
            || trimmed.starts_with("use ")
            || trimmed.starts_with("pub ")
            || trimmed.starts_with("let ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("def ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("import ")
            || trimmed.starts_with("return ")
        {
            continue;
        }

        // Format E: Plain name list: `path/to/file.rs`
        // Extra guard: Ignore lines that look like code syntax (e.g. function calls, colons, parentheses)
        if trimmed.contains('(')
            || trimmed.contains(')')
            || trimmed.contains('=')
            || trimmed.contains(':')
            || trimmed.contains(';')
        {
            continue;
        }

        let target = clean_git_path(trimmed);
        if !target.is_empty() {
            let path = PathBuf::from(target);
            if seen_paths.insert(path.clone()) {
                files.push(ModifiedFile::new(path, FileChangeStatus::Modified));
            }
        }
    }

    Ok(files)
}

const GIT_WALL_TIMEOUT: Duration = Duration::from_secs(30);

/// Result of running the `sync --git [--git-pull]` pre-scan workflow.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitSyncOutcome {
    pub preflight_ok: bool,
    pub pulled: bool,
    pub git_times_updated: bool,
    pub warnings: Vec<String>,
}

/// Validated Git work-tree context for a vault directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitPreflight {
    /// Vault path relative to repository top-level (`""` when vault is the repo root).
    pub vault_rel: String,
}

/// Runs a `git` subprocess in `vault` with hook-env hygiene, non-interactive stdio,
/// and a hard 30-second wall-clock timeout.
pub fn run_git_command(vault: &Path, args: &[&str]) -> Result<Output, String> {
    run_git_command_with_timeout(vault, args, GIT_WALL_TIMEOUT)
}

/// Runs a `git` subprocess with a configurable wall-clock timeout.
pub fn run_git_command_with_timeout(
    vault: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(vault)
        .arg("-c")
        .arg("core.quotepath=off")
        .arg("--no-optional-locks")
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn git: {e}"))?;

    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let stdout_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(ref mut pipe) = stdout_pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });
    let stderr_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(ref mut pipe) = stderr_pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_handle.join();
                    let _ = stderr_handle.join();
                    return Err(format!(
                        "git command timed out after {}s: git {}",
                        timeout.as_secs(),
                        args.join(" ")
                    ));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_handle.join();
                let _ = stderr_handle.join();
                return Err(format!("failed waiting for git process: {e}"));
            }
        }
    };

    let stdout = stdout_handle.join().unwrap_or_default();
    let stderr = stderr_handle.join().unwrap_or_default();

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// Validates that `vault` is inside a Git working tree with no in-progress merge, rebase, or cherry-pick.
pub fn git_preflight(vault: &Path) -> Result<GitPreflight, String> {
    let inside = run_git_command(vault, &["rev-parse", "--is-inside-work-tree"])?;
    if !inside.status.success() || String::from_utf8_lossy(&inside.stdout).trim() != "true" {
        return Err("vault is not inside a git working tree".to_string());
    }

    let toplevel_out = run_git_command(vault, &["rev-parse", "--show-toplevel"])?;
    if !toplevel_out.status.success() {
        return Err("failed to resolve git repository toplevel".to_string());
    }
    let toplevel_raw = String::from_utf8_lossy(&toplevel_out.stdout)
        .trim()
        .to_string();
    let canon_top = std::fs::canonicalize(&toplevel_raw)
        .map_err(|e| format!("failed to canonicalize git toplevel: {e}"))?;
    let canon_vault = std::fs::canonicalize(vault)
        .map_err(|e| format!("failed to canonicalize vault path: {e}"))?;

    let rel = canon_vault
        .strip_prefix(&canon_top)
        .map_err(|_| "vault path does not reside inside git work tree".to_string())?;
    let vault_rel = rel.to_string_lossy().replace('\\', "/");

    for marker in [
        "MERGE_HEAD",
        "REBASE_HEAD",
        "CHERRY_PICK_HEAD",
        "rebase-merge",
        "rebase-apply",
    ] {
        let gp_out = run_git_command(vault, &["rev-parse", "--git-path", marker])?;
        if gp_out.status.success() {
            let gp_raw = String::from_utf8_lossy(&gp_out.stdout).trim().to_string();
            if !gp_raw.is_empty() {
                let p = Path::new(&gp_raw);
                let abs = if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    canon_vault.join(p)
                };
                if abs.exists() {
                    return Err(format!("git operation in progress ({marker})"));
                }
            }
        }
    }

    Ok(GitPreflight { vault_rel })
}

/// Returns `true` if `rel_path` (relative to `vault`) has uncommitted Git changes or is untracked.
///
/// Returns `Ok(false)` when `vault` is not inside a Git working tree.
pub fn is_path_dirty(vault: &Path, rel_path: &str) -> Result<bool, String> {
    let inside = run_git_command(vault, &["rev-parse", "--is-inside-work-tree"])?;
    if !inside.status.success() || String::from_utf8_lossy(&inside.stdout).trim() != "true" {
        return Ok(false);
    }

    let status_out = run_git_command(vault, &["status", "--porcelain", "--", rel_path])?;
    if !status_out.status.success() {
        let stderr = String::from_utf8_lossy(&status_out.stderr);
        return Err(format!("git status failed: {}", stderr.trim()));
    }

    Ok(!String::from_utf8_lossy(&status_out.stdout)
        .trim()
        .is_empty())
}

/// Parses NUL-delimited `git log --no-renames --format=%x1e%ct --name-only -z` output
/// into `(vault_relative_path, last_commit_unix_secs)` pairs.
pub fn parse_git_log_times(bytes: &[u8], vault_rel: &str) -> Vec<(String, i64)> {
    let prefix = if vault_rel.is_empty() {
        String::new()
    } else {
        format!("{}/", vault_rel.trim_end_matches('/'))
    };

    let mut seen: HashSet<String> = HashSet::new();
    let mut times: Vec<(String, i64)> = Vec::new();

    for record in bytes.split(|&b| b == 0x1e) {
        if record.is_empty() {
            continue;
        }
        let mut parts = record.split(|&b| b == 0);
        let Some(ts_bytes) = parts.next() else {
            continue;
        };
        let Ok(commit_ts) = String::from_utf8_lossy(ts_bytes).trim().parse::<i64>() else {
            continue;
        };

        for path_bytes in parts {
            let raw_path = String::from_utf8_lossy(path_bytes);
            let trimmed = raw_path.trim_start_matches(['\n', '\r']);
            if trimmed.is_empty() {
                continue;
            }
            let normalized = trimmed.replace('\\', "/");
            let rel_path = if prefix.is_empty() {
                normalized.as_str()
            } else if let Some(stripped) = normalized.strip_prefix(&prefix) {
                stripped
            } else {
                continue;
            };

            if rel_path.is_empty()
                || !Path::new(rel_path)
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("md"))
            {
                continue;
            }

            if seen.insert(rel_path.to_string()) {
                times.push((rel_path.to_string(), commit_ts));
            }
        }
    }

    times
}

/// Runs Git preflight, optional `--ff-only` pull, and `doc_git_time` ingestion before incremental sync.
///
/// Never returns an error: any Git failure emits a warning and falls back to plain incremental sync.
pub fn sync_git_pre_scan(vault: &Path, storage: &mut SqliteStorage, pull: bool) -> GitSyncOutcome {
    let mut outcome = GitSyncOutcome::default();

    let preflight = match git_preflight(vault) {
        Ok(pf) => pf,
        Err(reason) => {
            let msg = format!("git preflight failed ({reason}); falling back to plain sync");
            eprintln!("[k0maru] warning: {msg}");
            outcome.warnings.push(msg);
            return outcome;
        }
    };
    outcome.preflight_ok = true;

    if pull {
        let mut pull_allowed = true;

        match run_git_command(vault, &["symbolic-ref", "-q", "HEAD"]) {
            Ok(out) if out.status.success() => {}
            _ => {
                pull_allowed = false;
                let msg = "skipping --git-pull: HEAD is detached".to_string();
                eprintln!("[k0maru] warning: {msg}");
                outcome.warnings.push(msg);
            }
        }

        if pull_allowed {
            match run_git_command(vault, &["status", "--porcelain", "--untracked-files=no"]) {
                Ok(out)
                    if out.status.success()
                        && String::from_utf8_lossy(&out.stdout).trim().is_empty() => {}
                Ok(out) if out.status.success() => {
                    pull_allowed = false;
                    let msg = "skipping --git-pull: working tree has tracked changes".to_string();
                    eprintln!("[k0maru] warning: {msg}");
                    outcome.warnings.push(msg);
                }
                _ => {
                    pull_allowed = false;
                    let msg =
                        "skipping --git-pull: failed to check working tree status".to_string();
                    eprintln!("[k0maru] warning: {msg}");
                    outcome.warnings.push(msg);
                }
            }
        }

        if pull_allowed {
            match run_git_command(vault, &["rev-parse", "--abbrev-ref", "@{u}"]) {
                Ok(out) if out.status.success() => {}
                _ => {
                    pull_allowed = false;
                    let msg =
                        "skipping --git-pull: no upstream tracking branch configured".to_string();
                    eprintln!("[k0maru] warning: {msg}");
                    outcome.warnings.push(msg);
                }
            }
        }

        if pull_allowed {
            match run_git_command(vault, &["pull", "--ff-only", "--no-rebase"]) {
                Ok(out) if out.status.success() => {
                    outcome.pulled = true;
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    let msg = format!("git pull --ff-only failed ({stderr}); continuing sync");
                    eprintln!("[k0maru] warning: {msg}");
                    outcome.warnings.push(msg);
                }
                Err(err) => {
                    let msg = format!("git pull --ff-only failed ({err}); continuing sync");
                    eprintln!("[k0maru] warning: {msg}");
                    outcome.warnings.push(msg);
                }
            }
        }
    }

    let head = match run_git_command(vault, &["rev-parse", "--verify", "HEAD"]) {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => return outcome,
    };

    if head.is_empty() {
        return outcome;
    }

    if let Ok(Some(prev_head)) = storage.meta_get(META_GIT_TIME_HEAD) {
        if prev_head == head {
            return outcome;
        }
    }

    match run_git_command(
        vault,
        &[
            "log",
            "--no-renames",
            "--format=%x1e%ct",
            "--name-only",
            "-z",
            "--",
            ".",
        ],
    ) {
        Ok(out) if out.status.success() => {
            let times = parse_git_log_times(&out.stdout, &preflight.vault_rel);
            if let Err(err) = storage.set_git_times(&times, &head) {
                let msg = format!("failed to persist git commit times: {err}");
                eprintln!("[k0maru] warning: {msg}");
                outcome.warnings.push(msg);
            } else {
                outcome.git_times_updated = true;
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let msg = format!("git log failed ({stderr}); skipping git time fill");
            eprintln!("[k0maru] warning: {msg}");
            outcome.warnings.push(msg);
        }
        Err(err) => {
            let msg = format!("git log failed ({err}); skipping git time fill");
            eprintln!("[k0maru] warning: {msg}");
            outcome.warnings.push(msg);
        }
    }

    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords_from_modified_file() {
        let file = ModifiedFile::new("src/storage/sqlite.rs", FileChangeStatus::Modified);
        let keywords = file.extract_keywords();
        assert!(keywords.contains(&"storage".to_string()));
        assert!(keywords.contains(&"sqlite".to_string()));

        let file2 = ModifiedFile::new(
            "crates/k0maru-core/src/vector/embedding_engine.rs",
            FileChangeStatus::Added,
        );
        let kw2 = file2.extract_keywords();
        assert!(kw2.contains(&"k0maru-core".to_string()) || kw2.contains(&"vector".to_string()));
        assert!(kw2.contains(&"embedding".to_string()));
        assert!(kw2.contains(&"engine".to_string()));
    }

    #[test]
    fn test_parse_diff_stream_unified_diff() {
        let diff = r#"
diff --git a/src/storage/sqlite.rs b/src/storage/sqlite.rs
index abc..def 100644
--- a/src/storage/sqlite.rs
+++ b/src/storage/sqlite.rs
@@ -1,3 +1,4 @@
+use std::path::Path;
diff --git a/src/loadout/builder.rs b/src/loadout/builder.rs
index 111..222 100644
--- a/src/loadout/builder.rs
+++ b/src/loadout/builder.rs
@@ -10,3 +10,4 @@
"#;
        let files = parse_diff_stream(diff.as_bytes()).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, PathBuf::from("src/storage/sqlite.rs"));
        assert_eq!(files[1].path, PathBuf::from("src/loadout/builder.rs"));
    }

    #[test]
    fn test_parse_diff_stream_porcelain() {
        let status = " M src/lib.rs\n?? new_file.txt\nA  src/git/mod.rs\n";
        let files = parse_diff_stream(status.as_bytes()).unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].status, FileChangeStatus::Modified);
        assert_eq!(files[1].status, FileChangeStatus::Untracked);
        assert_eq!(files[2].status, FileChangeStatus::Added);
    }

    #[test]
    fn test_parse_diff_stream_name_status() {
        let name_status = "M\tsrc/storage/sqlite.rs\nA\tREADME.md\nR100\told.rs\tnew.rs\n";
        let files = parse_diff_stream(name_status.as_bytes()).unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].path, PathBuf::from("src/storage/sqlite.rs"));
        assert_eq!(files[1].path, PathBuf::from("README.md"));
        assert_eq!(files[2].path, PathBuf::from("new.rs"));
        assert_eq!(files[2].status, FileChangeStatus::Renamed);
    }
}
