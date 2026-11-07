// SPDX-License-Identifier: GPL-3.0-or-later
//! Recursive folder search: name / glob plus type · size · date filters (AIA-234).

use super::list::{display_path, entry_is_hidden, system_time_ms, NavigatorEntry};
use super::path_guard::assert_os_path;
use serde::Serialize;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

const MAX_RESULTS: usize = 400;
const MAX_VISITED: usize = 20_000;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NavigatorSearch {
    pub path: String,
    pub query: String,
    pub entries: Vec<NavigatorEntry>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct SizeBound {
    op: CmpOp,
    bytes: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct DateBound {
    op: CmpOp,
    /// Inclusive day start (UTC ms) for `=` / `>=` / `>`; exclusive end handled by op.
    day_start_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmpOp {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct SearchQuery {
    /// Name / glob tokens (AND). Empty ⇒ any name.
    name_tokens: Vec<String>,
    /// Extension without dot, lowercased. Empty ⇒ any.
    extensions: Vec<String>,
    /// `dir` / `file` kind constraints.
    kinds: Vec<String>,
    size_bounds: Vec<SizeBound>,
    date_bounds: Vec<DateBound>,
}

pub fn search_directory(root: &str, query: &str, show_hidden: bool) -> Result<NavigatorSearch, String> {
    assert_os_path(root)?;
    let root_path = Path::new(root.trim());
    if !root_path.is_dir() {
        return Err(format!("Path is not a directory: {root}"));
    }
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(NavigatorSearch {
            path: display_path(root_path),
            query,
            entries: Vec::new(),
            truncated: false,
        });
    }

    let parsed = parse_search_query(&query);
    let mut entries = Vec::new();
    let mut truncated = false;
    let mut visited = 0usize;

    for item in WalkDir::new(root_path)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if show_hidden {
                return true;
            }
            match entry.metadata() {
                Ok(metadata) => !entry_is_hidden(&metadata),
                Err(_) => true,
            }
        })
    {
        visited += 1;
        if visited > MAX_VISITED {
            truncated = true;
            break;
        }
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy();
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let is_dir = metadata.is_dir();
        let kind = if is_dir { "dir" } else { "file" };
        let size = if metadata.is_file() { metadata.len() } else { 0 };
        let modified_ms = metadata.modified().ok().and_then(system_time_ms);

        if !entry_matches(&parsed, &name, kind, size, modified_ms) {
            continue;
        }

        entries.push(NavigatorEntry {
            name: name.to_string(),
            path: display_path(entry.path()),
            kind: kind.to_string(),
            size,
            modified_ms,
        });
        if entries.len() >= MAX_RESULTS {
            truncated = true;
            break;
        }
    }

    Ok(NavigatorSearch {
        path: display_path(root_path),
        query,
        entries,
        truncated,
    })
}

fn entry_matches(
    query: &SearchQuery,
    name: &str,
    kind: &str,
    size: u64,
    modified_ms: Option<u64>,
) -> bool {
    if !query.kinds.is_empty() && !query.kinds.iter().any(|k| k == kind) {
        return false;
    }
    if !query.extensions.is_empty() {
        if kind == "dir" {
            return false;
        }
        let ext = file_extension(name);
        if !query.extensions.iter().any(|want| want == &ext) {
            return false;
        }
    }
    for bound in &query.size_bounds {
        if !cmp_u64(size, bound.op, bound.bytes) {
            return false;
        }
    }
    for bound in &query.date_bounds {
        let Some(ms) = modified_ms else {
            return false;
        };
        if !cmp_date(ms, bound) {
            return false;
        }
    }
    if query.name_tokens.is_empty() {
        // Filters-only query (e.g. `type:pdf size:>1mb`) matches any name.
        return true;
    }
    query.name_tokens.iter().all(|token| name_matches(name, token))
}

fn file_extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn cmp_u64(value: u64, op: CmpOp, bound: u64) -> bool {
    match op {
        CmpOp::Eq => value == bound,
        CmpOp::Gt => value > bound,
        CmpOp::Gte => value >= bound,
        CmpOp::Lt => value < bound,
        CmpOp::Lte => value <= bound,
    }
}

fn cmp_date(modified_ms: u64, bound: &DateBound) -> bool {
    let day_ms: u64 = 86_400_000;
    let day_end = bound.day_start_ms.saturating_add(day_ms);
    match bound.op {
        CmpOp::Eq => modified_ms >= bound.day_start_ms && modified_ms < day_end,
        CmpOp::Gt => modified_ms >= day_end,
        CmpOp::Gte => modified_ms >= bound.day_start_ms,
        CmpOp::Lt => modified_ms < bound.day_start_ms,
        CmpOp::Lte => modified_ms < day_end,
    }
}

/// Split query into name tokens and structured filters (`type:` / `size:` / `date:` / `modified:` / `kind:`).
fn parse_search_query(raw: &str) -> SearchQuery {
    let mut out = SearchQuery::default();
    for token in tokenize(raw) {
        if let Some((key, value)) = split_filter(&token) {
            match key.as_str() {
                "type" | "ext" => {
                    let ext = value.trim_start_matches('.').to_lowercase();
                    if ext == "folder" || ext == "dir" || ext == "directory" {
                        out.kinds.push("dir".to_string());
                    } else if ext == "file" {
                        out.kinds.push("file".to_string());
                    } else if !ext.is_empty() {
                        out.extensions.push(ext);
                    }
                }
                "kind" => {
                    let v = value.to_lowercase();
                    if v == "folder" || v == "dir" || v == "directory" {
                        out.kinds.push("dir".to_string());
                    } else if v == "file" {
                        out.kinds.push("file".to_string());
                    }
                }
                "size" => {
                    if let Some(bound) = parse_size_bound(&value) {
                        out.size_bounds.push(bound);
                    }
                }
                "date" | "modified" | "datemodified" => {
                    if let Some(bound) = parse_date_bound(&value) {
                        out.date_bounds.push(bound);
                    }
                }
                _ => out.name_tokens.push(token),
            }
        } else {
            out.name_tokens.push(token);
        }
    }
    out
}

fn tokenize(raw: &str) -> Vec<String> {
    raw.split_whitespace()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn split_filter(token: &str) -> Option<(String, String)> {
    let (key, value) = token.split_once(':')?;
    if key.is_empty() || value.is_empty() {
        return None;
    }
    // Avoid treating Windows drive paths like `C:` as filters.
    if key.len() == 1 && key.chars().next()?.is_ascii_alphabetic() {
        return None;
    }
    Some((key.to_lowercase(), value.to_string()))
}

fn parse_cmp_prefix(raw: &str) -> (CmpOp, &str) {
    if let Some(rest) = raw.strip_prefix(">=") {
        (CmpOp::Gte, rest.trim())
    } else if let Some(rest) = raw.strip_prefix("<=") {
        (CmpOp::Lte, rest.trim())
    } else if let Some(rest) = raw.strip_prefix('>') {
        (CmpOp::Gt, rest.trim())
    } else if let Some(rest) = raw.strip_prefix('<') {
        (CmpOp::Lt, rest.trim())
    } else if let Some(rest) = raw.strip_prefix('=') {
        (CmpOp::Eq, rest.trim())
    } else {
        (CmpOp::Eq, raw.trim())
    }
}

fn parse_size_bound(raw: &str) -> Option<SizeBound> {
    let (op, rest) = parse_cmp_prefix(raw);
    let bytes = parse_byte_size(rest)?;
    Some(SizeBound { op, bytes })
}

fn parse_byte_size(raw: &str) -> Option<u64> {
    let trimmed = raw.trim().to_lowercase().replace('_', "").replace(',', "");
    if trimmed.is_empty() {
        return None;
    }
    let (num, mult) = if let Some(rest) = trimmed.strip_suffix("gb") {
        (rest.trim(), 1024u64 * 1024 * 1024)
    } else if let Some(rest) = trimmed.strip_suffix("mb") {
        (rest.trim(), 1024 * 1024)
    } else if let Some(rest) = trimmed.strip_suffix("kb") {
        (rest.trim(), 1024)
    } else if let Some(rest) = trimmed.strip_suffix('b') {
        (rest.trim(), 1)
    } else {
        (trimmed.as_str(), 1)
    };
    if num.is_empty() {
        return None;
    }
    let value: f64 = num.parse().ok()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    Some((value * mult as f64).round() as u64)
}

fn parse_date_bound(raw: &str) -> Option<DateBound> {
    let (op, rest) = parse_cmp_prefix(raw);
    let day_start_ms = parse_day_start_ms(rest)?;
    Some(DateBound { op, day_start_ms })
}

fn parse_day_start_ms(raw: &str) -> Option<u64> {
    let key = raw.trim().to_lowercase();
    let now = SystemTime::now();
    match key.as_str() {
        "today" => day_start_utc(now),
        "yesterday" => {
            let yesterday = now.checked_sub(Duration::from_secs(86_400))?;
            day_start_utc(yesterday)
        }
        _ => {
            // YYYY-MM-DD
            let parts: Vec<&str> = key.split('-').collect();
            if parts.len() != 3 {
                return None;
            }
            let year: i32 = parts[0].parse().ok()?;
            let month: u32 = parts[1].parse().ok()?;
            let day: u32 = parts[2].parse().ok()?;
            if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
                return None;
            }
            // Approximate UTC midnight via days since Unix epoch (civil calendar).
            let days = civil_days_from_ymd(year, month, day)?;
            Some((days as u64).saturating_mul(86_400).saturating_mul(1000))
        }
    }
}

fn day_start_utc(time: SystemTime) -> Option<u64> {
    let secs = time.duration_since(UNIX_EPOCH).ok()?.as_secs();
    let day = secs / 86_400;
    Some(day.saturating_mul(86_400).saturating_mul(1000))
}

/// Days since Unix epoch for a Gregorian Y-M-D (UTC).
fn civil_days_from_ymd(year: i32, month: u32, day: u32) -> Option<i64> {
    // Algorithm from Howard Hinnant (public domain).
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let m = month as i64;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + day as i64 - 1;
    let doe = yoe as i64 * 365 + (yoe as i64) / 4 - (yoe as i64) / 100 + doy;
    Some(era as i64 * 146_097 + doe - 719_468)
}

fn name_matches(name: &str, query: &str) -> bool {
    let name = name.to_lowercase();
    let query = query.to_lowercase();
    if query.contains('*') || query.contains('?') {
        glob_match(&name, &query)
    } else {
        name.contains(&query)
    }
}

/// `*` any run, `?` one character. Both sides are already lowercased.
fn glob_match(name: &str, pattern: &str) -> bool {
    fn rec(name: &[char], pattern: &[char]) -> bool {
        let mut ni = 0;
        let mut pi = 0;
        while pi < pattern.len() {
            if pattern[pi] == '*' {
                while pi < pattern.len() && pattern[pi] == '*' {
                    pi += 1;
                }
                if pi == pattern.len() {
                    return true;
                }
                while ni <= name.len() {
                    if rec(&name[ni..], &pattern[pi..]) {
                        return true;
                    }
                    if ni == name.len() {
                        break;
                    }
                    ni += 1;
                }
                return false;
            }
            if ni >= name.len() {
                return false;
            }
            if pattern[pi] != '?' && pattern[pi] != name[ni] {
                return false;
            }
            ni += 1;
            pi += 1;
        }
        ni == name.len()
    }

    let name: Vec<char> = name.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();
    rec(&name, &pattern)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    fn scratch(label: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "afm-search-{}-{}-{}",
            std::process::id(),
            label,
            n
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_nested_name_and_skips_root() {
        let dir = scratch("nested");
        fs::create_dir_all(dir.join("nested")).unwrap();
        let mut file = File::create(dir.join("nested").join("Invoice-2026.txt")).unwrap();
        write!(file, "x").unwrap();
        File::create(dir.join("other.dat")).unwrap();

        let hit = search_directory(&dir.to_string_lossy(), "invoice", true).unwrap();
        assert_eq!(hit.entries.len(), 1);
        assert_eq!(hit.entries[0].name, "Invoice-2026.txt");
        assert!(hit.entries[0].path.contains("nested"));
        assert!(!hit.truncated);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn wildcard_matches_extension_only() {
        let dir = scratch("wild");
        fs::create_dir_all(dir.join("a")).unwrap();
        File::create(dir.join("a").join("note.pdf")).unwrap();
        File::create(dir.join("a").join("note.txt")).unwrap();

        let hit = search_directory(&dir.to_string_lossy(), "*.pdf", true).unwrap();
        assert_eq!(hit.entries.len(), 1);
        assert_eq!(hit.entries[0].name, "note.pdf");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_query_does_not_walk() {
        let dir = scratch("empty");
        File::create(dir.join("keep.txt")).unwrap();
        let hit = search_directory(&dir.to_string_lossy(), "   ", true).unwrap();
        assert!(hit.entries.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn type_filter_matches_extension() {
        let dir = scratch("type");
        File::create(dir.join("a.pdf")).unwrap();
        File::create(dir.join("b.txt")).unwrap();
        let hit = search_directory(&dir.to_string_lossy(), "type:pdf", true).unwrap();
        assert_eq!(hit.entries.len(), 1);
        assert_eq!(hit.entries[0].name, "a.pdf");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn size_filter_gt() {
        let dir = scratch("size");
        {
            let mut big = File::create(dir.join("big.bin")).unwrap();
            big.write_all(&[0u8; 2048]).unwrap();
            big.flush().unwrap();
        }
        File::create(dir.join("tiny.bin")).unwrap();
        let hit = search_directory(&dir.to_string_lossy(), "size:>1kb", true).unwrap();
        assert_eq!(hit.entries.len(), 1);
        assert_eq!(hit.entries[0].name, "big.bin");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_combines_name_and_filters() {
        let q = parse_search_query("report type:pdf size:>1mb date:>=2024-01-01");
        assert_eq!(q.name_tokens, vec!["report".to_string()]);
        assert_eq!(q.extensions, vec!["pdf".to_string()]);
        assert_eq!(q.size_bounds.len(), 1);
        assert_eq!(q.date_bounds.len(), 1);
        assert_eq!(q.size_bounds[0].op, CmpOp::Gt);
        assert_eq!(q.size_bounds[0].bytes, 1024 * 1024);
    }

    #[test]
    fn parse_byte_sizes() {
        assert_eq!(parse_byte_size("10").unwrap(), 10);
        assert_eq!(parse_byte_size("1kb").unwrap(), 1024);
        assert_eq!(parse_byte_size("1.5MB").unwrap(), (1.5 * 1024.0 * 1024.0) as u64);
    }
}
