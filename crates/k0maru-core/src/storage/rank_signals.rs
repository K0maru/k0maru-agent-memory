//! Tiered temporal decay and `doc_access` reinforcement signals for hybrid search ranking.

use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::params_from_iter;
use serde_json::Value;

use crate::core::dates::extract_reference_timestamp;
use crate::core::models::HierarchyLevel;
use crate::storage::SqliteStorage;

/// Number of seconds in one UTC day.
pub const SECONDS_PER_DAY: f32 = 86_400.0;

/// Default half-life in days for [`HierarchyLevel::L0Ephemeral`] decay.
pub const DEFAULT_L0_HALF_LIFE_DAYS: f32 = 60.0;
/// Default minimum decay floor for [`HierarchyLevel::L0Ephemeral`] documents.
pub const DEFAULT_L0_FLOOR: f32 = 0.50;
/// Default half-life in days for [`HierarchyLevel::L2Log`] decay.
pub const DEFAULT_L2_HALF_LIFE_DAYS: f32 = 180.0;
/// Default minimum decay floor for [`HierarchyLevel::L2Log`] documents.
pub const DEFAULT_L2_FLOOR: f32 = 0.70;

/// Maximum additive access reinforcement bonus (`0.15`, yielding `r in [1.0, 1.15]`).
pub const ACCESS_REINFORCEMENT_WEIGHT: f32 = 0.15;
/// Half-life in days for `doc_access` recency attenuation.
pub const ACCESS_RECENCY_HALF_LIFE_DAYS: f32 = 30.0;
/// Multiplier applied to documents whose frontmatter `stale_after` date has passed.
pub const STALE_AFTER_MULTIPLIER: f32 = 0.45;

/// Configuration for tiered exponential temporal decay in [`crate::storage::HybridSearchEngine`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecayConfig {
    /// Half-life in days for [`HierarchyLevel::L0Ephemeral`] documents.
    pub l0_half_life_days: f32,
    /// Asymptotic floor in `[0.0, 1.0]` for [`HierarchyLevel::L0Ephemeral`] documents.
    pub l0_floor: f32,
    /// Half-life in days for [`HierarchyLevel::L2Log`] documents.
    pub l2_half_life_days: f32,
    /// Asymptotic floor in `[0.0, 1.0]` for [`HierarchyLevel::L2Log`] documents.
    pub l2_floor: f32,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            l0_half_life_days: DEFAULT_L0_HALF_LIFE_DAYS,
            l0_floor: DEFAULT_L0_FLOOR,
            l2_half_life_days: DEFAULT_L2_HALF_LIFE_DAYS,
            l2_floor: DEFAULT_L2_FLOOR,
        }
    }
}

/// Returns the current Unix timestamp in seconds.
pub fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Returns the declared `stale_after` string if it parses to a valid ISO date
/// strictly before `now` (i.e. `now >= stale_unix + 86_400`).
pub fn expired_stale_after_label(frontmatter: &Value, now: i64) -> Option<String> {
    let raw = frontmatter
        .get("stale_after")
        .and_then(|v| v.as_str())?
        .trim();
    if raw.is_empty() {
        return None;
    }
    let stale_unix = crate::core::dates::parse_first_iso_date_unix(raw)?;
    if now >= stale_unix.saturating_add(SECONDS_PER_DAY as i64) {
        Some(raw.to_string())
    } else {
        None
    }
}

/// Computes the tiered temporal decay multiplier `d`.
///
/// [`HierarchyLevel::L1Resource`] and [`HierarchyLevel::L3Evergreen`] do not time-decay (`1.0`),
/// unless an explicit `stale_after` deadline in frontmatter has expired (`STALE_AFTER_MULTIPLIER`).
/// [`HierarchyLevel::L0Ephemeral`] and [`HierarchyLevel::L2Log`] decay toward their configured
/// floor from the first plausible reference timestamp across frontmatter (`date`, `created`),
/// file stem ISO date, `git_time`, and `mtime`, further scaled by [`STALE_AFTER_MULTIPLIER`] when expired.
pub fn compute_decay(
    hierarchy: HierarchyLevel,
    frontmatter: &Value,
    path: &str,
    git_time: Option<i64>,
    mtime: u64,
    now: i64,
    config: DecayConfig,
) -> f32 {
    let stale_mult = if expired_stale_after_label(frontmatter, now).is_some() {
        STALE_AFTER_MULTIPLIER
    } else {
        1.0
    };

    let (half_life_days, floor) = match hierarchy {
        HierarchyLevel::L1Resource | HierarchyLevel::L3Evergreen => return stale_mult,
        HierarchyLevel::L0Ephemeral => (config.l0_half_life_days, config.l0_floor),
        HierarchyLevel::L2Log => (config.l2_half_life_days, config.l2_floor),
    };

    let file_stem = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let Some(t_ref) = extract_reference_timestamp(frontmatter, file_stem, git_time, mtime, now)
    else {
        return stale_mult;
    };

    let age_secs = now.saturating_sub(t_ref);
    if age_secs <= 0 {
        return stale_mult;
    }

    let floor = floor.clamp(0.0, 1.0);
    if half_life_days <= 0.0 {
        return floor * stale_mult;
    }

    let age_days = age_secs as f32 / SECONDS_PER_DAY;
    (floor + (1.0 - floor) * 2.0f32.powf(-age_days / half_life_days)).clamp(floor, 1.0) * stale_mult
}

/// Computes the `doc_access` reinforcement multiplier `r` in `[1.0, 1.15]`.
///
/// Missing access records or zero counts yield `1.0`.
pub fn compute_access_reinforcement(stats: Option<(u32, i64)>, now: i64) -> f32 {
    let Some((count, last_access)) = stats else {
        return 1.0;
    };
    if count == 0 {
        return 1.0;
    }

    let freq_factor = 1.0f32.min((1.0 + count as f32).log2() / 4.0);
    let age_secs = now.saturating_sub(last_access).max(0) as f32;
    let recency_factor = 2.0f32.powf(-age_secs / (ACCESS_RECENCY_HALF_LIFE_DAYS * SECONDS_PER_DAY));
    (1.0 + ACCESS_REINFORCEMENT_WEIGHT * freq_factor * recency_factor).clamp(1.0, 1.15)
}

/// Batch-fetches `documents.id` values keyed by relative document path.
pub(crate) fn fetch_doc_ids_by_paths(
    storage: &SqliteStorage,
    paths: &[&str],
) -> Result<HashMap<String, i64>, Box<dyn std::error::Error>> {
    if paths.is_empty() {
        return Ok(HashMap::new());
    }

    let mut map = HashMap::with_capacity(paths.len());
    for chunk in paths.chunks(500) {
        let placeholders: Vec<String> = (1..=chunk.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "SELECT path, id FROM documents WHERE path IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = storage.connection().prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(chunk.iter().copied()), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (path, id) = row?;
            map.insert(path, id);
        }
    }
    Ok(map)
}

/// Batch-fetches relative document paths keyed by `documents.id`.
pub(crate) fn fetch_paths_by_doc_ids(
    storage: &SqliteStorage,
    doc_ids: &[i64],
) -> Result<HashMap<i64, String>, Box<dyn std::error::Error>> {
    if doc_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut map = HashMap::with_capacity(doc_ids.len());
    for chunk in doc_ids.chunks(500) {
        let placeholders: Vec<String> = (1..=chunk.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "SELECT id, path FROM documents WHERE id IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = storage.connection().prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(chunk.iter()), |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (id, path) = row?;
            map.insert(id, path);
        }
    }
    Ok(map)
}
