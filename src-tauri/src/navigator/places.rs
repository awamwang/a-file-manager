// SPDX-License-Identifier: GPL-3.0-or-later
//! Home / known user folders + logical drives for Navigator root / guide (AIA-148).

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    /// Drive letter without colon, e.g. `"C"`.
    pub letter: String,
    /// Root path with trailing backslash, e.g. `C:\`.
    pub path: String,
    /// Volume label when available; otherwise empty.
    pub label: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserFolders {
    pub home: String,
    pub desktop: Option<String>,
    pub documents: Option<String>,
    pub downloads: Option<String>,
    pub pictures: Option<String>,
    pub music: Option<String>,
    pub videos: Option<String>,
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

fn existing_dir(path: PathBuf) -> Option<String> {
    if path.is_dir() {
        Some(display_path(&path))
    } else {
        None
    }
}

/// Resolve USERPROFILE (or fallback) and common child folders when they exist.
pub fn user_folders() -> Result<UserFolders, String> {
    let home = super::list::default_path()?;
    let home_path = PathBuf::from(&home);
    Ok(UserFolders {
        home: home.clone(),
        desktop: existing_dir(home_path.join("Desktop")),
        documents: existing_dir(home_path.join("Documents")),
        downloads: existing_dir(home_path.join("Downloads")),
        pictures: existing_dir(home_path.join("Pictures")),
        music: existing_dir(home_path.join("Music")),
        videos: existing_dir(home_path.join("Videos")),
    })
}

/// Enumerate mounted local drive roots (`A:\` … `Z:\`) that currently exist as directories.
pub fn list_drives() -> Result<Vec<DriveInfo>, String> {
    let mut drives = Vec::new();
    for code in b'A'..=b'Z' {
        let letter = (code as char).to_string();
        let path = format!("{letter}:\\");
        let root = Path::new(&path);
        if !root.is_dir() {
            continue;
        }
        // Volume labels need extra Win32 features; letter + path is enough for the guide/root menu.
        drives.push(DriveInfo {
            letter: letter.clone(),
            path,
            label: letter,
        });
    }
    Ok(drives)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_folders_home_exists() {
        let folders = user_folders().unwrap();
        assert!(Path::new(&folders.home).is_dir());
    }

    #[test]
    fn list_drives_returns_at_least_one_on_windows() {
        let drives = list_drives().unwrap();
        #[cfg(windows)]
        assert!(!drives.is_empty());
        for drive in &drives {
            assert!(drive.path.ends_with('\\'));
            assert!(Path::new(&drive.path).is_dir());
        }
    }
}
