// SPDX-License-Identifier: GPL-3.0-or-later

use super::path_guard::assert_os_path;
use serde::Serialize;
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NavigatorEntry {
    pub name: String,
    pub path: String,
    /// `"dir"` or `"file"` (symlinks reported by target kind when possible).
    pub kind: String,
    pub size: u64,
    /// Last write time as Unix epoch milliseconds (when the FS reports it).
    pub modified_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NavigatorListing {
    pub path: String,
    pub entries: Vec<NavigatorEntry>,
    pub dir_count: u64,
    pub file_count: u64,
}

pub fn list_directory(path: &str) -> Result<NavigatorListing, String> {
    assert_os_path(path)?;

    let root = PathBuf::from(path.trim());
    if !root.exists() {
        return Err(format!("Path does not exist: {path}"));
    }
    if !root.is_dir() {
        return Err(format!("Path is not a directory: {path}"));
    }

    let read_dir = fs::read_dir(&root).map_err(|err| format!("Failed to read directory: {err}"))?;

    let mut entries: Vec<NavigatorEntry> = Vec::new();
    for item in read_dir {
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let entry_path = entry.path();
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        let is_dir = metadata.is_dir();
        let kind = if is_dir { "dir" } else { "file" }.to_string();
        let size = if metadata.is_file() { metadata.len() } else { 0 };
        let name = entry.file_name().to_string_lossy().to_string();
        let modified_ms = metadata.modified().ok().and_then(system_time_ms);

        entries.push(NavigatorEntry {
            name,
            path: display_path(&entry_path),
            kind,
            size,
            modified_ms,
        });
    }

    entries.sort_by(compare_entries);

    let dir_count = entries.iter().filter(|e| e.kind == "dir").count() as u64;
    let file_count = entries.len() as u64 - dir_count;

    Ok(NavigatorListing {
        path: display_path(&root),
        entries,
        dir_count,
        file_count,
    })
}

pub fn default_path() -> Result<String, String> {
    if let Ok(home) = std::env::var("USERPROFILE") {
        if !home.trim().is_empty() && Path::new(&home).is_dir() {
            return Ok(display_path(Path::new(&home)));
        }
    }
    if Path::new(r"C:\").is_dir() {
        return Ok(r"C:\".to_string());
    }
    Err("Could not resolve a default browse path".to_string())
}

pub fn parent_path(path: &str) -> Result<Option<String>, String> {
    assert_os_path(path)?;
    let current = PathBuf::from(path.trim());
    match current.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => Ok(Some(display_path(parent))),
        _ => Ok(None),
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

fn system_time_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

fn compare_entries(left: &NavigatorEntry, right: &NavigatorEntry) -> Ordering {
    match (left.kind.as_str(), right.kind.as_str()) {
        ("dir", "file") => Ordering::Less,
        ("file", "dir") => Ordering::Greater,
        _ => left.name.to_ascii_lowercase().cmp(&right.name.to_ascii_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn lists_temp_directory_entries() {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-nav-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir(dir.join("alpha-dir")).unwrap();
        let mut file = File::create(dir.join("beta.txt")).unwrap();
        write!(file, "hi").unwrap();

        let listing = list_directory(&dir.to_string_lossy()).unwrap();
        assert_eq!(listing.dir_count, 1);
        assert_eq!(listing.file_count, 1);
        assert_eq!(listing.entries[0].kind, "dir");
        assert_eq!(listing.entries[0].name, "alpha-dir");
        assert_eq!(listing.entries[1].name, "beta.txt");
        assert_eq!(listing.entries[1].kind, "file");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_awam_in_list() {
        let err = list_directory("awam://local/C:/").unwrap_err();
        assert!(err.contains("awam://"));
    }

    #[test]
    fn default_path_is_os_directory() {
        let path = default_path().unwrap();
        assert!(Path::new(&path).is_dir());
        assert!(!path.to_ascii_lowercase().starts_with("awam://"));
    }

    #[test]
    fn parent_of_nested_path() {
        let parent = parent_path(r"C:\Windows\System32").unwrap();
        assert_eq!(parent.as_deref(), Some(r"C:\Windows"));
    }

    #[test]
    fn parent_of_drive_roots_is_none() {
        for p in [r"D:\", r"D:/", r"D:", r"C:\", r"G:\", r"\\server\share"] {
            let got = parent_path(p).unwrap();
            assert!(got.is_none(), "{p} should have no FS parent, got {got:?}");
        }
        assert_eq!(parent_path(r"D:\Projects").unwrap().as_deref(), Some(r"D:\"));
    }
}
