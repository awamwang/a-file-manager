// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows adapter. Semantics are identical to the pre-port code: native `\`,
//! drive letters / UNC, case-insensitive comparison, HKCU Run, Win32 icons.

use super::{
    has_uri_scheme, Platform, PlatformCapabilities, Volume, VolumeKind, VolumeSpace, WindowChrome,
};
use std::fs::Metadata;
use std::path::{Path, PathBuf};

pub struct WindowsPlatform;

impl Platform for WindowsPlatform {
    fn id(&self) -> &'static str {
        "windows"
    }

    fn primary_modifier(&self) -> &'static str {
        "ctrl"
    }

    fn path_separator(&self) -> char {
        '\\'
    }

    fn case_insensitive_paths(&self) -> bool {
        true
    }

    fn home_dir(&self) -> Option<PathBuf> {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .or_else(|| directories::UserDirs::new().map(|d| d.home_dir().to_path_buf()))
    }

    fn to_native_fs_path(&self, path: &str) -> String {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        let bytes = trimmed.as_bytes();
        let is_drive = bytes.len() >= 2
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && (bytes.len() == 2 || bytes[2] == b'/' || bytes[2] == b'\\');
        let is_unc = trimmed.starts_with("\\\\") || trimmed.starts_with("//");
        if is_drive || is_unc {
            let mut native = trimmed.replace('/', "\\");
            if native.len() == 3
                && native.as_bytes()[0].is_ascii_alphabetic()
                && native.as_bytes()[1] == b':'
                && native.as_bytes()[2] == b'\\'
            {
                return native;
            }
            while native.ends_with('\\') && native.len() > 3 {
                native.pop();
            }
            return native;
        }
        if has_uri_scheme(trimmed) {
            return trimmed.trim_end_matches('/').to_string();
        }
        let mut native = trimmed.replace('/', "\\");
        while native.ends_with('\\') {
            native.pop();
        }
        native
    }

    fn normalize_for_comparison(&self, path: &str) -> String {
        self.to_native_fs_path(path).to_ascii_lowercase()
    }

    fn capabilities(&self) -> PlatformCapabilities {
        let mut caps = PlatformCapabilities::neutral(
            self.id(),
            self.primary_modifier(),
            self.path_separator(),
        );
        caps.case_insensitive_paths = self.case_insensitive_paths();
        caps.shell_icons = true;
        caps.system_thumbnails = true;
        caps.trash_delete = true;
        caps.trash_list_restore = true;
        caps.native_open_with_dialog = true;
        caps.native_context_menu = true;
        caps.os_shell_verb = true;
        caps.registry_autostart = true;
        caps.single_instance = true;
        caps.global_search = true;
        caps.quick_look_preview = true;
        caps.ole_drag_out = true;
        caps.hidden_file_attributes = true;
        caps.drive_letters = true;
        caps.volume_list = true; // volumes() enumerates the drive-letter bitmap
        caps.system_tray = true;
        caps.webview2_required = true;
        caps
    }

    /// Moved verbatim from `navigator::places::list_drives` (7.16): the
    /// `GetLogicalDrives` bitmap avoids stat-ing offline mapped drives.
    fn volumes(&self) -> Result<Vec<Volume>, String> {
        use windows::Win32::Storage::FileSystem::GetLogicalDrives;

        let mask = unsafe { GetLogicalDrives() };
        let mut drives = Vec::new();
        for i in 0..26u32 {
            if mask & (1 << i) == 0 {
                continue;
            }
            let letter = ((b'A' + i as u8) as char).to_string();
            let path = format!("{letter}:\\");
            let kind = drive_kind_for_root(&path);
            let present = match kind {
                VolumeKind::Remote | VolumeKind::Cdrom | VolumeKind::Ramdisk => true,
                _ => Path::new(&path).is_dir(),
            };
            if !present {
                continue;
            }
            let label = match kind {
                VolumeKind::Fixed | VolumeKind::Removable => volume_label_for_root(&path),
                _ => String::new(),
            };
            drives.push(Volume {
                letter,
                path,
                label,
                kind,
            });
        }
        Ok(drives)
    }

    /// Moved verbatim from `navigator::disk_space` (7.16).
    fn volume_space(&self, path: &str) -> Result<VolumeSpace, String> {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

        let root = volume_root(path).ok_or_else(|| "Path has no volume root".to_string())?;
        let wide: Vec<u16> = std::ffi::OsStr::new(&root)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut free_available = 0u64;
        let mut total = 0u64;
        let mut total_free = 0u64;
        unsafe {
            GetDiskFreeSpaceExW(
                PCWSTR(wide.as_ptr()),
                Some(&mut free_available),
                Some(&mut total),
                Some(&mut total_free),
            )
            .map_err(|err| format!("GetDiskFreeSpaceExW failed: {err}"))?;
        }
        Ok(VolumeSpace {
            path: root,
            free_bytes: free_available,
            total_bytes: total,
        })
    }

    fn is_hidden(&self, _name: &str, metadata: &Metadata) -> bool {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 2;
        metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
    }

    fn window_chrome(&self) -> WindowChrome {
        WindowChrome::FluentSelfDrawn
    }
}

fn drive_kind_for_root(path: &str) -> VolumeKind {
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
        DRIVE_REMOVABLE => VolumeKind::Removable,
        DRIVE_FIXED => VolumeKind::Fixed,
        DRIVE_REMOTE => VolumeKind::Remote,
        DRIVE_CDROM => VolumeKind::Cdrom,
        DRIVE_RAMDISK => VolumeKind::Ramdisk,
        _ => VolumeKind::Unknown,
    }
}

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

fn volume_root(path: &str) -> Option<String> {
    let trimmed = path.trim().replace('/', "\\");
    if trimmed.starts_with("\\\\") {
        let parts: Vec<&str> = trimmed.trim_start_matches('\\').split('\\').collect();
        if parts.len() >= 2 {
            return Some(format!(r"\\{}\{}\", parts[0], parts[1]));
        }
        return None;
    }
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        let letter = trimmed.chars().next()?.to_ascii_uppercase();
        return Some(format!(r"{letter}:\"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_drive_and_uri() {
        let p = WindowsPlatform;
        assert_eq!(p.to_native_fs_path("G:/Projects/A"), r"G:\Projects\A");
        assert_eq!(p.to_native_fs_path(r"C:\"), r"C:\");
        assert_eq!(p.to_native_fs_path("afm://guide"), "afm://guide");
        assert_eq!(
            p.normalize_for_comparison("g:/projects/a"),
            r"g:\projects\a"
        );
    }

    #[test]
    fn volume_root_drive_and_unc() {
        assert_eq!(volume_root(r"C:\Temp\afm-fixture\a").as_deref(), Some(r"C:\"));
        assert_eq!(
            volume_root(r"\\server\share\folder").as_deref(),
            Some(r"\\server\share\")
        );
        assert_eq!(volume_root("afm://guide"), None);
    }
}
