// SPDX-License-Identifier: GPL-3.0-or-later
//! Recursive name search under one folder (current directory and descendants).

use super::list::{display_path, entry_is_hidden, system_time_ms, NavigatorEntry};
use super::path_guard::assert_os_path;
use serde::Serialize;
use std::path::Path;
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
        if !name_matches(&name, &query) {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let is_dir = metadata.is_dir();
        let modified_ms = metadata.modified().ok().and_then(system_time_ms);
        entries.push(NavigatorEntry {
            name: name.to_string(),
            path: display_path(entry.path()),
            kind: if is_dir { "dir" } else { "file" }.to_string(),
            size: if metadata.is_file() { metadata.len() } else { 0 },
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
}
