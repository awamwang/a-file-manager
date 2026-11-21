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

/// `launch_terminal` 的失败细分。`Cancelled` 表示用户在 UAC 授权弹窗点了
/// “否”——调用方应立即停止尝试其它终端，而不是给下一个终端再弹一次 UAC。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalLaunchError {
    /// 用户取消了 UAC 授权（elevated 启动）。
    Cancelled,
    /// 启动失败，附带人话错误。
    Failed(String),
}

impl TerminalLaunchError {
    pub fn message(&self) -> String {
        match self {
            Self::Cancelled => "已取消管理员授权".to_string(),
            Self::Failed(msg) => msg.clone(),
        }
    }
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

/// Neutral file-clipboard payload (CF_HDROP + Preferred DropEffect on Windows).
/// The DTO lives here so the command layer can return it without importing an
/// OS-specific module (7.24).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellClipboard {
    pub paths: Vec<String>,
    pub mode: Option<String>,
}

/// Neutral Recycle Bin row. Same reason as `ShellClipboard` (7.24).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecycleBinItem {
    pub name: String,
    pub original_path: String,
    /// Shell item path inside the bin (used for restore).
    pub recycle_path: String,
}

/// File-trash provider. `delete` works on every host through the `trash`
/// crate; list/restore need a host bin API, so the adapter declares whether
/// it really supports them instead of letting a capability bit drift away.
pub trait TrashProvider: Send + Sync {
    fn delete(&self, path: &std::path::Path, path_text: &str) -> Result<(), String>;
    fn list(&self) -> Result<Vec<RecycleBinItem>, String>;
    fn restore(&self, original_paths: &[String]) -> Result<Vec<String>, String>;
    fn supports_list_restore(&self) -> bool;
}

/// Host-native preview backend. Windows keeps its named-pipe / Peek bridge in
/// `preview_bridge`; macOS registers this provider so the same setting really
/// opens Quick Look instead of silently falling back to the built-in pane.
pub trait PreviewProvider: Send + Sync {
    /// Open (or focus) the preview for one local path.
    fn open(&self, path: &str) -> Result<(), String>;
    /// Close the window this provider owns. Must be safe to call repeatedly.
    fn close(&self) -> Result<(), String>;
}

/// Host clipboard provider for file copy/cut/paste. Only Windows has the
/// CF_HDROP + Preferred DropEffect backend; other hosts register no provider
/// and the neutral free functions degrade without panicking (7.24).
pub trait ClipboardProvider: Send + Sync {
    fn set(&self, paths: &[String], mode: &str) -> Result<(), String>;
    fn get(&self) -> Result<ShellClipboard, String>;
    fn clear(&self) -> Result<(), String>;
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
    /// File clipboard provider is registered (CF_HDROP on Windows). Derived
    /// from `clipboard_provider().is_some()`, never hand-declared (7.24).
    pub shell_clipboard: bool,
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
    /// macOS system Quick Look / Windows third-party QuickLook pipe is usable.
    pub quick_look_preview: bool,
    /// PowerToys Peek CLI is a valid preview backend (Windows only).
    pub peek_preview: bool,
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
    /// A synchronous OS permission pre-check exists for microphone capture
    /// (Windows ConsentStore). Derived from `microphone_permission_provider()`.
    /// macOS reports false: TCC asks at device-open time, so no pre-check can
    /// be performed before the stream starts.
    pub microphone_permission_precheck: bool,
    /// The managed-dependency feed ships native sidecar artifacts for this host.
    /// Both Windows and macOS opt in; each feed entry is tagged with
    /// `artifact_platform_key()` so a host downloads only its own binary.
    /// A host with no published artifact simply gets no managed binding.
    pub sidecar_native_artifacts: bool,
}

impl PlatformCapabilities {
    /// Everything defaults to unavailable; each adapter opts in explicitly.
    pub(crate) fn neutral(os: &str, primary_modifier: &str, separator: char) -> Self {
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
            shell_clipboard: false,
            native_open_with_dialog: false,
            native_context_menu: false,
            os_shell_verb: false,
            registry_autostart: false,
            native_autostart: false,
            single_instance: false,
            global_search: false,
            global_search_engine: "none".to_string(),
            quick_look_preview: false,
            peek_preview: false,
            ole_drag_out: false,
            native_drag_out: false,
            webview_drag_out: false,
            hidden_file_attributes: false,
            drive_letters: false,
            volume_list: false,
            system_tray: false,
            webview2_required: false,
            microphone_permission_precheck: false,
            sidecar_native_artifacts: false,
        }
    }

    /// Derive every provider-backed bit from the providers the adapter has
    /// actually registered (6.5 §4). Called at the end of each adapter's
    /// `capabilities()`, so a capability can never claim a feature whose
    /// provider is absent. Pure host facts (native dialogs, registry, WebView2)
    /// stay in the adapters; only provider-backed bits come from here.
    pub fn apply_provider_capabilities(&mut self, platform: &dyn Platform) {
        self.trash_delete = platform.trash_provider().is_some();
        self.trash_list_restore = platform
            .trash_provider()
            .is_some_and(|provider| provider.supports_list_restore());
        self.shell_clipboard = platform.clipboard_provider().is_some();
        self.global_search = platform.global_search_engine() != GlobalSearchEngine::None;
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
    /// ADR 0026: live computers from the OS "Network" folder (WS-Discovery on
    /// Windows). Default empty so non-Windows adapters opt out explicitly.
    fn network_computers(&self) -> Vec<String> {
        Vec::new()
    }
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
    ) -> Result<(), TerminalLaunchError>;
    fn launch_ide(
        &self,
        target: &LaunchTarget,
        path: &std::path::Path,
        extra_args: &[String],
    ) -> Result<(), String>;
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
    /// Default Node interpreter for shared-runtime sidecars. On macOS a GUI
    /// launch does not inherit the shell PATH, so probe the standard Homebrew
    /// locations before falling back to the bare name (7.x follow-up).
    fn node_command(&self) -> &'static str;
    /// Grant execute permission on a downloaded sidecar artifact. Windows is a
    /// no-op (PE files need no exec bit); Unix adds owner/group/other exec.
    /// `Some(mode)` (from a zip entry's `unix_mode`) applies that exact mode
    /// so pack-provided scripts keep their executable bit across install.
    fn make_executable(&self, path: &std::path::Path, mode: Option<u32>) -> Result<(), String>;
    /// Native file-type icon in RGBA form, without leaking OS image types.
    fn file_icon(&self, path: &std::path::Path, size: u16) -> Result<RgbaIcon, String>;
    /// Native content thumbnail when the host exposes one; otherwise `Err` and
    /// callers fall back to `file_icon`.
    fn system_thumbnail(&self, path: &std::path::Path, size: u16) -> Result<RgbaIcon, String>;
    /// Registered file-trash provider (every adapter has one via the `trash`
    /// crate). Capability bits are derived from this, not declared by hand.
    fn trash_provider(&self) -> Option<&dyn TrashProvider>;
    /// Registered file-clipboard provider. `None` on hosts without CF_HDROP.
    fn clipboard_provider(&self) -> Option<&dyn ClipboardProvider>;
    /// Host-native preview backend, if one exists. Windows returns `None`:
    /// its QuickLook pipe / Peek CLI bridge lives in `preview_bridge` and is
    /// selected with the `quick_look_preview` / `peek_preview` capability bits.
    fn preview_provider(&self) -> Option<&dyn PreviewProvider>;
}

/// The provider backends. Their OS-specific bodies stay in `file_ops/*` —
/// 7.24 only moves the *entry point* into the platform layer; moving ~600 lines
/// of Win32/PowerShell code with no Windows compiler here would risk behavior
/// drift. Business code calls the neutral free functions below, never these
/// backends directly.
struct NativeTrash;

impl TrashProvider for NativeTrash {
    fn delete(&self, path: &std::path::Path, path_text: &str) -> Result<(), String> {
        crate::file_ops::delete_to_recycle(path, path_text)
    }

    fn list(&self) -> Result<Vec<RecycleBinItem>, String> {
        crate::file_ops::recycle::list_recycle_bin()
    }

    fn restore(&self, original_paths: &[String]) -> Result<Vec<String>, String> {
        crate::file_ops::recycle::restore_original_paths(original_paths)
    }

    fn supports_list_restore(&self) -> bool {
        cfg!(windows)
    }
}

#[cfg(windows)]
struct NativeClipboard;

#[cfg(windows)]
impl ClipboardProvider for NativeClipboard {
    fn set(&self, paths: &[String], mode: &str) -> Result<(), String> {
        crate::file_ops::clipboard::set_shell_clipboard(paths, mode)
    }

    fn get(&self) -> Result<ShellClipboard, String> {
        crate::file_ops::clipboard::get_shell_clipboard()
    }

    fn clear(&self) -> Result<(), String> {
        crate::file_ops::clipboard::clear_shell_clipboard()
    }
}

static NATIVE_TRASH: NativeTrash = NativeTrash;
#[cfg(windows)]
static NATIVE_CLIPBOARD: NativeClipboard = NativeClipboard;

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

/// Single entry point for live-computer enumeration (ADR 0026). Empty on hosts
/// whose adapter does not implement it.
pub fn network_computers() -> Vec<String> {
    current().network_computers()
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
) -> Result<(), TerminalLaunchError> {
    current().launch_terminal(target, dir, elevated)
}

pub fn launch_ide(
    target: &LaunchTarget,
    path: &std::path::Path,
    extra_args: &[String],
) -> Result<(), String> {
    current().launch_ide(target, path, extra_args)
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

pub fn node_command() -> &'static str {
    current().node_command()
}

/// Single entry point for marking a downloaded/provisioned artifact executable.
pub fn make_executable(path: &std::path::Path, mode: Option<u32>) -> Result<(), String> {
    current().make_executable(path, mode)
}

/// Canonical `<os>-<arch>` key for selecting a managed-feed sidecar artifact,
/// e.g. `windows-x86_64`, `macos-x86_64`, `macos-aarch64`.
///
/// The managed-dependency feed may publish several native builds of one
/// sidecar; each `StoreEntry` is tagged with this key so a host downloads only
/// its own binary. Keeping the format here (not in the feed/store layer)
/// ensures every consumer agrees on the spelling and no `#[cfg]` leaks into
/// business modules (7.24 / ADR 0022 macOS packaging).
pub fn artifact_platform_key() -> String {
    let caps = current().capabilities();
    format!("{}-{}", caps.os, caps.arch)
}

pub fn file_icon(path: &std::path::Path, size: u16) -> Result<RgbaIcon, String> {
    current().file_icon(path, size)
}

pub fn system_thumbnail(path: &std::path::Path, size: u16) -> Result<RgbaIcon, String> {
    current().system_thumbnail(path, size)
}

// 7.24: file trash / file clipboard entry points. Business modules call these
// instead of the OS backends in `file_ops/*`, so there is one place to gate on
// provider availability and one place for the shared provider tests.

/// Move `path` into the host trash through the registered provider.
pub fn delete_to_trash(path: &std::path::Path, path_text: &str) -> Result<(), String> {
    current()
        .trash_provider()
        .ok_or_else(|| "File trash is unavailable on this host".to_string())?
        .delete(path, path_text)
}

/// List Recycle Bin rows through the provider (`Err` when it has no bin API).
pub fn trash_list() -> Result<Vec<RecycleBinItem>, String> {
    current()
        .trash_provider()
        .ok_or_else(|| "File trash is unavailable on this host".to_string())?
        .list()
}

/// Restore rows by original path through the provider.
pub fn trash_restore(original_paths: &[String]) -> Result<Vec<String>, String> {
    current()
        .trash_provider()
        .ok_or_else(|| "File trash is unavailable on this host".to_string())?
        .restore(original_paths)
}

pub fn clipboard_set(paths: &[String], mode: &str) -> Result<(), String> {
    match current().clipboard_provider() {
        Some(provider) => provider.set(paths, mode),
        None => Err("Shell clipboard is only available on Windows".to_string()),
    }
}

pub fn clipboard_get() -> Result<ShellClipboard, String> {
    match current().clipboard_provider() {
        Some(provider) => provider.get(),
        None => Ok(ShellClipboard {
            paths: Vec::new(),
            mode: None,
        }),
    }
}

pub fn clipboard_clear() -> Result<(), String> {
    match current().clipboard_provider() {
        Some(provider) => provider.clear(),
        None => Ok(()),
    }
}

/// Host-owned preview backend (`None` on Windows). Business code uses this
/// single entry point instead of opening `#[cfg(target_os = "macos")]` branches.
pub fn preview_provider() -> Option<&'static dyn PreviewProvider> {
    current().preview_provider()
}

/// Open / close through the registered provider. Callers gate on
/// `capabilities().quick_look_preview && preview_provider().is_some()`.
pub fn preview_open(path: &str) -> Result<(), String> {
    preview_provider()
        .ok_or_else(|| "No host-native preview provider on this host".to_string())?
        .open(path)
}

pub fn preview_close() -> Result<(), String> {
    preview_provider()
        .ok_or_else(|| "No host-native preview provider on this host".to_string())?
        .close()
}

/// Shared provider test (6.5 §4 "OpenDAL-style shared tests"). Every adapter
/// runs the same assertions against its own instance, instead of duplicating
/// `#[cfg]`-specific capability assertions in each adapter's test module.
#[cfg(test)]
pub(crate) fn assert_provider_capability_contract(platform: &dyn Platform) {
    let caps = platform.capabilities();
    let trash = platform.trash_provider();
    let clipboard = platform.clipboard_provider();
    assert_eq!(caps.trash_delete, trash.is_some());
    assert_eq!(
        caps.trash_list_restore,
        trash.is_some_and(|provider| provider.supports_list_restore())
    );
    assert_eq!(caps.shell_clipboard, clipboard.is_some());
    assert_eq!(
        caps.global_search,
        platform.global_search_engine() != GlobalSearchEngine::None
    );
    // A missing clipboard provider must degrade, never panic or claim success.
    if clipboard.is_none() {
        assert!(crate::platform::clipboard_set(&[], "copy").is_err());
        assert!(crate::platform::clipboard_get().unwrap().paths.is_empty());
        assert!(crate::platform::clipboard_clear().is_ok());
    }
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
        assert!(!has_uri_scheme("/Users/demo"));
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

    #[test]
    fn artifact_platform_key_is_os_dash_arch() {
        let key = artifact_platform_key();
        let caps = current().capabilities();
        assert_eq!(key, format!("{}-{}", caps.os, caps.arch));
        assert!(key.contains('-'), "key must be <os>-<arch>: {key}");
        // Two hosts must never collide on one key.
        assert_ne!(key, format!("{}-{}", "other-os", caps.arch));
        #[cfg(target_os = "macos")]
        assert!(key.starts_with("macos-"), "{key}");
        #[cfg(windows)]
        assert!(key.starts_with("windows-"), "{key}");
    }

    /// Same shared contract also runs against each adapter in its own module.
    #[test]
    fn active_adapter_honours_the_provider_contract() {
        assert_provider_capability_contract(current());
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
