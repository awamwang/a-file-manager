// SPDX-License-Identifier: GPL-3.0-or-later
//! Drill-through listing: flatten a directory tree up to N levels of subfolders
//! (ADR 0031).
//!
//! Depth semantics: depth N flattens N levels of subfolders below the drill
//! root — depth 1 lists the root's own entries plus the contents of its direct
//! subfolders. Depth 0 means "drill off" and is never requested by the
//! frontend.
//!
//! The drill root stays put while the user walks inside the tree, so this
//! command is asked for once per (root, depth) pair and reused afterwards.
use super::list::{display_path, entry_is_hidden, system_time_ms};
use super::path_guard::assert_os_path;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Browse-mode result cap. The in-folder search keeps its own 400 because it
/// is a "find the needle" surface; drill-through is a browsing surface.
const MAX_RESULTS: usize = 2_000;

/// Visited-entry cap — the real filesystem protection. A deep tree must never
/// stall the UI; the walk stops here and reports `truncated`.
const MAX_VISITED: usize = 20_000;

/// Deepest drill depth the UI may request.
pub const MAX_DRILL_DEPTH: u32 = 5;

/// One flattened entry. `path` stays absolute (every operation addresses it),
/// while `relative_path` is the root-relative location shown per row and used
/// as the "name" sort key.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DrillEntry {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub is_link: bool,
    pub link_target: Option<String>,
    pub size: u64,
    pub modified_ms: Option<u64>,
    /// Location below the drill root, e.g. `sub1\sub2\file.txt`.
    pub relative_path: String,
    /// Levels below the drill root (0 = direct child of the root).
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DrillListing {
    /// The drill root that was flattened.
    pub path: String,
    pub entries: Vec<DrillEntry>,
    pub dir_count: u64,
    pub file_count: u64,
    /// Subfolders that contributed rows (shown as "from N subfolders").
    pub source_dir_count: u64,
    /// A cap cut the walk short.
    pub truncated: bool,
}

pub fn drill_directory(root: &str, depth: u32, show_hidden: bool) -> Result<DrillListing, String> {
    assert_os_path(root)?;

    let root_path = Path::new(root.trim());
    if !root_path.is_dir() {
        return Err(format!("Path is not a directory: {root}"));
    }

    // Depth 0 means "drill off". The frontend never asks for it, but keep the
    // command total: an empty listing, never a full-tree walk.
    if depth == 0 {
        return Ok(DrillListing {
            path: display_path(root_path),
            entries: Vec::new(),
            dir_count: 0,
            file_count: 0,
            source_dir_count: 0,
            truncated: false,
        });
    }

    let depth = depth.min(MAX_DRILL_DEPTH);
    // Walk depth = depth + 1: the root's own children (walk depth 1) plus the
    // contents of the deepest expanded subfolder level (walk depth depth+1).
    let max_walk_depth = depth as usize + 1;

    let mut entries: Vec<DrillEntry> = Vec::new();
    let mut source_dirs: BTreeSet<PathBuf> = BTreeSet::new();
    let mut truncated = false;
    let mut visited = 0usize;

    for item in WalkDir::new(root_path)
        .min_depth(1)
        .max_depth(max_walk_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if show_hidden {
                return true;
            }
            match entry.metadata() {
                Ok(metadata) => !entry_is_hidden(&metadata, &entry.file_name().to_string_lossy()),
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
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = metadata.is_dir();
        let kind = if is_dir { "dir" } else { "file" };
        let walk_depth = entry.depth();

        // A folder sitting on the boundary level has no flattened contents —
        // hide it instead of showing an expandable-looking empty row.
        if is_dir && walk_depth >= max_walk_depth {
            continue;
        }
        // Folders that were expanded (walk depth 1..=depth) are the sources of
        // the flattened rows.
        if is_dir {
            source_dirs.insert(entry.path().to_path_buf());
        }

        let is_link = entry.file_type().is_symlink();
        // `info.target` is the directory the link points to (not its parent).
        let link_target = if is_link || name.to_ascii_lowercase().ends_with(".lnk") {
            super::stat::entry_link_info(entry.path()).and_then(|info| info.target)
        } else {
            None
        };
        let size = if metadata.is_file() { metadata.len() } else { 0 };
        let modified_ms = metadata.modified().ok().and_then(system_time_ms);

        entries.push(DrillEntry {
            name,
            path: display_path(entry.path()),
            kind: kind.to_string(),
            is_link,
            link_target,
            size,
            modified_ms,
            relative_path: relative_to(root_path, entry.path()),
            depth: walk_depth.saturating_sub(1) as u32,
        });

        if entries.len() >= MAX_RESULTS {
            truncated = true;
            break;
        }
    }

    entries.sort_by(compare_entries);

    let dir_count = entries.iter().filter(|e| e.kind == "dir").count() as u64;
    let file_count = entries.len() as u64 - dir_count;

    Ok(DrillListing {
        path: display_path(root_path),
        entries,
        dir_count,
        file_count,
        source_dir_count: source_dirs.len() as u64,
        truncated,
    })
}

/// Root-relative location of an entry, joined with the platform separator.
fn relative_to(root: &Path, path: &Path) -> String {
    let Ok(relative) = path.strip_prefix(root) else {
        return path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
    };
    let mut out = String::new();
    for (index, part) in relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .enumerate()
    {
        if index > 0 {
            out.push(std::path::MAIN_SEPARATOR);
        }
        out.push_str(&part);
    }
    out
}

/// Folders first (kernel default), then root-relative path.
fn compare_entries(left: &DrillEntry, right: &DrillEntry) -> Ordering {
    match (left.kind.as_str(), right.kind.as_str()) {
        ("dir", "file") => Ordering::Less,
        ("file", "dir") => Ordering::Greater,
        _ => left
            .relative_path
            .to_ascii_lowercase()
            .cmp(&right.relative_path.to_ascii_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::fs::File;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn scratch(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "afm-drill-{}-{}-{}",
            std::process::id(),
            label,
            SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, body: &str) {
        let mut file = File::create(path).unwrap();
        file.write_all(body.as_bytes()).unwrap();
        file.flush().unwrap();
    }

    /// root/{a.txt, sub1/{b.txt, sub1deep/{c.txt}}, sub2/{d.txt}}
    fn tree(label: &str) -> PathBuf {
        let dir = scratch(label);
        write_file(&dir.join("a.txt"), "a");
        fs::create_dir_all(dir.join("sub1").join("sub1deep")).unwrap();
        write_file(&dir.join("sub1").join("b.txt"), "b");
        write_file(&dir.join("sub1").join("sub1deep").join("c.txt"), "c");
        fs::create_dir_all(dir.join("sub2")).unwrap();
        write_file(&dir.join("sub2").join("d.txt"), "d");
        dir
    }

    #[test]
    fn depth_one_flattens_one_subfolder_level() {
        let dir = tree("depth1");
        let listing = drill_directory(&dir.to_string_lossy(), 1, true).unwrap();

        let mut names: Vec<String> = listing
            .entries
            .iter()
            .map(|entry| entry.relative_path.clone())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "a.txt".to_string(),
                "sub1".to_string(),
                "sub1\\b.txt".to_string(),
                "sub2".to_string(),
                "sub2\\d.txt".to_string(),
            ]
        );
        // The boundary folder (sub1deep) has no flattened contents — hidden.
        assert!(!names.iter().any(|name| name.contains("sub1deep")));
        assert_eq!(listing.source_dir_count, 2);
        assert_eq!(listing.dir_count, 2);
        assert_eq!(listing.file_count, 3);
        assert!(!listing.truncated);
        assert_eq!(listing.path, display_path(&dir));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn depth_two_reaches_the_second_subfolder_level() {
        let dir = tree("depth2");
        let listing = drill_directory(&dir.to_string_lossy(), 2, true).unwrap();

        let mut names: Vec<String> = listing
            .entries
            .iter()
            .map(|entry| entry.relative_path.clone())
            .collect();
        names.sort();
        assert!(names.iter().any(|name| name.ends_with("sub1deep")));
        assert!(names
            .iter()
            .any(|name| name.ends_with(&format!("sub1deep{}c.txt", std::path::MAIN_SEPARATOR))));
        assert_eq!(listing.source_dir_count, 3);
        assert!(!listing.truncated);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn depth_zero_is_empty_and_never_walks() {
        let dir = tree("depth0");
        let listing = drill_directory(&dir.to_string_lossy(), 0, true).unwrap();
        assert!(listing.entries.is_empty());
        assert_eq!(listing.source_dir_count, 0);
        assert!(!listing.truncated);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn relative_depth_counts_levels_below_the_root() {
        let dir = tree("depths");
        let listing = drill_directory(&dir.to_string_lossy(), 2, true).unwrap();

        let root_file = listing
            .entries
            .iter()
            .find(|entry| entry.name == "a.txt")
            .expect("root file");
        assert_eq!(root_file.depth, 0);
        assert_eq!(root_file.relative_path, "a.txt");

        let nested = listing
            .entries
            .iter()
            .find(|entry| entry.name == "c.txt")
            .expect("nested file");
        assert_eq!(nested.depth, 2);
        assert!(nested.relative_path.contains("sub1"));
        assert!(nested.relative_path.contains("sub1deep"));
        // Operations address the absolute path, never the relative one.
        assert!(Path::new(&nested.path).is_absolute() || nested.path.contains(':'));

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn hidden_entries_are_omitted_until_show_hidden() {
        let dir = scratch("hidden");
        write_file(&dir.join("visible.txt"), "v");
        write_file(&dir.join("secret.txt"), "s");
        let status = std::process::Command::new("attrib")
            .arg("+H")
            .arg(dir.join("secret.txt"))
            .status()
            .expect("attrib");
        assert!(status.success());

        let concealed = drill_directory(&dir.to_string_lossy(), 1, false).unwrap();
        assert!(concealed
            .entries
            .iter()
            .all(|entry| entry.name != "secret.txt"));

        let shown = drill_directory(&dir.to_string_lossy(), 1, true).unwrap();
        assert!(shown.entries.iter().any(|entry| entry.name == "secret.txt"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_awam_paths() {
        let err = drill_directory("awam://local/C:/", 1, false).unwrap_err();
        assert!(err.contains("awam://"));
    }
}
