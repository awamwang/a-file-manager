// SPDX-License-Identifier: GPL-3.0-or-later
//! Home / known user folders + logical drives for Navigator root / guide (AIA-148).

use serde::Serialize;
use std::path::{Path, PathBuf};

/// Logical drive kind from `GetDriveTypeW` (guide / This PC visuals).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DriveKind {
    Fixed,
    Removable,
    Remote,
    Cdrom,
    Ramdisk,
    Unknown,
}

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
        onedrive: resolve_onedrive(&home_path),
    })
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

#[cfg(windows)]
fn drive_kind_for_root(path: &str) -> DriveKind {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;

    // Win32 DRIVE_* constants (avoid extra Cargo feature for WindowsProgramming).
    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;
    const DRIVE_REMOTE: u32 = 4;
    const DRIVE_CDROM: u32 = 5;
    const DRIVE_RAMDISK: u32 = 6;

    let wide: Vec<u16> = std::ffi::OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    match unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) } {
        DRIVE_REMOVABLE => DriveKind::Removable,
        DRIVE_FIXED => DriveKind::Fixed,
        DRIVE_REMOTE => DriveKind::Remote,
        DRIVE_CDROM => DriveKind::Cdrom,
        DRIVE_RAMDISK => DriveKind::Ramdisk,
        _ => DriveKind::Unknown,
    }
}

#[cfg(not(windows))]
fn drive_kind_for_root(_path: &str) -> DriveKind {
    DriveKind::Unknown
}

#[cfg(windows)]
fn volume_label_for_root(path: &str) -> String {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetVolumeInformationW;

    let wide: Vec<u16> = std::ffi::OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut name = [0u16; 64];
    let ok = unsafe {
        GetVolumeInformationW(
            PCWSTR(wide.as_ptr()),
            Some(&mut name),
            None,
            None,
            None,
            None,
        )
    };
    if ok.is_err() {
        return String::new();
    }
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    String::from_utf16_lossy(&name[..end]).trim().to_string()
}

#[cfg(not(windows))]
fn volume_label_for_root(_path: &str) -> String {
    String::new()
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
        let kind = drive_kind_for_root(&path);
        let label = volume_label_for_root(&path);
        drives.push(DriveInfo {
            letter: letter.clone(),
            path,
            label,
            kind,
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
            assert!(!drive.letter.is_empty());
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
