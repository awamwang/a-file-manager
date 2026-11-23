// SPDX-License-Identifier: GPL-3.0-or-later
//! Home / known user folders + logical drives for Navigator root / guide (AIA-148).

use serde::Serialize;
use std::collections::HashSet;
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

/// A browsable network location. `path` is always a share-level UNC on
/// Windows (`\\server\share`) or a local mount point on macOS (`/Volumes/...`),
/// so the frontend can hand it straight to `navigator_list` (ADR 0026). `name`
/// is the short label (share / mount basename) shown in the tree + Guide card.
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
///
/// Kept intentionally FAST: `network` only reflects already-connected
/// `net use` shares so the awaited Guide first paint never blocks on LAN
/// discovery. Slower network-neighborhood discovery lives behind
/// [`network_shares`] on its own lazy command (ADR 0026).
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
        network: mapped_network_places(),
    })
}

/// Mapped / connected shares from `net use` (fast). Always returns UNC paths so
/// the tree + Guide can navigate them directly with `navigator_list`.
fn mapped_network_places() -> Vec<NetworkPlace> {
    #[cfg(not(windows))]
    {
        Vec::new()
    }

    #[cfg(windows)]
    {
        // AIA-148: must go through win_process::command — a bare Command::new
        // from the GUI subsystem allocates a console, which Win11 hands to the
        // default terminal (Windows Terminal) and pops a window on startup.
        let output = crate::win_process::command("cmd")
            .args(["/C", "net use"])
            .output();
        match output {
            Ok(output) if output.status.success() => {
                parse_net_use(&String::from_utf8_lossy(&output.stdout))
            }
            _ => Vec::new(),
        }
    }
}

/// Full network view for the This PC → Network tree node and the Guide network
/// cards: connected shares plus best-effort LAN discovery, de-duplicated by
/// path. Runs on the blocking pool behind its own command so it never gates the
/// fast Guide path (ADR 0026).
pub fn network_shares() -> Vec<NetworkPlace> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut places: Vec<NetworkPlace> = Vec::new();
    for place in mapped_network_places()
        .into_iter()
        .chain(discovered_network_places())
    {
        if place.path.trim().is_empty() {
            continue;
        }
        if seen.insert(place.path.to_lowercase()) {
            places.push(place);
        }
    }
    places.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.path.to_lowercase().cmp(&b.path.to_lowercase()))
    });
    places
}

/// Network-neighborhood discovery behind the mapped list. Bounded + best-effort:
/// a flat LAN answers in seconds; a dead network degrades to "no shares found"
/// without ever blocking the UI, because callers run this off the main thread.
fn discovered_network_places() -> Vec<NetworkPlace> {
    #[cfg(windows)]
    {
        // ADR 0026: enumerate live computers from the Shell Network folder
        // (WS-Discovery), NOT the legacy SMB1 `net view` browser — the latter is
        // disabled by default on Win10/11 and hangs. Each computer becomes a
        // bare `\\server` node; its shares are listed lazily via
        // [`list_server_shares`] when the node expands.
        crate::platform::network_computers()
            .into_iter()
            .map(|server| NetworkPlace {
                name: server.clone(),
                path: format!(r"\\{server}"),
            })
            .collect()
    }

    #[cfg(not(windows))]
    {
        // macOS / Linux: enumerate already-mounted network volumes from `mount`.
        // No `-p`: BSD `mount -p` exists on FreeBSD but macOS rejects it with
        // "illegal option -- p", which made the Network node permanently empty.
        // Plain `mount` works everywhere and its output is parsed by
        // `parse_mount_network` (both `dev on /path (fstype, …)` and tab forms).
        // Bonjour (dns-sd) enumeration of *unmounted* servers is intentionally
        // left out — it is a different transport, can hang, and cannot be
        // navigated until mounted, so it would not yield a browsable path here.
        let output = std::process::Command::new("mount").output();
        match output {
            Ok(output) if output.status.success() => {
                parse_mount_network(&String::from_utf8_lossy(&output.stdout))
            }
            _ => Vec::new(),
        }
    }
}

/// Shares exported by a single server (`net view \\server`), bounded by a hard
/// timeout so an unreachable host cannot hang the listing. ADR 0026: this backs
/// the two-level Network tree — expanding a `\\server` node lists its shares.
pub fn list_server_shares(server: &str) -> Vec<NetworkPlace> {
    #[cfg(not(windows))]
    {
        let _ = server;
        Vec::new()
    }

    #[cfg(windows)]
    {
        let server_root = format!(r"\\{server}");
        let Some(output) = crate::win_process::output_capped(
            "cmd",
            &["/C", "net view", server_root.as_str()],
            std::time::Duration::from_secs(8),
        ) else {
            return Vec::new();
        };
        parse_net_view_shares(&String::from_utf8_lossy(&output))
            .into_iter()
            .map(|share| NetworkPlace {
                name: share.clone(),
                path: format!(r"\\{server}\{share}"),
            })
            .collect()
    }
}

/// Pure CLI / mount output parsers. Each OS uses a subset of them in its
/// `cfg` branch, but all are exercised by `#[cfg(test)]` on every host, so the
/// ones an OS build does not call are marked `allow(dead_code)` (keeps the
/// repo's `cargo check` warning-free on both Windows and macOS).
///
/// `net use` table → `NetworkPlace`. Lines look like:
/// `OK    Z:    \\server\share    Microsoft Windows Network`. Only rows that
/// carry a `\\…` UNC column are network locations; the UNC itself is the label
/// source so a drive letter and its bare connection both normalise to `\\…`.
#[allow(dead_code)]
pub(crate) fn parse_net_use(text: &str) -> Vec<NetworkPlace> {
    let mut places = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.contains("\\\\") {
            continue;
        }
        let unc = match trimmed.split_whitespace().find(|part| part.starts_with(r"\\")) {
            Some(unc) => unc,
            None => continue,
        };
        places.push(NetworkPlace {
            name: share_label(unc),
            path: unc.to_string(),
        });
    }
    places
}

/// `net view \\server` share table → share names. Disk shares only; lines look
/// like `PUBLIC   Disk`. The header row ("Share name …") and Print/IPC entries
/// are filtered out.
#[allow(dead_code)]
pub(crate) fn parse_net_view_shares(text: &str) -> Vec<String> {
    let mut shares = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(r"\\") {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let name = match parts.next() {
            Some(name) => name,
            None => continue,
        };
        let kind = parts.next().unwrap_or("");
        // Keep Disk shares; drop the "Share name" header and admin/IPC/Print.
        if name.eq_ignore_ascii_case("Share") || !kind.eq_ignore_ascii_case("Disk") {
            continue;
        }
        shares.push(name.to_string());
    }
    shares
}

/// Parse one `mount` line into `(mountpoint, fstype)`.
/// Accepts both the BSD `mount -p` tab form (`dev \t mountpoint \t fstype \t opts`)
/// and the portable `dev on /mount/point (fstype, opts…)` form that macOS / Linux
/// print with a bare `mount`. The mountpoint may contain spaces, so the `on`
/// form is split at the trailing ` (` that opens the fstype parenthesis rather
/// than on whitespace.
fn parse_mount_line(line: &str) -> Option<(&str, &str)> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() >= 3 {
        return Some((fields[1].trim(), fields[2].trim()));
    }
    let (_, rest) = line.split_once(" on ")?;
    let open = rest.rfind(" (")?;
    let mount = rest[..open].trim();
    let inside = rest[open + 2..].trim_end_matches(')');
    let fstype = inside.split(',').next()?.trim();
    Some((mount, fstype))
}

/// `mount` output → network volume entries. The mount point is what we can
/// actually browse, so it becomes `path`; the mount basename becomes the label.
#[allow(dead_code)]
pub(crate) fn parse_mount_network(text: &str) -> Vec<NetworkPlace> {
    const NETWORK_FSTYPES: [&str; 5] = ["smbfs", "afpfs", "nfs", "cifs", "webdav"];
    let mut places = Vec::new();
    for line in text.lines() {
        let Some((mount, fstype_raw)) = parse_mount_line(line) else {
            continue;
        };
        let fstype = fstype_raw.to_ascii_lowercase();
        if !NETWORK_FSTYPES.contains(&fstype.as_str()) {
            continue;
        }
        if mount.is_empty() {
            continue;
        }
        places.push(NetworkPlace {
            name: Path::new(mount)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| mount.to_string()),
            path: mount.to_string(),
        });
    }
    places
}

/// `\\server\share` → `share`; falls back to the full UNC when parsing leaves
/// nothing usable (e.g. a bare `\\server` row).
#[allow(dead_code)]
fn share_label(unc: &str) -> String {
    unc.trim_end_matches('\\')
        .rsplit(['\\', '/'])
        .next()
        .filter(|s| !s.is_empty() && *s != unc.trim_start_matches('\\'))
        .map(|s| s.to_string())
        .unwrap_or_else(|| unc.to_string())
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

    #[test]
    fn parse_net_use_keeps_unc_and_share_label() {
        let text = concat!(
            "New connections will be remembered.\r\n",
            "Status   Local   Remote            Network\r\n",
            "---      -----   ------            -------\r\n",
            "OK       Z:      \\\\FILE01\\Public   Microsoft Windows Network\r\n",
            "OK               \\\\NAS\\Media       Microsoft Windows Network\r\n",
            "The command completed successfully.\r\n",
        );
        let places = parse_net_use(text);
        assert_eq!(places.len(), 2);
        assert_eq!(places[0].path, r"\\FILE01\Public");
        assert_eq!(places[0].name, "Public");
        assert_eq!(places[1].path, r"\\NAS\Media");
        assert_eq!(places[1].name, "Media");
    }

    #[test]
    fn parse_net_use_ignores_local_only_output() {
        assert!(parse_net_use("Local status     Network status\r\nNo entries.\r\n").is_empty());
    }

    #[test]
    fn parse_net_view_shares_keeps_disk_only() {
        let text = concat!(
            "Share name  Type   Used as  Disk\r\n",
            "----------       ----   -------  ----\r\n",
            "Public           Disk\r\n",
            "ADMIN$           Disk\r\n",
            "print$           Print\r\n",
            "IPC$             IPC\r\n",
            "The command completed successfully.\r\n",
        );
        assert_eq!(parse_net_view_shares(text), vec!["Public", "ADMIN$"]);
    }

    #[test]
    fn parse_mount_network_filters_to_network_fstypes() {
        let text = concat!(
            "//user@NAS.local/Media\t/Volumes/Media\tsmbfs\tlocal,nodev\n",
            "/dev/disk3s1s1\t/\tapfs\tlocal,Snapshot,encrypted\n",
            "server:/export\t/Volumes/nfs\tnfs\tlocal\n",
        );
        let places = parse_mount_network(text);
        assert_eq!(places.len(), 2);
        assert_eq!(places[0].path, "/Volumes/Media");
        assert_eq!(places[0].name, "Media");
        assert_eq!(places[1].name, "nfs");
    }

    #[test]
    fn parse_mount_network_accepts_macos_bare_mount_output() {
        // macOS `mount` (no -p) prints `dev on /mount/point (fstype, options)`.
        // Mount points with spaces are legal and must stay intact.
        let text = concat!(
            "/dev/disk3s5 on / (apfs, local, journaled)\n",
            "//user@NAS.local/Media on /Volumes/Media (smbfs, nodev, nosuid, mounted by user)\n",
            "server:/export on /Volumes/My Shared Files (nfs, nodev, nosuid, mounted by user)\n",
            "//guest@host/Public on /Volumes/Public (smbfs, nodev, nosuid)\n",
        );
        let places = parse_mount_network(text);
        assert_eq!(places.len(), 3);
        assert_eq!(places[0].path, "/Volumes/Media");
        assert_eq!(places[0].name, "Media");
        assert_eq!(places[1].path, "/Volumes/My Shared Files");
        assert_eq!(places[1].name, "My Shared Files");
        assert_eq!(places[2].name, "Public");
    }
}
