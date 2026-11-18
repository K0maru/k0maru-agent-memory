//! Zero-LLM Atomic Typed Session Handoff Engine.
//!
//! Stores structured session handoffs outside the disposable SQLite cache at
//! `<vault_root>/.k0maru/handoffs/{pending,claimed}/<id>.json`. Claims use an
//! atomic filesystem `rename(pending/<id>.json, claimed/<id>.json)` primitive so
//! concurrent threads or processes cannot double-claim the same handoff.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, SecondsFormat, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64;

/// Schema identifier written into every `HandoffRecord`.
pub const HANDOFF_SCHEMA_VERSION: &str = "k0maru.handoff/v1";

/// Maximum serialized byte size allowed for a single handoff payload (64 KiB).
pub const MAX_HANDOFF_BYTES: usize = 65_536;

const GIT_TIMEOUT: Duration = Duration::from_secs(5);

static HANDOFF_ID_COUNTER: AtomicU64 = AtomicU64::new(0);
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Workflow state of a handoff record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffState {
    #[default]
    InProgress,
    Blocked,
    Done,
}

impl HandoffState {
    /// Returns the canonical snake-case string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Done => "done",
        }
    }
}

impl fmt::Display for HandoffState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for HandoffState {
    type Err = HandoffError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "in_progress" | "in-progress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "done" => Ok(Self::Done),
            other => Err(HandoffError::InvalidInput(format!(
                "invalid handoff state '{}'; expected in_progress, blocked, or done",
                other
            ))),
        }
    }
}

/// Vault-relative file reference attached to a handoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffFileRef {
    pub path: String,
    #[serde(default)]
    pub lines: Option<String>,
}

fn default_schema() -> String {
    HANDOFF_SCHEMA_VERSION.to_string()
}

/// Typed session handoff record (`k0maru.handoff/v1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffRecord {
    #[serde(default = "default_schema")]
    pub schema: String,
    pub id: String,
    pub created_at: String,
    pub from_agent: String,
    #[serde(default)]
    pub to_agent: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    pub title: String,
    #[serde(default)]
    pub goal: String,
    #[serde(default)]
    pub state: HandoffState,
    #[serde(default)]
    pub done: Vec<String>,
    #[serde(default)]
    pub next_steps: Vec<String>,
    #[serde(default)]
    pub open_questions: Vec<String>,
    #[serde(default)]
    pub refs: Vec<String>,
    #[serde(default)]
    pub files: Vec<HandoffFileRef>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub claimed_by: Option<String>,
    #[serde(default)]
    pub claimed_at: Option<String>,
}

/// Storage lifecycle status derived from directory placement and `expires_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffStatus {
    Pending,
    Claimed,
    Expired,
}

impl HandoffStatus {
    /// Returns the canonical status label.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
            Self::Expired => "expired",
        }
    }
}

impl fmt::Display for HandoffStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Handoff record paired with its derived store status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffEntry {
    pub status: HandoffStatus,
    #[serde(flatten)]
    pub record: HandoffRecord,
}

impl HandoffEntry {
    /// Renders this entry as concise human/agent-readable Markdown.
    pub fn to_markdown(&self) -> String {
        format_handoff_markdown(self)
    }
}

/// Parameters for creating a new pending handoff record.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateHandoffRequest {
    pub from_agent: String,
    pub to_agent: Option<String>,
    pub project: Option<String>,
    pub title: String,
    pub goal: String,
    pub state: HandoffState,
    pub done: Vec<String>,
    pub next_steps: Vec<String>,
    pub open_questions: Vec<String>,
    pub refs: Vec<String>,
    pub files: Vec<HandoffFileRef>,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub expires_at: Option<String>,
    /// Optional repository path for read-only git branch/commit detection.
    /// Defaults to `vault_root` when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_path: Option<PathBuf>,
}

/// Parameters for atomically claiming a pending handoff record.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaimHandoffRequest {
    pub agent: String,
    pub id: Option<String>,
    pub project: Option<String>,
    pub latest: bool,
}

/// Summary of a `prune` operation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PruneResult {
    pub deleted_claimed: usize,
    pub deleted_expired: usize,
    pub kept_pending: usize,
    pub kept_claimed: usize,
    pub deleted_ids: Vec<String>,
}

/// Typed errors returned by `HandoffStore` operations.
#[derive(Debug)]
pub enum HandoffError {
    InvalidInput(String),
    PayloadTooLarge {
        size: usize,
        max: usize,
    },
    AlreadyClaimedOrMissing(String),
    Expired(String),
    TargetAgentMismatch {
        id: String,
        expected: String,
        actual: String,
    },
    CorruptRecord {
        path: PathBuf,
        reason: String,
    },
    Io(std::io::Error),
}

impl HandoffError {
    /// Maps the error to a deterministic CLI process exit code.
    ///
    /// - `3`: missing, already claimed, expired, or target-agent mismatch
    /// - `2`: invalid caller input or oversized payload
    /// - `1`: I/O or corrupt record error
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::AlreadyClaimedOrMissing(_)
            | Self::Expired(_)
            | Self::TargetAgentMismatch { .. } => 3,
            Self::InvalidInput(_) | Self::PayloadTooLarge { .. } => 2,
            Self::CorruptRecord { .. } | Self::Io(_) => 1,
        }
    }
}

impl fmt::Display for HandoffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(msg) => write!(f, "Invalid handoff input: {}", msg),
            Self::PayloadTooLarge { size, max } => write!(
                f,
                "Handoff payload too large: {} bytes exceeds maximum {} bytes",
                size, max
            ),
            Self::AlreadyClaimedOrMissing(id) => {
                write!(f, "Handoff already claimed or missing: {}", id)
            }
            Self::Expired(id) => write!(f, "Handoff has expired: {}", id),
            Self::TargetAgentMismatch {
                id,
                expected,
                actual,
            } => write!(
                f,
                "Handoff {} is targeted at '{}', not '{}'",
                id, expected, actual
            ),
            Self::CorruptRecord { path, reason } => write!(
                f,
                "Corrupt handoff record at {}: {}",
                path.display(),
                reason
            ),
            Self::Io(err) => write!(f, "Handoff I/O error: {}", err),
        }
    }
}

impl std::error::Error for HandoffError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for HandoffError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// Filesystem-backed session handoff store anchored at `<vault_root>/.k0maru/handoffs`.
#[derive(Debug, Clone)]
pub struct HandoffStore {
    vault_root: PathBuf,
}

impl HandoffStore {
    /// Creates a store targeting `<vault_root>/.k0maru/handoffs`.
    ///
    /// Does not touch the filesystem until a write operation (`create` / `claim`) runs.
    pub fn new(vault_root: impl Into<PathBuf>) -> Self {
        Self {
            vault_root: vault_root.into(),
        }
    }

    /// Returns the vault root directory.
    pub fn vault_root(&self) -> &Path {
        &self.vault_root
    }

    /// Returns `<vault_root>/.k0maru/handoffs`.
    pub fn handoffs_dir(&self) -> PathBuf {
        self.vault_root.join(".k0maru").join("handoffs")
    }

    /// Returns `<vault_root>/.k0maru/handoffs/pending`.
    pub fn pending_dir(&self) -> PathBuf {
        self.handoffs_dir().join("pending")
    }

    /// Returns `<vault_root>/.k0maru/handoffs/claimed`.
    pub fn claimed_dir(&self) -> PathBuf {
        self.handoffs_dir().join("claimed")
    }

    fn ensure_dirs(&self) -> Result<(), HandoffError> {
        fs::create_dir_all(self.pending_dir())?;
        fs::create_dir_all(self.claimed_dir())?;
        Ok(())
    }

    /// Validates, populates, and atomically writes a new pending handoff record.
    pub fn create(&self, req: CreateHandoffRequest) -> Result<HandoffRecord, HandoffError> {
        let from_agent = req.from_agent.trim().to_string();
        if from_agent.is_empty() {
            return Err(HandoffError::InvalidInput(
                "from_agent must not be empty".to_string(),
            ));
        }

        let title = req.title.trim().to_string();
        if title.is_empty() {
            return Err(HandoffError::InvalidInput(
                "title must not be empty".to_string(),
            ));
        }

        for r in &req.refs {
            let path_part = extract_ref_path_portion(r);
            validate_relative_no_traversal(path_part, "refs")?;
        }

        for f in &req.files {
            validate_relative_no_traversal(&f.path, "files")?;
        }

        let expires_at = match req.expires_at {
            Some(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    let parsed = DateTime::parse_from_rfc3339(trimmed).map_err(|e| {
                        HandoffError::InvalidInput(format!(
                            "invalid RFC3339 expires_at '{}': {}",
                            trimmed, e
                        ))
                    })?;
                    Some(format_rfc3339_utc(parsed.with_timezone(&Utc)))
                }
            }
            None => None,
        };

        let to_agent = normalize_optional(req.to_agent);
        let project = normalize_optional(req.project);
        let mut branch = normalize_optional(req.branch);
        let mut commit = normalize_optional(req.commit);

        if branch.is_none() || commit.is_none() {
            let git_dir = req.repo_path.as_deref().unwrap_or(&self.vault_root);
            let (detected_branch, detected_commit) = detect_git_head(git_dir);
            if branch.is_none() {
                branch = detected_branch;
            }
            if commit.is_none() {
                commit = detected_commit;
            }
        }

        let now = Utc::now();
        let id = generate_handoff_id(now);
        let record = HandoffRecord {
            schema: HANDOFF_SCHEMA_VERSION.to_string(),
            id: id.clone(),
            created_at: format_rfc3339_utc(now),
            from_agent,
            to_agent,
            project,
            title,
            goal: req.goal,
            state: req.state,
            done: req.done,
            next_steps: req.next_steps,
            open_questions: req.open_questions,
            refs: req.refs,
            files: req.files,
            branch,
            commit,
            expires_at,
            claimed_by: None,
            claimed_at: None,
        };

        let bytes = serde_json::to_vec_pretty(&record).map_err(|e| {
            HandoffError::InvalidInput(format!("failed to serialize handoff record: {}", e))
        })?;
        if bytes.len() > MAX_HANDOFF_BYTES {
            return Err(HandoffError::PayloadTooLarge {
                size: bytes.len(),
                max: MAX_HANDOFF_BYTES,
            });
        }

        self.ensure_dirs()?;
        let pending_path = self.pending_dir().join(format!("{}.json", id));
        atomic_write_json(&self.handoffs_dir(), &pending_path, &bytes)?;
        Ok(record)
    }

    /// Atomically claims a matching unexpired pending handoff for `req.agent`.
    pub fn claim(&self, req: ClaimHandoffRequest) -> Result<HandoffRecord, HandoffError> {
        let agent = req.agent.trim().to_string();
        if agent.is_empty() {
            return Err(HandoffError::InvalidInput(
                "agent must not be empty".to_string(),
            ));
        }

        self.ensure_dirs()?;
        let now = Utc::now();

        if let Some(raw_id) = req.id.as_deref() {
            let id = validate_handoff_id(raw_id)?;
            let pending_path = self.pending_dir().join(format!("{}.json", id));
            let claimed_path = self.claimed_dir().join(format!("{}.json", id));

            let mut record = match read_record_file(&pending_path) {
                Ok(Some(rec)) => rec,
                Ok(None) => return Err(HandoffError::AlreadyClaimedOrMissing(id)),
                Err(err) => return Err(err),
            };

            if is_expired(&record, now) {
                return Err(HandoffError::Expired(id));
            }
            if !is_agent_eligible(&record, &agent) {
                return Err(HandoffError::TargetAgentMismatch {
                    id,
                    expected: record.to_agent.unwrap_or_default(),
                    actual: agent,
                });
            }

            match fs::rename(&pending_path, &claimed_path) {
                Ok(()) => {}
                Err(err)
                    if matches!(err.kind(), ErrorKind::NotFound | ErrorKind::AlreadyExists) =>
                {
                    return Err(HandoffError::AlreadyClaimedOrMissing(id));
                }
                Err(err) => return Err(HandoffError::Io(err)),
            }

            record.claimed_by = Some(agent);
            record.claimed_at = Some(format_rfc3339_utc(now));
            let bytes = serialize_record(&record)?;
            atomic_write_json(&self.handoffs_dir(), &claimed_path, &bytes)?;
            return Ok(record);
        }

        let project_filter = normalize_optional(req.project);
        let mut candidates = read_dir_records(&self.pending_dir())?;
        candidates.retain(|rec| {
            !is_expired(rec, now)
                && is_agent_eligible(rec, &agent)
                && matches_project(rec, project_filter.as_deref())
        });
        sort_records_newest_first(&mut candidates);

        for mut record in candidates {
            let pending_path = self.pending_dir().join(format!("{}.json", record.id));
            let claimed_path = self.claimed_dir().join(format!("{}.json", record.id));

            match fs::rename(&pending_path, &claimed_path) {
                Ok(()) => {
                    record.claimed_by = Some(agent);
                    record.claimed_at = Some(format_rfc3339_utc(Utc::now()));
                    let bytes = serialize_record(&record)?;
                    atomic_write_json(&self.handoffs_dir(), &claimed_path, &bytes)?;
                    return Ok(record);
                }
                Err(err)
                    if matches!(err.kind(), ErrorKind::NotFound | ErrorKind::AlreadyExists) =>
                {
                    // Another concurrent worker claimed this candidate; try the next one.
                    continue;
                }
                Err(err) => return Err(HandoffError::Io(err)),
            }
        }

        Err(HandoffError::AlreadyClaimedOrMissing(
            "no matching unexpired pending handoff".to_string(),
        ))
    }

    /// Shows a specific handoff by ID (in `pending/` or `claimed/`) or the latest matching record.
    pub fn show(
        &self,
        id_or_latest: Option<&str>,
        project_filter: Option<&str>,
    ) -> Result<HandoffEntry, HandoffError> {
        let now = Utc::now();
        let project_filter = project_filter.map(str::trim).filter(|s| !s.is_empty());

        if let Some(raw_id) = id_or_latest {
            let trimmed = raw_id.trim();
            if trimmed.is_empty() {
                return Err(HandoffError::InvalidInput(
                    "handoff id must not be empty".to_string(),
                ));
            }
            if !trimmed.eq_ignore_ascii_case("latest") {
                let id = validate_handoff_id(trimmed)?;
                let pending_path = self.pending_dir().join(format!("{}.json", id));
                if let Some(record) = read_record_file(&pending_path)? {
                    let status = if is_expired(&record, now) {
                        HandoffStatus::Expired
                    } else {
                        HandoffStatus::Pending
                    };
                    return Ok(HandoffEntry { status, record });
                }

                let claimed_path = self.claimed_dir().join(format!("{}.json", id));
                if let Some(record) = read_record_file(&claimed_path)? {
                    return Ok(HandoffEntry {
                        status: HandoffStatus::Claimed,
                        record,
                    });
                }

                return Err(HandoffError::AlreadyClaimedOrMissing(id));
            }
        }

        let mut all_entries = self.list(project_filter, true)?;
        if all_entries.is_empty() {
            return Err(HandoffError::AlreadyClaimedOrMissing(
                "no matching handoff found".to_string(),
            ));
        }

        // Prefer newest unexpired pending record; otherwise fall back to newest claimed/expired.
        if let Some(idx) = all_entries
            .iter()
            .position(|entry| entry.status == HandoffStatus::Pending)
        {
            return Ok(all_entries.swap_remove(idx));
        }

        Ok(all_entries.swap_remove(0))
    }

    /// Lists handoffs matching `project_filter`, newest first.
    ///
    /// When `include_claimed` is `false`, only unexpired pending records are returned.
    /// When `include_claimed` is `true`, pending, expired, and claimed records are all returned.
    pub fn list(
        &self,
        project_filter: Option<&str>,
        include_claimed: bool,
    ) -> Result<Vec<HandoffEntry>, HandoffError> {
        let now = Utc::now();
        let project_filter = project_filter.map(str::trim).filter(|s| !s.is_empty());
        let mut entries = Vec::new();

        for record in read_dir_records(&self.pending_dir())? {
            if !matches_project(&record, project_filter) {
                continue;
            }
            let expired = is_expired(&record, now);
            if expired && !include_claimed {
                continue;
            }
            entries.push(HandoffEntry {
                status: if expired {
                    HandoffStatus::Expired
                } else {
                    HandoffStatus::Pending
                },
                record,
            });
        }

        if include_claimed {
            for record in read_dir_records(&self.claimed_dir())? {
                if !matches_project(&record, project_filter) {
                    continue;
                }
                entries.push(HandoffEntry {
                    status: HandoffStatus::Claimed,
                    record,
                });
            }
        }

        sort_entries_newest_first(&mut entries);
        Ok(entries)
    }

    /// Returns a concise Markdown hint for the newest unexpired pending handoff
    /// without claiming or modifying any files (used by `hook session-start`).
    pub fn latest_pending_hint(
        &self,
        project_filter: Option<&str>,
    ) -> Result<Option<String>, HandoffError> {
        let now = Utc::now();
        let project_filter = project_filter.map(str::trim).filter(|s| !s.is_empty());
        let mut candidates = read_dir_records(&self.pending_dir())?;
        candidates.retain(|rec| !is_expired(rec, now) && matches_project(rec, project_filter));
        sort_records_newest_first(&mut candidates);

        Ok(candidates.first().map(|rec| {
            format!(
                "## Pending Handoff\n- {}: {} (from {})",
                rec.id, rec.title, rec.from_agent
            )
        }))
    }

    /// Deletes claimed or expired handoffs older than `older_than_secs`.
    ///
    /// Unexpired pending handoffs are never deleted.
    pub fn prune(&self, older_than_secs: u64) -> Result<PruneResult, HandoffError> {
        self.prune_with_now(older_than_secs, Utc::now())
    }

    /// Deterministic clock variant of [`Self::prune`] for testing.
    pub fn prune_with_now(
        &self,
        older_than_secs: u64,
        now: DateTime<Utc>,
    ) -> Result<PruneResult, HandoffError> {
        let clamped_secs = older_than_secs.min(i64::MAX as u64) as i64;
        let cutoff = now - TimeDelta::seconds(clamped_secs);
        let mut result = PruneResult::default();

        for (path, record) in read_dir_records_with_paths(&self.pending_dir())? {
            if let Some(exp_dt) = parse_expires_at(&record) {
                if exp_dt <= now {
                    if exp_dt <= cutoff {
                        remove_if_exists(&path)?;
                        result.deleted_expired += 1;
                        result.deleted_ids.push(record.id);
                    } else {
                        result.kept_pending += 1;
                    }
                    continue;
                }
            }
            // Unexpired pending handoffs must never be pruned.
            result.kept_pending += 1;
        }

        for (path, record) in read_dir_records_with_paths(&self.claimed_dir())? {
            let record_ts = claimed_timestamp(&record, &path);
            if record_ts <= cutoff {
                remove_if_exists(&path)?;
                result.deleted_claimed += 1;
                result.deleted_ids.push(record.id);
            } else {
                result.kept_claimed += 1;
            }
        }

        Ok(result)
    }
}

/// Formats a single [`HandoffEntry`] as Markdown, gracefully handling incomplete
/// crash-recovery records in `claimed/` where `claimed_by` is `None`.
pub fn format_handoff_markdown(entry: &HandoffEntry) -> String {
    let r = &entry.record;
    let status_line = match entry.status {
        HandoffStatus::Pending => "pending".to_string(),
        HandoffStatus::Expired => match &r.expires_at {
            Some(exp) => format!("expired (at {})", exp),
            None => "expired".to_string(),
        },
        HandoffStatus::Claimed => match (&r.claimed_by, &r.claimed_at) {
            (Some(by), Some(at)) => format!("claimed by {} at {}", by, at),
            (Some(by), None) => format!("claimed by {}", by),
            (None, _) => "claimed (claimer unknown)".to_string(),
        },
    };

    let mut out = String::new();
    out.push_str(&format!("## Handoff: {} ({})\n", r.title, r.id));
    out.push_str(&format!("- **State**: {}\n", r.state));
    out.push_str(&format!("- **Status**: {}\n", status_line));
    if let Some(to) = &r.to_agent {
        out.push_str(&format!("- **From**: {} -> **To**: {}\n", r.from_agent, to));
    } else {
        out.push_str(&format!("- **From**: {}\n", r.from_agent));
    }
    out.push_str(&format!("- **Created**: {}\n", r.created_at));
    if let Some(proj) = &r.project {
        out.push_str(&format!("- **Project**: {}\n", proj));
    }
    match (&r.branch, &r.commit) {
        (Some(b), Some(c)) => out.push_str(&format!("- **Git**: {} @ {}\n", b, c)),
        (Some(b), None) => out.push_str(&format!("- **Git**: {}\n", b)),
        (None, Some(c)) => out.push_str(&format!("- **Git**: {}\n", c)),
        (None, None) => {}
    }
    if let Some(exp) = &r.expires_at {
        out.push_str(&format!("- **Expires**: {}\n", exp));
    }
    if !r.goal.trim().is_empty() {
        out.push_str(&format!("\n**Goal**: {}\n", r.goal.trim()));
    }

    append_bullet_section(&mut out, "Done", &r.done);
    append_bullet_section(&mut out, "Next Steps", &r.next_steps);
    append_bullet_section(&mut out, "Open Questions", &r.open_questions);
    append_bullet_section(&mut out, "Refs", &r.refs);

    if !r.files.is_empty() {
        out.push_str("\n### Files\n");
        for file in &r.files {
            if let Some(lines) = &file.lines {
                out.push_str(&format!("- {}:{}\n", file.path, lines));
            } else {
                out.push_str(&format!("- {}\n", file.path));
            }
        }
    }

    out
}

/// Formats a list of [`HandoffEntry`] records as a concise Markdown list.
pub fn format_handoff_list_markdown(entries: &[HandoffEntry]) -> String {
    if entries.is_empty() {
        return "No handoffs found.\n".to_string();
    }

    let mut out = String::from("## Handoffs\n");
    for entry in entries {
        let r = &entry.record;
        let status_tag = match entry.status {
            HandoffStatus::Pending => "pending".to_string(),
            HandoffStatus::Expired => "expired".to_string(),
            HandoffStatus::Claimed => match &r.claimed_by {
                Some(by) => format!("claimed by {}", by),
                None => "claimed (claimer unknown)".to_string(),
            },
        };
        let project_tag = r
            .project
            .as_deref()
            .map(|p| format!(" [{}]", p))
            .unwrap_or_default();
        out.push_str(&format!(
            "- `{}` [{} | {}]{} {} (from {})\n",
            r.id, r.state, status_tag, project_tag, r.title, r.from_agent
        ));
    }
    out
}

/// Parses a human-friendly duration string such as `14d`, `24h`, `30m`, `60s`, or `2w`
/// into seconds. Plain integers are interpreted as seconds.
pub fn parse_duration_secs(spec: &str) -> Result<u64, HandoffError> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(HandoffError::InvalidInput(
            "duration must not be empty".to_string(),
        ));
    }

    let (num_part, multiplier) = match trimmed.as_bytes().last().copied() {
        Some(b's' | b'S') => (&trimmed[..trimmed.len() - 1], 1u64),
        Some(b'm' | b'M') => (&trimmed[..trimmed.len() - 1], 60u64),
        Some(b'h' | b'H') => (&trimmed[..trimmed.len() - 1], 3_600u64),
        Some(b'd' | b'D') => (&trimmed[..trimmed.len() - 1], 86_400u64),
        Some(b'w' | b'W') => (&trimmed[..trimmed.len() - 1], 604_800u64),
        _ => (trimmed, 1u64),
    };

    let value: u64 = num_part.trim().parse().map_err(|_| {
        HandoffError::InvalidInput(format!(
            "invalid duration '{}'; expected <n>[s|m|h|d|w]",
            trimmed
        ))
    })?;

    value.checked_mul(multiplier).ok_or_else(|| {
        HandoffError::InvalidInput(format!("duration '{}' overflows u64 seconds", trimmed))
    })
}

fn append_bullet_section(out: &mut String, heading: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    out.push_str(&format!("\n### {}\n", heading));
    for item in items {
        out.push_str(&format!("- {}\n", item));
    }
}

fn generate_handoff_id(now: DateTime<Utc>) -> String {
    let pid = std::process::id().to_le_bytes();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_le_bytes();
    let seq = HANDOFF_ID_COUNTER
        .fetch_add(1, Ordering::Relaxed)
        .to_le_bytes();

    let mut input = [0u8; 28];
    input[..4].copy_from_slice(&pid);
    input[4..20].copy_from_slice(&nanos);
    input[20..28].copy_from_slice(&seq);

    let digest = xxh3_64(&input);
    let suffix = (digest & 0xffff_ffff) as u32;
    format!("{}-{:08x}", now.format("%Y%m%dT%H%M%SZ"), suffix)
}

fn format_rfc3339_utc(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn normalize_optional(opt: Option<String>) -> Option<String> {
    opt.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn validate_handoff_id(raw: &str) -> Result<String, HandoffError> {
    let id = raw.trim();
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(HandoffError::InvalidInput(format!(
            "invalid handoff id '{}': only ASCII alphanumeric, '-', and '_' are allowed",
            raw
        )));
    }
    Ok(id.to_string())
}

fn extract_ref_path_portion(raw_ref: &str) -> &str {
    let trimmed = raw_ref.trim();
    let lower = trimmed.to_ascii_lowercase();
    let bytes = lower.as_bytes();

    // Match the last `.md` followed by end-of-string, `:`, or `#`, mirroring locator syntax.
    let mut last_md_end: Option<usize> = None;
    let mut search_from = 0usize;
    while let Some(rel_idx) = lower[search_from..].find(".md") {
        let abs_idx = search_from + rel_idx;
        let after = abs_idx + 3;
        if after == bytes.len() || matches!(bytes[after], b':' | b'#') {
            last_md_end = Some(after);
        }
        search_from = abs_idx + 1;
    }

    if let Some(end) = last_md_end {
        return &trimmed[..end];
    }

    let before_fragment = trimmed.split('#').next().unwrap_or(trimmed);
    if let Some(colon_idx) = before_fragment.rfind(':') {
        let suffix = &before_fragment[colon_idx + 1..];
        if !suffix.is_empty()
            && suffix.bytes().all(|b| b.is_ascii_digit() || b == b'-')
            && suffix.bytes().any(|b| b.is_ascii_digit())
        {
            return &before_fragment[..colon_idx];
        }
    }

    before_fragment
}

fn validate_relative_no_traversal(path_str: &str, field: &str) -> Result<(), HandoffError> {
    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        return Err(HandoffError::InvalidInput(format!(
            "{} entry must not be empty",
            field
        )));
    }
    if trimmed.contains('\0') {
        return Err(HandoffError::InvalidInput(format!(
            "{} entry '{}' contains NUL byte",
            field, path_str
        )));
    }
    if trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return Err(HandoffError::InvalidInput(format!(
            "{} entry '{}' must be relative, not absolute",
            field, path_str
        )));
    }

    let bytes = trimmed.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Err(HandoffError::InvalidInput(format!(
            "{} entry '{}' must not include a drive prefix",
            field, path_str
        )));
    }

    let normalized = trimmed.replace('\\', "/");
    for seg in normalized.split('/') {
        if seg == ".." {
            return Err(HandoffError::InvalidInput(format!(
                "{} entry '{}' must not contain '..' traversal components",
                field, path_str
            )));
        }
    }

    for comp in Path::new(&normalized).components() {
        if matches!(
            comp,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return Err(HandoffError::InvalidInput(format!(
                "{} entry '{}' must be relative with no traversal components",
                field, path_str
            )));
        }
    }

    Ok(())
}

fn detect_git_head(repo_dir: &Path) -> (Option<String>, Option<String>) {
    if !repo_dir.exists() {
        return (None, None);
    }

    let inside = run_git_readonly(repo_dir, &["rev-parse", "--is-inside-work-tree"]);
    if inside.as_deref() != Some("true") {
        return (None, None);
    }

    let branch = run_git_readonly(repo_dir, &["rev-parse", "--abbrev-ref", "HEAD"])
        .filter(|b| !b.is_empty() && b != "HEAD");
    let commit = run_git_readonly(repo_dir, &["rev-parse", "HEAD"]).filter(|c| !c.is_empty());

    (branch, commit)
}

fn run_git_readonly(repo_dir: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo_dir)
        .args(["-c", "core.quotepath=off", "--no-optional-locks"])
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().ok()?;
    let deadline = Instant::now() + GIT_TIMEOUT;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let output = child.wait_with_output().ok()?;
                let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return if text.is_empty() { None } else { Some(text) };
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }
}

fn serialize_record(record: &HandoffRecord) -> Result<Vec<u8>, HandoffError> {
    serde_json::to_vec_pretty(record).map_err(|e| {
        HandoffError::InvalidInput(format!("failed to serialize handoff record: {}", e))
    })
}

fn atomic_write_json(handoffs_root: &Path, dest: &Path, bytes: &[u8]) -> Result<(), HandoffError> {
    let pid = std::process::id();
    let seq = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_path = handoffs_root.join(format!(".tmp-handoff-{}-{}.json", pid, seq));

    let write_result = (|| -> Result<(), std::io::Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp_path, dest)?;
        Ok(())
    })();

    if let Err(err) = write_result {
        let _ = fs::remove_file(&tmp_path);
        return Err(HandoffError::Io(err));
    }

    Ok(())
}

fn read_record_file(path: &Path) -> Result<Option<HandoffRecord>, HandoffError> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(HandoffError::Io(err)),
    };

    let record: HandoffRecord =
        serde_json::from_slice(&bytes).map_err(|e| HandoffError::CorruptRecord {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
    Ok(Some(record))
}

fn read_dir_records(dir: &Path) -> Result<Vec<HandoffRecord>, HandoffError> {
    Ok(read_dir_records_with_paths(dir)?
        .into_iter()
        .map(|(_, record)| record)
        .collect())
}

fn read_dir_records_with_paths(dir: &Path) -> Result<Vec<(PathBuf, HandoffRecord)>, HandoffError> {
    let entries = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(HandoffError::Io(err)),
    };

    let mut records = Vec::new();
    for entry_res in entries {
        let entry = match entry_res {
            Ok(e) => e,
            Err(err) if err.kind() == ErrorKind::NotFound => continue,
            Err(err) => return Err(HandoffError::Io(err)),
        };

        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.starts_with('.') || !name.ends_with(".json") {
            continue;
        }

        if let Some(record) = read_record_file(&path)? {
            records.push((path, record));
        }
    }

    Ok(records)
}

fn remove_if_exists(path: &Path) -> Result<(), HandoffError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(HandoffError::Io(err)),
    }
}

fn parse_rfc3339(ts: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts.trim())
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn parse_expires_at(record: &HandoffRecord) -> Option<DateTime<Utc>> {
    record.expires_at.as_deref().and_then(parse_rfc3339)
}

fn is_expired(record: &HandoffRecord, now: DateTime<Utc>) -> bool {
    parse_expires_at(record).is_some_and(|exp| exp <= now)
}

fn is_agent_eligible(record: &HandoffRecord, agent: &str) -> bool {
    match record.to_agent.as_deref() {
        None => true,
        Some(target) => target.trim().eq_ignore_ascii_case(agent.trim()),
    }
}

fn matches_project(record: &HandoffRecord, project_filter: Option<&str>) -> bool {
    match project_filter {
        None => true,
        Some(filter) => record
            .project
            .as_deref()
            .is_some_and(|p| p.trim().eq_ignore_ascii_case(filter)),
    }
}

fn claimed_timestamp(record: &HandoffRecord, path: &Path) -> DateTime<Utc> {
    if let Some(ts) = record.claimed_at.as_deref().and_then(parse_rfc3339) {
        return ts;
    }
    if let Some(ts) = parse_rfc3339(&record.created_at) {
        return ts;
    }
    fs::metadata(path)
        .and_then(|m| m.modified())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now())
}

fn compare_records_newest_first(a: &HandoffRecord, b: &HandoffRecord) -> std::cmp::Ordering {
    let ta = parse_rfc3339(&a.created_at);
    let tb = parse_rfc3339(&b.created_at);
    tb.cmp(&ta)
        .then_with(|| b.created_at.cmp(&a.created_at))
        .then_with(|| b.id.cmp(&a.id))
}

fn sort_records_newest_first(records: &mut [HandoffRecord]) {
    records.sort_by(compare_records_newest_first);
}

fn sort_entries_newest_first(entries: &mut [HandoffEntry]) {
    entries.sort_by(|a, b| compare_records_newest_first(&a.record, &b.record));
}
