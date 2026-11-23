// SPDX-License-Identifier: GPL-3.0-or-later
//! Per-path Navigator visit / open counts + Quick Access ranking (AIA-74).
//!
//! Stored as `visit-stats.json` beside favorites/settings in app config dir.
//! Independent of Work Group `openState` (no schema conflict).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

pub const VISIT_STATS_FILE_NAME: &str = "visit-stats.json";
/// Soft cap on persisted paths (prune by oldest lastOpenedAt).
pub const VISIT_STORE_CAP: usize = 200;
/// Max entries returned for Quick Access UI.
pub const QUICK_ACCESS_CAP: usize = 12;
/// Recency window used by the combined score (days).
pub const RECENCY_WINDOW_DAYS: f64 = 30.0;
/// Weight of recency boost relative to raw openCount.
pub const RECENCY_WEIGHT: f64 = 10.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VisitItem {
    pub path: String,
    pub open_count: u64,
    pub last_opened_at: u64,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct VisitStatsDocument {
    pub visits: Vec<VisitItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VisitRecordResult {
    pub document: VisitStatsDocument,
    pub path: String,
    pub open_count: u64,
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config dir: {err}"))
}

fn visit_stats_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join(VISIT_STATS_FILE_NAME)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn normalize_path(path: &str) -> String {
    // Comparison key only. Delegate to the platform adapter so POSIX paths
    // keep `/` instead of becoming a relative `\Users\...` key (7.20).
    crate::paths::to_native_fs_path(path)
}

pub fn load_from_path(config_dir: &Path) -> Result<VisitStatsDocument, String> {
    let path = visit_stats_file_path(config_dir);
    if !path.exists() {
        let document = VisitStatsDocument::default();
        save_to_path(config_dir, &document)?;
        return Ok(document);
    }
    let text =
        fs::read_to_string(&path).map_err(|err| format!("Failed to read visit-stats: {err}"))?;
    let document: VisitStatsDocument = serde_json::from_str(&text)
        .map_err(|err| format!("Failed to parse visit-stats JSON: {err}"))?;
    Ok(document)
}

pub fn save_to_path(config_dir: &Path, document: &VisitStatsDocument) -> Result<(), String> {
    fs::create_dir_all(config_dir)
        .map_err(|err| format!("Failed to create visit-stats directory: {err}"))?;
    let path = visit_stats_file_path(config_dir);
    let text = serde_json::to_string_pretty(document)
        .map_err(|err| format!("Failed to serialize visit-stats: {err}"))?;
    fs::write(&path, text).map_err(|err| format!("Failed to write visit-stats: {err}"))?;
    Ok(())
}

/// Combined score: openCount + recency boost decaying over RECENCY_WINDOW_DAYS.
pub fn visit_score(item: &VisitItem, now_ms: u64) -> f64 {
    let age_ms = now_ms.saturating_sub(item.last_opened_at) as f64;
    let age_days = age_ms / 86_400_000.0;
    let recency = (1.0 - (age_days / RECENCY_WINDOW_DAYS)).clamp(0.0, 1.0) * RECENCY_WEIGHT;
    item.open_count as f64 + recency
}

pub fn rank_quick_access(visits: &[VisitItem], now_ms: u64, cap: usize) -> Vec<VisitItem> {
    let mut ranked: Vec<VisitItem> = visits.to_vec();
    ranked.sort_by(|a, b| {
        let sa = visit_score(a, now_ms);
        let sb = visit_score(b, now_ms);
        sb.partial_cmp(&sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.last_opened_at.cmp(&a.last_opened_at))
            .then_with(|| a.path.cmp(&b.path))
    });
    if ranked.len() > cap {
        ranked.truncate(cap);
    }
    ranked
}

fn prune_store(document: &mut VisitStatsDocument) {
    if document.visits.len() <= VISIT_STORE_CAP {
        return;
    }
    document
        .visits
        .sort_by(|a, b| b.last_opened_at.cmp(&a.last_opened_at));
    document.visits.truncate(VISIT_STORE_CAP);
}

/// Increment openCount / refresh lastOpenedAt for a path (identity = normalized path).
pub fn record_visit(
    document: &mut VisitStatsDocument,
    path: &str,
    is_dir: bool,
    at_ms: u64,
) -> Result<u64, String> {
    let normalized = normalize_path(path);
    if normalized.is_empty() {
        return Err("Path is empty".to_string());
    }
    if let Some(existing) = document
        .visits
        .iter_mut()
        .find(|item| normalize_path(&item.path).eq_ignore_ascii_case(&normalized))
    {
        existing.open_count = existing.open_count.saturating_add(1);
        existing.last_opened_at = at_ms;
        existing.is_dir = is_dir;
        let count = existing.open_count;
        prune_store(document);
        return Ok(count);
    }
    document.visits.push(VisitItem {
        path: normalized,
        open_count: 1,
        last_opened_at: at_ms,
        is_dir,
    });
    prune_store(document);
    Ok(1)
}

/// Drop the entry whose identity matches `path` (case-insensitive, normalized).
/// Returns how many rows were removed (0 or 1 — paths are unique keys).
pub fn remove_visit(document: &mut VisitStatsDocument, path: &str) -> usize {
    let normalized = normalize_path(path);
    if normalized.is_empty() {
        return 0;
    }
    let before = document.visits.len();
    document.visits.retain(|item| {
        !normalize_path(&item.path).eq_ignore_ascii_case(&normalized)
    });
    before - document.visits.len()
}

/// Forget every recorded visit (used by "clear recent visits").
pub fn clear_visits(document: &mut VisitStatsDocument) {
    document.visits.clear();
}

#[tauri::command]
pub fn visit_stats_get(app: AppHandle) -> Result<VisitStatsDocument, String> {
    let dir = config_dir(&app)?;
    load_from_path(&dir)
}

#[tauri::command]
pub fn visit_stats_record(
    app: AppHandle,
    path: String,
    is_dir: bool,
) -> Result<VisitRecordResult, String> {
    let dir = config_dir(&app)?;
    let mut document = load_from_path(&dir)?;
    let count = record_visit(&mut document, &path, is_dir, now_ms())?;
    save_to_path(&dir, &document)?;
    Ok(VisitRecordResult {
        path: normalize_path(&path),
        open_count: count,
        document,
    })
}

/// Remove a single recent-visit entry by path (the hover ✕ affordance).
#[tauri::command]
pub fn visit_stats_remove(app: AppHandle, path: String) -> Result<VisitStatsDocument, String> {
    let dir = config_dir(&app)?;
    let mut document = load_from_path(&dir)?;
    remove_visit(&mut document, &path);
    save_to_path(&dir, &document)?;
    Ok(document)
}

/// Clear all recent visits (the "clear recent visits" affordance).
#[tauri::command]
pub fn visit_stats_clear(app: AppHandle) -> Result<VisitStatsDocument, String> {
    let dir = config_dir(&app)?;
    let mut document = load_from_path(&dir)?;
    clear_visits(&mut document);
    save_to_path(&dir, &document)?;
    Ok(document)
}

#[tauri::command]
pub fn visit_stats_quick_access(app: AppHandle) -> Result<Vec<VisitItem>, String> {
    let dir = config_dir(&app)?;
    let document = load_from_path(&dir)?;
    Ok(rank_quick_access(
        &document.visits,
        now_ms(),
        QUICK_ACCESS_CAP,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_increments_same_path() {
        let mut doc = VisitStatsDocument::default();
        // Both spellings of the *host's* separator must fold to one key. The
        // old `C:\Docs` / `C:/Docs` fixture only made sense on Windows (7.20).
        let (first, second) = if cfg!(windows) {
            (r"C:\Docs", "C:/Docs")
        } else {
            ("/Users/demo/Docs", "/Users/demo/Docs/")
        };
        assert_eq!(record_visit(&mut doc, first, true, 1000).unwrap(), 1);
        assert_eq!(record_visit(&mut doc, second, true, 2000).unwrap(), 2);
        assert_eq!(doc.visits.len(), 1);
        assert_eq!(doc.visits[0].last_opened_at, 2000);
        assert_eq!(doc.visits[0].open_count, 2);
    }

    #[cfg(not(windows))]
    #[test]
    fn posix_paths_never_become_backslash_keys() {
        assert_eq!(
            normalize_path("/Users/demo/Docs/"),
            "/Users/demo/Docs"
        );
        let mut doc = VisitStatsDocument::default();
        record_visit(&mut doc, "/Users/demo/Docs/", false, 1000).unwrap();
        assert_eq!(doc.visits[0].path, "/Users/demo/Docs");
        assert!(!doc.visits[0].path.contains('\\'));
    }

    #[test]
    fn empty_path_errors() {
        let mut doc = VisitStatsDocument::default();
        assert!(record_visit(&mut doc, "  ", true, 1).is_err());
    }

    #[test]
    fn ranking_prefers_frequency_then_recency() {
        let now = 30 * 86_400_000;
        let visits = vec![
            VisitItem {
                path: r"C:\Rare".into(),
                open_count: 1,
                last_opened_at: now,
                is_dir: true,
            },
            VisitItem {
                path: r"C:\Hot".into(),
                open_count: 20,
                last_opened_at: now - 2 * 86_400_000,
                is_dir: true,
            },
            VisitItem {
                path: r"C:\Warm".into(),
                open_count: 5,
                last_opened_at: now,
                is_dir: true,
            },
        ];
        let ranked = rank_quick_access(&visits, now, 3);
        assert_eq!(ranked[0].path, r"C:\Hot");
        assert_eq!(ranked[1].path, r"C:\Warm");
        assert_eq!(ranked[2].path, r"C:\Rare");
    }

    #[test]
    fn prune_keeps_most_recent() {
        let mut doc = VisitStatsDocument::default();
        for i in 0..(VISIT_STORE_CAP + 5) {
            let path = format!(r"C:\p{i}");
            record_visit(&mut doc, &path, true, i as u64).unwrap();
        }
        assert_eq!(doc.visits.len(), VISIT_STORE_CAP);
        let max_ts = doc.visits.iter().map(|v| v.last_opened_at).max().unwrap();
        assert_eq!(max_ts, (VISIT_STORE_CAP + 4) as u64);
    }

    #[test]
    fn persists_round_trip() {
        let dir =
            std::env::temp_dir().join(format!("a-file-manager-visits-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut document = load_from_path(&dir).unwrap();
        record_visit(&mut document, r"D:\Work", true, 42).unwrap();
        save_to_path(&dir, &document).unwrap();

        let loaded = load_from_path(&dir).unwrap();
        assert_eq!(loaded.visits.len(), 1);
        assert_eq!(loaded.visits[0].open_count, 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_drops_matching_entry_case_insensitively() {
        let mut doc = VisitStatsDocument::default();
        record_visit(&mut doc, r"C:\Work", true, 1000).unwrap();
        record_visit(&mut doc, r"C:\Other", true, 2000).unwrap();
        // The hover ✕ passes the path as stored (or a differently-cased spelling).
        let removed = remove_visit(&mut doc, r"c:\work");
        assert_eq!(removed, 1);
        assert_eq!(doc.visits.len(), 1);
        assert_eq!(doc.visits[0].path, r"C:\Other");
    }

    #[test]
    fn remove_missing_path_is_noop() {
        let mut doc = VisitStatsDocument::default();
        record_visit(&mut doc, r"C:\Work", true, 1000).unwrap();
        assert_eq!(remove_visit(&mut doc, r"C:\Nope"), 0);
        assert_eq!(doc.visits.len(), 1);
        assert_eq!(remove_visit(&mut doc, "   "), 0);
    }

    #[test]
    fn clear_empties_all_visits() {
        let mut doc = VisitStatsDocument::default();
        record_visit(&mut doc, r"C:\A", true, 1000).unwrap();
        record_visit(&mut doc, r"C:\B", true, 2000).unwrap();
        clear_visits(&mut doc);
        assert!(doc.visits.is_empty());
    }
}
