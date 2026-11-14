// SPDX-License-Identifier: GPL-3.0-or-later
//! Thin platform abstraction layer (macOS port, step 1).
//!
//! Cross-platform browsers keep OS-specific code out of business logic: they
//! define a platform-neutral contract, select one thin adapter at build time,
//! and expose capability negotiation instead of throwing on missing features.
//! Tauri only adapts the WebView/window, not file-manager semantics, so this
//! module is where those differences belong.
//!
//! Rules for new code:
//! - Never hardcode `\`, drive letters, `USERPROFILE`, or `%VAR%` outside this
//!   module. Use `to_native_fs_path` / `normalize_for_comparison` / `home_dir`.
//! - Query `platform_capabilities()` before showing a Windows-only feature.
//! - OS-specific imports must be `#[cfg]`-gated *and* target-gated in
//!   `Cargo.toml`, or macOS builds fail with unresolved imports.

use once_cell::sync::Lazy;
use serde::Serialize;
use std::path::PathBuf;

#[cfg(windows)]
mod windows;
// Unix-like fallback. macOS is the first non-Windows target; Linux reuses it.
#[cfg(not(windows))]
mod macos;

/// Describes which desktop capabilities this build can actually deliver.
/// The frontend should hide/disable unsupported entries instead of invoking
/// a command that returns `Err("... only supported on Windows")`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub os: String,
    pub arch: String,
    /// "ctrl" on Windows, "cmd" on macOS — drives shortcut labels and matching.
    pub primary_modifier: String,
    pub path_separator: String,
    pub case_insensitive_paths: bool,
    pub shell_icons: bool,
    pub system_thumbnails: bool,
    pub trash_delete: bool,
    pub trash_list_restore: bool,
    pub native_open_with_dialog: bool,
    pub native_context_menu: bool,
    pub os_shell_verb: bool,
    pub registry_autostart: bool,
    pub native_autostart: bool,
    pub single_instance: bool,
    pub global_search: bool,
    pub quick_look_preview: bool,
    pub ole_drag_out: bool,
    pub native_drag_out: bool,
    pub hidden_file_attributes: bool,
    pub drive_letters: bool,
    pub system_tray: bool,
    pub webview2_required: bool,
}

impl PlatformCapabilities {
    /// Everything defaults to unavailable; each adapter opts in explicitly.
    fn neutral(os: &str, primary_modifier: &str, separator: char) -> Self {
        Self {
            os: os.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            primary_modifier: primary_modifier.to_string(),
            path_separator: separator.to_string(),
            case_insensitive_paths: false,
            shell_icons: false,
            system_thumbnails: false,
            trash_delete: false,
            trash_list_restore: false,
            native_open_with_dialog: false,
            native_context_menu: false,
            os_shell_verb: false,
            registry_autostart: false,
            native_autostart: false,
            single_instance: false,
            global_search: false,
            quick_look_preview: false,
            ole_drag_out: false,
            native_drag_out: false,
            hidden_file_attributes: false,
            drive_letters: false,
            system_tray: false,
            webview2_required: false,
        }
    }
}

/// The neutral contract. Implementations live in `windows.rs` / `macos.rs`.
pub trait Platform: Send + Sync {
    fn id(&self) -> &'static str;
    fn primary_modifier(&self) -> &'static str;
    fn path_separator(&self) -> char;
    fn case_insensitive_paths(&self) -> bool;
    fn home_dir(&self) -> Option<PathBuf>;
    fn to_native_fs_path(&self, path: &str) -> String;
    fn normalize_for_comparison(&self, path: &str) -> String;
    fn capabilities(&self) -> PlatformCapabilities;
}

static CURRENT: Lazy<Box<dyn Platform>> = Lazy::new(|| {
    #[cfg(windows)]
    {
        Box::new(windows::WindowsPlatform)
    }
    #[cfg(not(windows))]
    {
        Box::new(macos::MacosPlatform)
    }
});

pub fn current() -> &'static dyn Platform {
    CURRENT.as_ref()
}

/// Single entry point for display-path → filesystem-path conversion.
pub fn to_native_fs_path(path: &str) -> String {
    current().to_native_fs_path(path)
}

/// Single entry point for path keys / cache keys / comparisons.
pub fn normalize_for_comparison(path: &str) -> String {
    current().normalize_for_comparison(path)
}

pub fn home_dir() -> Option<PathBuf> {
    current().home_dir()
}

/// True for `afm://guide`, `http://…`, etc. — these are not filesystem paths
/// and must keep their separators on every OS.
pub(super) fn has_uri_scheme(value: &str) -> bool {
    let Some(idx) = value.find(':') else {
        return false;
    };
    let scheme = &value[..idx];
    !scheme.is_empty()
        && scheme.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_alphanumeric() || b == b'+' || b == b'-' || b == b'.' || (i > 0 && b == b'_')
        })
        && value[idx + 1..].starts_with("//")
}

/// Frontend entry point: one call replaces scattered `if (isWindows)` checks.
#[tauri::command]
pub fn platform_capabilities() -> PlatformCapabilities {
    current().capabilities()
}

/// Startup browse path. `navigator::default_path` should use this instead of
/// reading `USERPROFILE` and falling back to `C:\`.
#[tauri::command]
pub fn platform_home_dir() -> Result<String, String> {
    home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not resolve a default browse path".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_scheme_detection() {
        assert!(has_uri_scheme("afm://guide"));
        assert!(has_uri_scheme("https://example.com/a"));
        assert!(!has_uri_scheme("C:/Users"));
        assert!(!has_uri_scheme("/Users/wangmeng"));
    }

    #[test]
    fn capabilities_match_the_active_adapter() {
        let caps = current().capabilities();
        assert_eq!(caps.os, current().id());
        assert_eq!(caps.primary_modifier, current().primary_modifier());
        assert_eq!(caps.path_separator, current().path_separator().to_string());
    }

    #[test]
    fn native_path_keeps_uri_untouched() {
        assert_eq!(to_native_fs_path("afm://guide"), "afm://guide");
    }
}
