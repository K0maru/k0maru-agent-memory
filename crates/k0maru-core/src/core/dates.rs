//! Pure-Rust civil date parsing and reference timestamp extraction.
//!
//! Uses Howard Hinnant's `days_from_civil` algorithm with zero external date/time dependencies.

use serde_json::Value;

/// Minimum plausible Unix timestamp (2000-01-01T00:00:00Z).
pub const MIN_PLAUSIBLE_TIMESTAMP: i64 = 946_684_800;

const SECONDS_PER_DAY: i64 = 86_400;

/// Returns days since the Unix epoch (`1970-01-01`) for a proleptic Gregorian civil date.
///
/// Implements Howard Hinnant's `days_from_civil` algorithm.
pub fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = i64::from(year) - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let m = i64::from(month);
    let d = i64::from(day);
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Finds the first valid `YYYY-MM-DD` date anywhere in `s` and returns its midnight UTC Unix timestamp.
///
/// Handles raw dates (`"2026-09-29"`), WikiLink dates (`"[[2026-09-29]]"`,
/// `"[[00_Daily/2026/2026-11-18]]"`), and prefixed stems (`"2026-10-01-phase4-notes"`).
/// Validates calendar months and days including leap years.
pub fn parse_first_iso_date_unix(s: &str) -> Option<i64> {
    let bytes = s.as_bytes();
    if bytes.len() < 10 {
        return None;
    }

    for i in 0..=bytes.len() - 10 {
        if i > 0 && bytes[i - 1].is_ascii_digit() {
            continue;
        }
        if i + 10 < bytes.len() && bytes[i + 10].is_ascii_digit() {
            continue;
        }
        if bytes[i + 4] != b'-' || bytes[i + 7] != b'-' {
            continue;
        }
        let year_slice = &bytes[i..i + 4];
        let month_slice = &bytes[i + 5..i + 7];
        let day_slice = &bytes[i + 8..i + 10];
        if !year_slice.iter().all(u8::is_ascii_digit)
            || !month_slice.iter().all(u8::is_ascii_digit)
            || !day_slice.iter().all(u8::is_ascii_digit)
        {
            continue;
        }

        let year = parse_digits_i32(year_slice);
        let month = parse_digits_u32(month_slice);
        let day = parse_digits_u32(day_slice);

        if is_valid_civil_date(year, month, day) {
            return Some(days_from_civil(year, month, day) * SECONDS_PER_DAY);
        }
    }

    None
}

/// Returns `true` when `ts` falls within the plausible document timestamp window
/// (`[2000-01-01T00:00:00Z, now + 86400s]`).
pub fn is_plausible_timestamp(ts: i64, now: i64) -> bool {
    ts >= MIN_PLAUSIBLE_TIMESTAMP && ts <= now.saturating_add(SECONDS_PER_DAY)
}

/// Extracts the best plausible reference Unix timestamp for decay scoring.
///
/// Priority order:
/// 1. `frontmatter["date"]`
/// 2. `frontmatter["created"]`
/// 3. First valid ISO date in `file_stem`
/// 4. `git_time`
/// 5. `mtime`
///
/// Each candidate is checked with [`is_plausible_timestamp`] so sentinel values
/// (`0`, `1000`, `u64::MAX`, `-1`) are rejected and fall through.
pub fn extract_reference_timestamp(
    frontmatter: &Value,
    file_stem: &str,
    git_time: Option<i64>,
    mtime: u64,
    now: i64,
) -> Option<i64> {
    if let Some(ts) = frontmatter
        .get("date")
        .and_then(parse_frontmatter_timestamp)
        .filter(|&ts| is_plausible_timestamp(ts, now))
    {
        return Some(ts);
    }

    if let Some(ts) = frontmatter
        .get("created")
        .and_then(parse_frontmatter_timestamp)
        .filter(|&ts| is_plausible_timestamp(ts, now))
    {
        return Some(ts);
    }

    if let Some(ts) =
        parse_first_iso_date_unix(file_stem).filter(|&ts| is_plausible_timestamp(ts, now))
    {
        return Some(ts);
    }

    if let Some(ts) = git_time.filter(|&ts| is_plausible_timestamp(ts, now)) {
        return Some(ts);
    }

    i64::try_from(mtime)
        .ok()
        .filter(|&ts| is_plausible_timestamp(ts, now))
}

fn parse_frontmatter_timestamp(value: &Value) -> Option<i64> {
    match value {
        Value::String(s) => parse_first_iso_date_unix(s),
        Value::Number(n) => n.as_i64(),
        _ => None,
    }
}

fn parse_digits_i32(digits: &[u8]) -> i32 {
    digits
        .iter()
        .fold(0i32, |acc, &b| acc * 10 + i32::from(b - b'0'))
}

fn parse_digits_u32(digits: &[u8]) -> u32 {
    digits
        .iter()
        .fold(0u32, |acc, &b| acc * 10 + u32::from(b - b'0'))
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn is_valid_civil_date(year: i32, month: u32, day: u32) -> bool {
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    day >= 1 && day <= max_day
}
