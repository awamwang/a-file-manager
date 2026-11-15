// SPDX-License-Identifier: GPL-3.0-or-later
//! macOS / Unix-like adapter (compiled for every non-Windows target).
//!
//! This is an honest capability declaration, not a claim of feature parity:
//! Windows-only integrations (Shell verb, Everything, Peek, HKCU Run) stay
//! `false` so the UI can hide them until a native implementation lands.

use super::{
    has_uri_scheme, Platform, PlatformCapabilities, Volume, VolumeKind, VolumeSpace, WindowChrome,
};
use std::fs::Metadata;
use std::path::PathBuf;

pub struct MacosPlatform;

impl Platform for MacosPlatform {
    fn id(&self) -> &'static str {
        #[cfg(target_os = "macos")]
        {
            "macos"
        }
        #[cfg(not(target_os = "macos"))]
        {
            std::env::consts::OS
        }
    }

    fn primary_modifier(&self) -> &'static str {
        #[cfg(target_os = "macos")]
        {
            "cmd"
        }
        #[cfg(not(target_os = "macos"))]
        {
            "ctrl"
        }
    }

    fn path_separator(&self) -> char {
        '/'
    }

    fn case_insensitive_paths(&self) -> bool {
        false
    }

    fn home_dir(&self) -> Option<PathBuf> {
        directories::UserDirs::new()
            .map(|d| d.home_dir().to_path_buf())
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
    }

    fn to_native_fs_path(&self, path: &str) -> String {
        let trimmed = path.trim();
        if trimmed.is_empty() || has_uri_scheme(trimmed) {
            return trimmed.trim_end_matches('/').to_string();
        }
        // POSIX paths already use `/`. A backslash is a legal filename
        // character here, so never rewrite it the way Windows does.
        let mut native = trimmed.to_string();
        while native.ends_with('/') && native.len() > 1 {
            native.pop();
        }
        native
    }

    fn normalize_for_comparison(&self, path: &str) -> String {
        // APFS is usually case-insensitive but not guaranteed; do not fold
        // case at the contract level (Windows did, and that leaked into keys).
        self.to_native_fs_path(path)
    }

    fn capabilities(&self) -> PlatformCapabilities {
        let mut caps = PlatformCapabilities::neutral(
            self.id(),
            self.primary_modifier(),
            self.path_separator(),
        );
        caps.case_insensitive_paths = self.case_insensitive_paths();
        // Already available through existing cross-platform crates / plugins.
        caps.shell_icons = true; // file_icon_provider (NSWorkspace)
                                 // trash 5.2 moves items via Finder automation on macOS; it needs the
                                 // app to hold Automation/Finder TCC permission (see research doc 7.5).
        caps.trash_delete = true;
        // trash::os_limited (list/restore/purge) is Windows + freedesktop only;
        // macOS is explicitly excluded by the crate, so keep this off and let
        // the UI offer `open_recycle_bin_system()` (`~/.Trash`) instead.
        caps.trash_list_restore = false;
        caps.system_tray = true; // Tauri TrayIconBuilder → NSStatusItem
        // 7.13: the AppKit drag-out command in `tauri-plugin-drag` aborts the
        // process on macOS, so `native_drag_out` stays false. The working
        // transport is the WebView-native HTML5 `text/uri-list` drag-out
        // (src/state/dnd/dragOut.ts + NavigatorView.vue).
        caps.webview_drag_out = true;
        // 7.16: volumes() lists `/` and `/Volumes/*`; there are no drive letters.
        caps.volume_list = true;
        // Deliberately false until implemented: see docs/macos-port-research.md.
        caps
    }

    /// Moved from `navigator::places::list_drives` (7.16). `sysinfo::Disks`
    /// exposes real mount points: the root volume plus user `/Volumes/*`.
    /// APFS helper volumes under `/System/Volumes/*` are invisible in Finder,
    /// so they are excluded to avoid flooding the sidebar.
    fn volumes(&self) -> Result<Vec<Volume>, String> {
        use sysinfo::Disks;

        let disks = Disks::new_with_refreshed_list()
            .map_err(|err| format!("Failed to enumerate mounted volumes: {err}"))?;
        let mut volumes = Vec::new();
        for disk in disks.iter() {
            let mount = disk.mount_point();
            let path = crate::paths::display_path(mount);
            if path != "/" && !path.starts_with("/Volumes/") {
                continue;
            }
            if !mount.is_dir() || volumes.iter().any(|v: &Volume| v.path == path) {
                continue;
            }
            volumes.push(Volume {
                letter: mount
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| path.clone()),
                path,
                label: disk.name().to_string_lossy().to_string(),
                kind: VolumeKind::Fixed,
            });
        }
        Ok(volumes)
    }

    /// Moved from `navigator::disk_space` (7.16): pick the longest mount point
    /// containing `path` (e.g. `/Volumes/USB` wins over `/`).
    fn volume_space(&self, path: &str) -> Result<VolumeSpace, String> {
        use sysinfo::Disks;

        let target = path.trim();
        if target.is_empty() {
            return Err("Path must not be empty".to_string());
        }
        let disks = Disks::new_with_refreshed_list()
            .map_err(|err| format!("Failed to enumerate mounted volumes: {err}"))?;
        let target_path = std::path::Path::new(target);
        let mut best: Option<(String, u64, u64, usize)> = None;
        for disk in disks.iter() {
            let mount = disk.mount_point();
            if !target_path.starts_with(mount) {
                continue;
            }
            let len = mount.as_os_str().len();
            if best.as_ref().map_or(true, |b| len > b.3) {
                best = Some((
                    crate::paths::display_path(mount),
                    disk.available_space(),
                    disk.total_space(),
                    len,
                ));
            }
        }
        let (root, free_bytes, total_bytes, _) =
            best.ok_or_else(|| "Path has no volume root".to_string())?;
        Ok(VolumeSpace {
            path: root,
            free_bytes,
            total_bytes,
        })
    }

    fn is_hidden(&self, name: &str, _metadata: &Metadata) -> bool {
        // POSIX has no hidden attribute: dot-files are the convention.
        name.starts_with('.') && name != "." && name != ".."
    }

    fn window_chrome(&self) -> WindowChrome {
        #[cfg(target_os = "macos")]
        {
            WindowChrome::MacOverlay
        }
        #[cfg(not(target_os = "macos"))]
        {
            WindowChrome::FluentSelfDrawn
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_paths_are_not_rewritten() {
        let p = MacosPlatform;
        assert_eq!(
            p.to_native_fs_path("/Users/wangmeng/Docs/"),
            "/Users/wangmeng/Docs"
        );
        assert_eq!(p.to_native_fs_path("/"), "/");
        assert_eq!(p.to_native_fs_path("afm://guide"), "afm://guide");
        assert_eq!(
            p.normalize_for_comparison("/Users/A/File.txt"),
            "/Users/A/File.txt"
        );
    }

    #[test]
    fn windows_only_capabilities_are_disabled() {
        let caps = MacosPlatform.capabilities();
        assert!(!caps.registry_autostart);
        assert!(!caps.os_shell_verb);
        assert!(!caps.trash_list_restore);
        assert!(!caps.system_thumbnails);
        assert!(!caps.webview2_required);
        assert_eq!(caps.path_separator, "/");
        #[cfg(target_os = "macos")]
        assert_eq!(caps.primary_modifier, "cmd");
    }
}
