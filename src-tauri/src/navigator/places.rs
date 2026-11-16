// SPDX-License-Identifier: GPL-3.0-or-later
//! Home / known user folders + logical drives for Navigator root / guide (AIA-148).

use serde::Serialize;
use std::path::{Path, PathBuf};

/// Logical drive kind (guide / This PC visuals). 7.16: the neutral enum now
/// lives in the platform layer; `DriveKind` stays as an alias so the frontend
/// wire contract and existing `DriveKind::Fixed` call sites are unchanged.
pub use crate::platform::VolumeKind as DriveKind;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    /// Drive letter without colon, e.g. `"C"`.
    pub letter: String,
    /// Root path with trailing backslash, e.g. `C:\`.
    pub path: String,
    /// Volume label when available; otherwise empty.
    pub label: String,
    /// Drive type for guide coloring / icons.
    pub kind: DriveKind,
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
    pub onedrive: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlace {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkPlace {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellPlaces {
    pub onedrive: Option<String>,
    pub libraries: Vec<LibraryPlace>,
    pub network: Vec<NetworkPlace>,
}

fn existing_dir(path: PathBuf) -> Option<String> {
    if path.is_dir() {
        Some(crate::paths::display_path(&path))
    } else {
        None
    }
}

/// `directories::UserDirs` returns `Option<&Path>`; only keep folders that exist.
#[cfg(not(windows))]
fn existing_opt_dir(path: Option<&Path>) -> Option<String> {
    path.and_then(|p| existing_dir(p.to_path_buf()))
}

fn resolve_onedrive(home_path: &Path) -> Option<String> {
    if let Ok(value) = std::env::var("OneDrive") {
        if let Some(path) = existing_dir(PathBuf::from(value.trim())) {
            return Some(path);
        }
    }
    if let Ok(value) = std::env::var("OneDriveConsumer") {
        if let Some(path) = existing_dir(PathBuf::from(value.trim())) {
            return Some(path);
        }
    }
    existing_dir(home_path.join("OneDrive"))
}

/// Resolve the home directory and common child folders when they exist.
///
/// Windows keeps the original USERPROFILE-relative layout (including the
/// OneDrive env vars). Other platforms use `directories::UserDirs`, which is
/// locale-aware and follows the OS special-folder rules instead of guessing
/// English child names under `$HOME`.
pub fn user_folders() -> Result<UserFolders, String> {
    let home = super::list::default_path()?;
    let home_path = PathBuf::from(&home);

    #[cfg(windows)]
    {
        Ok(UserFolders {
            home: home.clone(),
            desktop: existing_dir(home_path.join("Desktop")),
            documents: existing_dir(home_path.join("Documents")),
            downloads: existing_dir(home_path.join("Downloads")),
            pictures: existing_dir(home_path.join("Pictures")),
            music: existing_dir(home_path.join("Music")),
            videos: existing_dir(home_path.join("Videos")),
            onedrive: resolve_onedrive(&home_path),
        })
    }

    #[cfg(not(windows))]
    {
        let dirs = directories::UserDirs::new();
        Ok(UserFolders {
            home,
            desktop: dirs
                .as_ref()
                .and_then(|d| existing_opt_dir(d.desktop_dir())),
            documents: dirs
                .as_ref()
                .and_then(|d| existing_opt_dir(d.document_dir())),
            downloads: dirs
                .as_ref()
                .and_then(|d| existing_opt_dir(d.download_dir())),
            pictures: dirs
                .as_ref()
                .and_then(|d| existing_opt_dir(d.picture_dir())),
            music: dirs.as_ref().and_then(|d| existing_opt_dir(d.audio_dir())),
            videos: dirs.as_ref().and_then(|d| existing_opt_dir(d.video_dir())),
            onedrive: resolve_onedrive(&home_path),
        })
    }
}

/// Libraries / OneDrive / mapped network shares for Guide + Places.
pub fn shell_places() -> Result<ShellPlaces, String> {
    let folders = user_folders()?;
    let home_path = PathBuf::from(&folders.home);
    let mut libraries = Vec::new();
    for (id, path) in [
        ("documents", folders.documents.clone()),
        ("music", folders.music.clone()),
        ("pictures", folders.pictures.clone()),
        ("videos", folders.videos.clone()),
    ] {
        if let Some(path) = path {
            libraries.push(LibraryPlace {
                id: id.to_string(),
                path,
            });
        }
    }
    Ok(ShellPlaces {
        onedrive: folders.onedrive.or_else(|| resolve_onedrive(&home_path)),
        libraries,
        network: list_network_places(),
    })
}

fn list_network_places() -> Vec<NetworkPlace> {
    #[cfg(not(windows))]
    {
        // No `net use` on macOS. Mounted shares live under /Volumes and belong
        // to the P1 `sysinfo::Disks` volume work, not to network-place enumeration.
        return Vec::new();
    }

    #[cfg(windows)]
    {
        let output = std::process::Command::new("cmd")
            .args(["/C", "net use"])
            .output();
        let Ok(output) = output else {
            return Vec::new();
        };
        if !output.status.success() {
            return Vec::new();
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut places = Vec::new();
        for line in text.lines() {
            // Typical: OK  Z:  \\server\share  Microsoft Windows Network
            let trimmed = line.trim();
            if !trimmed.contains("\\\\") {
                continue;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            let unc = parts.iter().find(|part| part.starts_with(r"\\"));
            let Some(unc) = unc else { continue };
            let letter = parts
                .iter()
                .find(|part| part.len() == 2 && part.ends_with(':'))
                .copied();
            let name = letter.unwrap_or(*unc).to_string();
            let path = if let Some(letter) = letter {
                format!("{letter}\\")
            } else {
                (*unc).to_string()
            };
            places.push(NetworkPlace { name, path });
        }
        places
    }
}

/// Enumerate mounted volumes. 7.16: the OS-specific enumeration (Win32
/// `GetLogicalDrives` bitmap / `sysinfo::Disks` mount points) now lives in the
/// platform adapters; this layer only maps the neutral `Volume` contract onto
/// the frontend `DriveInfo` wire type.
///
pub fn list_drives() -> Result<Vec<DriveInfo>, String> {
    Ok(crate::platform::volumes()?
        .into_iter()
        .map(|volume| DriveInfo {
            letter: volume.letter,
            path: volume.path,
            label: volume.label,
            kind: volume.kind,
        })
        .collect())
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
            assert!(Path::new(&drive.path).is_dir());
            assert!(!drive.letter.is_empty());
            #[cfg(windows)]
            assert!(drive.path.ends_with('\\'));
            #[cfg(not(windows))]
            assert!(drive.path.starts_with('/'));
        }
        #[cfg(windows)]
        {
            let system = drives.iter().find(|d| d.letter.eq_ignore_ascii_case("C"));
            if let Some(drive) = system {
                assert_eq!(drive.kind, DriveKind::Fixed);
            }
        }
    }

    #[test]
    fn shell_places_libraries_come_from_user_folders() {
        let places = shell_places().unwrap();
        for lib in &places.libraries {
            assert!(Path::new(&lib.path).is_dir());
            assert!(["documents", "music", "pictures", "videos"].contains(&lib.id.as_str()));
        }
    }
}
