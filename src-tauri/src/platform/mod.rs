// SPDX-License-Identifier: GPL-3.0-or-later
//! Thin platform abstraction layer (macOS port). 7.16 brought volumes, disk
//! space, hidden-file semantics and window chrome back under this roof.
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
//! - OS-specific behaviour (volume enumeration, free space, hidden files,
//!   window chrome) belongs in `windows.rs` / `macos.rs`; business modules
//!   call the free functions here instead of opening new `#[cfg]` branches.
//! - On the frontend, real filesystem paths go through `src/state/fsPath.ts`.
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

/// Neutral, OS-independent description of a mounted volume. The frontend
/// contract (`navigator::places::DriveInfo`) is built from this in the
/// navigator layer, so no drive letters / Win32 types leak through here.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VolumeKind {
    Fixed,
    Removable,
    Remote,
    Cdrom,
    Ramdisk,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    /// Native root / mount point, e.g. `C:\`, `/`, `/Volumes/USB`.
    pub path: String,
    /// Short identifier for the sidebar: drive letter or mount basename.
    pub letter: String,
    pub label: String,
    pub kind: VolumeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeSpace {
    /// Root of the volume that actually contains the requested path.
    pub path: String,
    pub free_bytes: u64,
    pub total_bytes: u64,
}

/// Neutral description of a launchable desktop application (terminal / IDE).
/// `resolved_path` is an absolute executable on Windows and an `.app` bundle
/// or CLI binary on macOS; `None` means the app is not installed, so the UI
/// can mark it unavailable without knowing how discovery works (7.20).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchTarget {
    pub id: String,
    pub name: String,
    pub resolved_path: Option<String>,
    /// Windows UAC elevation; macOS has no equivalent and reports false so
    /// the "run as administrator" submenu is not rendered.
    pub supports_elevation: bool,
}

/// How the main window chrome is composed. The window setup code reads this
/// instead of hardcoding `#[cfg(target_os = "macos")]` branches (7.16).
// Some variants are only constructed by the adapter for another target.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowChrome {
    /// Windows: `decorations:false` + the self-drawn Fluent caption bar.
    FluentSelfDrawn,
    /// macOS: native decorations with a transparent Overlay titlebar, so the
    /// traffic lights float over the Fluent caption instead of a second row.
    MacOverlay,
}

/// Which OS index backs the global search page (7.21). The UI uses this to
/// label the primary engine card and to hide Windows-only settings; the
/// discovery service uses it to pick its backend.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalSearchEngine {
    /// `Everything` IPC (Windows only).
    EverythingIpc,
    /// Spotlight via `/usr/bin/mdfind` (macOS only).
    Spotlight,
    /// No host-wide index available (other Unix targets).
    None,
}

/// One hit from the host-wide index. Paths are host-native; `is_dir` follows
/// symlinks so the Navigator can open folders directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQueryResult {
    /// Total hits the host index reported before paging.
    pub total_count: u64,
    pub offset: u64,
    pub hits: Vec<SearchHit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchProbe {
    pub available: bool,
    pub detail: String,
}

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
    /// A host-wide index exists (Everything on Windows, Spotlight on macOS).
    pub global_search: bool,
    /// Which one: "everything" | "spotlight" | "none" (7.21). Kept separate
    /// from the bool so macOS can hide the "Prefer Everything" setting while
    /// still reporting a working global search backend.
    pub global_search_engine: String,
    pub quick_look_preview: bool,
    pub ole_drag_out: bool,
    /// AppKit / native drag-out transport. Off on macOS since 7.13: the
    /// `tauri-plugin-drag` command aborts the process there.
    pub native_drag_out: bool,
    /// WebView-native HTML5 file drag-out (`text/uri-list`), the macOS
    /// transport used after 7.13. Combine with `ole_drag_out` to decide
    /// whether cross-application drag-out is available at all.
    pub webview_drag_out: bool,
    pub hidden_file_attributes: bool,
    pub drive_letters: bool,
    /// Mounted volumes are enumerable (drive letters on Windows, mount
    /// points on macOS). Independent of `drive_letters` (7.16).
    pub volume_list: bool,
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
            global_search_engine: "none".to_string(),
            quick_look_preview: false,
            ole_drag_out: false,
            native_drag_out: false,
            webview_drag_out: false,
            hidden_file_attributes: false,
            drive_letters: false,
            volume_list: false,
            system_tray: false,
            webview2_required: false,
        }
    }
}

/// OS-neutral RGBA bitmap. Adapters own every platform image type; callers only
/// see pixels, so `shell_icons` can stay free of `#[cfg]` (7.23).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaIcon {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
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
    /// Mounted volumes for the Navigator's "This PC" sidebar.
    fn volumes(&self) -> Result<Vec<Volume>, String>;
    /// Free / total space of the volume containing `path`.
    fn volume_space(&self, path: &str) -> Result<VolumeSpace, String>;
    /// Hidden-file semantics (Win32 attribute vs POSIX dot-files).
    fn is_hidden(&self, name: &str, metadata: &std::fs::Metadata) -> bool;
    /// Window chrome composition for the main window.
    fn window_chrome(&self) -> WindowChrome;
    /// Installed terminals / IDEs. Returns every preset with `resolved_path`
    /// set only when discovery succeeded, so the UI keeps its unavailable rows.
    fn terminal_targets(&self) -> Vec<LaunchTarget>;
    fn ide_targets(&self) -> Vec<LaunchTarget>;
    /// Validate a user-configured override from Settings → Open tools.
    fn configured_launch_target(&self, id: &str, path: &str) -> Option<LaunchTarget>;
    /// Launch a resolved terminal at `dir` / a resolved IDE at `path`.
    fn launch_terminal(
        &self,
        target: &LaunchTarget,
        dir: &std::path::Path,
        elevated: bool,
    ) -> Result<(), String>;
    fn launch_ide(&self, target: &LaunchTarget, path: &std::path::Path) -> Result<(), String>;
    /// Host-wide index selection (7.21). `EverythingIpc` means the discovery
    /// service keeps its original Windows backend; `Spotlight` means it should
    /// route queries through `global_search()` instead.
    fn global_search_engine(&self) -> GlobalSearchEngine;
    /// Cheap availability probe — must not shell out or run a query.
    fn probe_global_search(&self) -> SearchProbe;
    /// Name-based host-wide search with paging. Only called for adapters that
    /// report a real engine; Windows returns `Err` and uses Everything IPC.
    fn global_search(
        &self,
        query: &str,
        max_results: u32,
        offset: u32,
    ) -> Result<SearchQueryResult, String>;
    /// Terminate a process together with its descendants (rclone → ssh, sidecars).
    /// Windows keeps `taskkill /F /T`; Unix walks the PPID graph via `sysinfo`.
    fn kill_process_tree(&self, pid: u32);
    /// Default interpreter for MCP sidecars (`python` on Windows, `python3` on Unix).
    fn python_command(&self) -> &'static str;
    /// Native file-type icon in RGBA form, without leaking OS image types.
    fn file_icon(&self, path: &std::path::Path, size: u16) -> Result<RgbaIcon, String>;
    /// Native content thumbnail when the host exposes one; otherwise `Err` and
    /// callers fall back to `file_icon`.
    fn system_thumbnail(&self, path: &std::path::Path, size: u16) -> Result<RgbaIcon, String>;
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

/// Single entry point for volume enumeration (both OS adapters).
pub fn volumes() -> Result<Vec<Volume>, String> {
    current().volumes()
}

/// Single entry point for free-space queries (both OS adapters).
pub fn volume_space(path: &str) -> Result<VolumeSpace, String> {
    current().volume_space(path)
}

/// Single entry point for hidden-file semantics (both OS adapters).
pub fn is_hidden(name: &str, metadata: &std::fs::Metadata) -> bool {
    current().is_hidden(name, metadata)
}

/// Single entry point for main-window chrome composition.
pub fn window_chrome() -> WindowChrome {
    current().window_chrome()
}

/// 7.20: OS-specific launcher discovery / spawning lives only in the adapters.
pub fn terminal_targets() -> Vec<LaunchTarget> {
    current().terminal_targets()
}

pub fn ide_targets() -> Vec<LaunchTarget> {
    current().ide_targets()
}

pub fn configured_launch_target(id: &str, path: &str) -> Option<LaunchTarget> {
    current().configured_launch_target(id, path)
}

pub fn launch_terminal(
    target: &LaunchTarget,
    dir: &std::path::Path,
    elevated: bool,
) -> Result<(), String> {
    current().launch_terminal(target, dir, elevated)
}

pub fn launch_ide(target: &LaunchTarget, path: &std::path::Path) -> Result<(), String> {
    current().launch_ide(target, path)
}

/// 7.21: host-wide search lives in the adapters; no `mdfind` / `Everything`
/// IPC details may leak into `discovery/`.
pub fn global_search_engine() -> GlobalSearchEngine {
    current().global_search_engine()
}

pub fn probe_global_search() -> SearchProbe {
    current().probe_global_search()
}

pub fn global_search(
    query: &str,
    max_results: u32,
    offset: u32,
) -> Result<SearchQueryResult, String> {
    current().global_search(query, max_results, offset)
}

pub fn kill_process_tree(pid: u32) {
    current().kill_process_tree(pid);
}

pub fn python_command() -> &'static str {
    current().python_command()
}

pub fn file_icon(path: &std::path::Path, size: u16) -> Result<RgbaIcon, String> {
    current().file_icon(path, size)
}

pub fn system_thumbnail(path: &std::path::Path, size: u16) -> Result<RgbaIcon, String> {
    current().system_thumbnail(path, size)
}

/// macOS-only finishing touch for `WindowChrome::MacOverlay`: let the webview
/// content reach the window top. Runtime `set_decorations(true)` drops the
/// `FullSizeContentView` bit, which leaves a 28pt native titlebar band above
/// the self-drawn caption (7.18). No-op elsewhere; the AppKit details stay
/// inside the macOS adapter.
pub fn extend_content_under_titlebar(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    macos::extend_content_under_titlebar(window);
    #[cfg(not(target_os = "macos"))]
    let _ = window;
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

    /// 7.21: the TS field list is checked statically, but the IPC payload is
    /// produced by serde — make sure the camelCase key really reaches the UI.
    #[test]
    fn capabilities_serialize_the_search_engine() {
        let value = serde_json::to_value(current().capabilities()).unwrap();
        let engine = value["globalSearchEngine"].as_str().unwrap();
        #[cfg(target_os = "macos")]
        assert_eq!(engine, "spotlight");
        #[cfg(windows)]
        assert_eq!(engine, "everything");
        #[cfg(not(any(target_os = "macos", windows)))]
        assert_eq!(engine, "none");
    }
}
