// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows adapter. Semantics are identical to the pre-port code: native `\`,
//! drive letters / UNC, case-insensitive comparison, HKCU Run, Win32 icons.

use super::{has_uri_scheme, Platform, PlatformCapabilities};
use std::path::PathBuf;

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
        caps.system_tray = true;
        caps.webview2_required = true;
        caps
    }
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
}
