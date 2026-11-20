// SPDX-License-Identifier: GPL-3.0-or-later
//! macOS / Unix-like adapter (compiled for every non-Windows target).
//!
//! This is an honest capability declaration, not a claim of feature parity:
//! Windows-only integrations (Shell verb, Everything, Peek, HKCU Run) stay
//! `false` so the UI can hide them until a native implementation lands.

#[cfg(target_os = "macos")]
use super::SearchHit;
use super::{
    has_uri_scheme, ClipboardProvider, GlobalSearchEngine, LaunchTarget, Platform,
    PlatformCapabilities, PreviewProvider, RgbaIcon, SearchProbe, SearchQueryResult,
    TerminalLaunchError, TrashProvider, Volume, VolumeKind, VolumeSpace, WindowChrome,
};
use std::collections::{HashMap, HashSet};
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use sysinfo::{Pid, ProcessesToUpdate, System};

/// System Quick Look backend for `preview.externalProvider = quicklook|auto`.
/// `qlmanage -p` is the public Quick Look command-line front end: no private
/// framework symbols and no AppKit run loop are needed. The child is owned by
/// the provider so Space-toggle / selection-follow can close it deterministically.
#[cfg(target_os = "macos")]
struct NativeQuickLook {
    child: std::sync::Mutex<Option<std::process::Child>>,
}

#[cfg(target_os = "macos")]
impl PreviewProvider for NativeQuickLook {
    fn open(&self, path: &str) -> Result<(), String> {
        self.close()?;
        let exe = "/usr/bin/qlmanage";
        if !Path::new(exe).is_file() {
            return Err(format!("{exe} is unavailable on this macOS version"));
        }
        let child = std::process::Command::new(exe)
            .arg("-p")
            .arg(path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|err| format!("Failed to open Quick Look: {err}"))?;
        let mut guard = self
            .child
            .lock()
            .map_err(|_| "Quick Look session lock poisoned".to_string())?;
        *guard = Some(child);
        Ok(())
    }

    fn close(&self) -> Result<(), String> {
        let mut guard = self
            .child
            .lock()
            .map_err(|_| "Quick Look session lock poisoned".to_string())?;
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
static NATIVE_QUICK_LOOK: NativeQuickLook = NativeQuickLook {
    child: std::sync::Mutex::new(None),
};

pub struct MacosPlatform;

/**
 * Walk the PPID graph once, then collect a root's descendants depth-first.
 * Kept pure (no `System`, no signals) so the traversal is testable without
 * spawning sacrificial processes; `Pid` is a sysinfo newtype.
 */
fn collect_process_tree(children: &HashMap<Pid, Vec<Pid>>, root: Pid) -> Vec<Pid> {
    let mut seen = HashSet::new();
    let mut stack = vec![root];
    let mut order = Vec::new();
    while let Some(pid) = stack.pop() {
        if !seen.insert(pid) {
            continue;
        }
        if let Some(kids) = children.get(&pid) {
            stack.extend(kids.iter().copied());
        }
        order.push(pid);
    }
    order
}

/// Self-drawn caption height, mirrored from `--fluent-titlebar-height` in
/// `src/styles/fluent-tokens.css`. The native traffic lights are centred on
/// this row, not on AppKit's 28pt titlebar band (7.19).
#[cfg(target_os = "macos")]
const CAPTION_HEIGHT_PT: f64 = 40.0;

/// Vertically centre the three native window buttons on the self-drawn caption.
/// AppKit leaves them ~6pt above that line. The delta is derived from the
/// button's current window-relative centre instead of a fixed offset, so the
/// call converges (idempotent) and keeps working if a future macOS moves the
/// default buttons. Re-apply after AppKit resets them on resize / title change.
#[cfg(target_os = "macos")]
fn center_traffic_lights_on_caption(ns_window: &objc2_app_kit::NSWindow) {
    use objc2_app_kit::{NSView, NSWindowButton};

    let window_height = ns_window.frame().size.height;
    for kind in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        let Some(button) = ns_window.standardWindowButton(kind) else {
            continue;
        };
        let bounds = NSView::bounds(&button);
        let in_window = NSView::convertRect_toView(&button, bounds, None);
        let center_from_top = window_height - (in_window.origin.y + in_window.size.height / 2.0);
        let delta = CAPTION_HEIGHT_PT / 2.0 - center_from_top;
        if delta.abs() > 0.5 {
            let mut origin = NSView::frame(&button).origin;
            origin.y -= delta;
            NSView::setFrameOrigin(&button, origin);
        }
    }
}

/// Restore the native window geometry that `TitleBarStyle::Overlay` implies.
///
/// `tao::set_decorations(true)` rewrites the whole `NSWindow.styleMask` on a
/// dispatch_async task with a plain `Titled|Closable|Miniaturizable|Resizable`
/// mask. That drops `FullSizeContentView`: AppKit then keeps `contentView`
/// 28pt shorter than the window, so WKWebView — and with it the self-drawn
/// Fluent caption — starts below an empty native titlebar band (7.18).
/// Re-applying the bit grows `contentView` back to the full window frame;
/// wry's parent view is pinned to the new bounds because it was sized against
/// the shorter content view.
///
/// The traffic lights keep their native horizontal position; a second pass
/// centres them vertically on the 40pt caption (7.19). All values are absolute
/// or derived from the current frame, so re-applying after a resize (AppKit
/// re-lays out the content view and resets the buttons) is idempotent.
#[cfg(target_os = "macos")]
pub fn extend_content_under_titlebar(window: &tauri::WebviewWindow) {
    use objc2_app_kit::{NSView, NSWindow, NSWindowStyleMask};

    let Ok(ns_window) = window.ns_window() else {
        return;
    };
    unsafe {
        let w: &NSWindow = &*(ns_window as *const NSWindow);
        let mask = w.styleMask();
        if !mask.contains(NSWindowStyleMask::FullSizeContentView) {
            w.setStyleMask(mask | NSWindowStyleMask::FullSizeContentView);
        }
        if !w.titlebarAppearsTransparent() {
            w.setTitlebarAppearsTransparent(true);
        }
        if let (Ok(ns_view), Some(content_view)) = (window.ns_view(), w.contentView()) {
            let v: &NSView = &*(ns_view as *const NSView);
            let bounds = NSView::bounds(&content_view);
            let frame = NSView::frame(v);
            if frame.size.width != bounds.size.width || frame.size.height != bounds.size.height {
                NSView::setFrame(v, bounds);
            }
        }
        center_traffic_lights_on_caption(w);
    }
}

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
                                 // `trash_delete` / `trash_list_restore` / `shell_clipboard` are derived
                                 // from the registered providers by `apply_provider_capabilities` below.
        caps.system_tray = true; // Tauri TrayIconBuilder → NSStatusItem
                                 // 7.13: the AppKit drag-out command in `tauri-plugin-drag` aborts the
                                 // process on macOS, so `native_drag_out` stays false. The working
                                 // transport is the WebView-native HTML5 `text/uri-list` drag-out
                                 // (src/state/dnd/dragOut.ts + NavigatorView.vue).
        caps.webview_drag_out = true;
        // 7.16: volumes() lists `/` and `/Volumes/*`; there are no drive letters.
        caps.volume_list = true;
        // 7.21: Spotlight is a host-wide index just like Everything, but it is
        // macOS-only — Linux must keep reporting "none".
        #[cfg(target_os = "macos")]
        {
            caps.global_search_engine = "spotlight".to_string();
            caps.quick_look_preview = true;
        }
        // Provider-backed bits (trash / clipboard / search) come from the
        // registered providers, not hand-written booleans (7.24).
        caps.apply_provider_capabilities(self);
        // microphone_permission_precheck stays false: macOS asks for TCC
        // microphone access when the capture device opens, so there is no
        // synchronous ConsentStore-style check to run beforehand. The bundle
        // ships NSMicrophoneUsageDescription + the audio-input entitlement so
        // that prompt actually appears (see Info.plist / entitlements).
        // ADR 0022 macOS packaging: native sidecars are published per host and
        // selected by `artifact_platform_key()` (e.g. macos-aarch64). Opting in
        // lets macOS enter managed provisioning; a feed with no macOS artifact
        // still yields no binding and degrades to Config, and a Windows .exe is
        // never selected because feed lookup is platform-scoped.
        caps.sidecar_native_artifacts = true;
        caps
    }

    fn trash_provider(&self) -> Option<&dyn TrashProvider> {
        Some(&super::NATIVE_TRASH)
    }

    fn clipboard_provider(&self) -> Option<&dyn ClipboardProvider> {
        // No CF_HDROP / Preferred DropEffect backend on Unix. The neutral
        // `clipboard_*` free functions degrade without panicking (7.24).
        None
    }

    fn preview_provider(&self) -> Option<&dyn PreviewProvider> {
        #[cfg(target_os = "macos")]
        {
            Some(&NATIVE_QUICK_LOOK)
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
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

    /// 7.20: macOS has no `wt` / `cmd`; discover `.app` bundles instead and
    /// keep the Windows preset ids out of the macOS UI.
    fn terminal_targets(&self) -> Vec<LaunchTarget> {
        targets_from(TERMINAL_APPS, false)
    }

    fn ide_targets(&self) -> Vec<LaunchTarget> {
        targets_from(IDE_APPS, false)
    }

    fn configured_launch_target(&self, id: &str, path: &str) -> Option<LaunchTarget> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return None;
        }
        let candidate = Path::new(trimmed);
        // An `.app` is a directory, not a file — Windows' `is_file()` guard
        // would reject every manually configured macOS application.
        #[cfg(target_os = "macos")]
        let usable = candidate.is_file() || (candidate.is_dir() && trimmed.ends_with(".app"));
        #[cfg(not(target_os = "macos"))]
        let usable = candidate.is_file();
        if !usable {
            return None;
        }
        Some(LaunchTarget {
            id: id.to_string(),
            name: ide_name(id),
            resolved_path: Some(trimmed.to_string()),
            supports_elevation: false,
        })
    }

    fn launch_terminal(
        &self,
        target: &LaunchTarget,
        dir: &Path,
        _elevated: bool,
    ) -> Result<(), TerminalLaunchError> {
        #[cfg(target_os = "macos")]
        {
            let app = target
                .resolved_path
                .as_ref()
                .ok_or_else(|| {
                    TerminalLaunchError::Failed(format!(
                        "Terminal not available: {}",
                        target.id
                    ))
                })?;
            // Terminal.app / iTerm2 open a shell at the directory passed to
            // `open -a`. There is no UAC equivalent on macOS, so `elevated`
            // is intentionally ignored (supports_elevation = false).
            std::process::Command::new("/usr/bin/open")
                .arg("-a")
                .arg(app)
                .arg(dir)
                .spawn()
                .map(|_| ())
                .map_err(|err| {
                    TerminalLaunchError::Failed(format!(
                        "Could not launch {} for {}: {err}",
                        target.name,
                        dir.display()
                    ))
                })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (target, dir);
            Err(TerminalLaunchError::Failed(
                "Terminal launch is only supported on macOS".to_string(),
            ))
        }
    }

    fn launch_ide(
        &self,
        target: &LaunchTarget,
        path: &Path,
        extra_args: &[String],
    ) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            let exe = target
                .resolved_path
                .as_ref()
                .ok_or_else(|| format!("IDE not found: {}", target.id))?;
            let mut cmd = if exe.ends_with(".app") {
                // `.app` bundles open the path as a document; CLI-only project
                // flags do not translate, so they are intentionally dropped.
                let mut open = std::process::Command::new("/usr/bin/open");
                open.arg("-a").arg(exe);
                open
            } else {
                // PATH CLI shims (`code`, `cursor`, …): `[flags] <path>`.
                let mut cli = std::process::Command::new(exe);
                cli.args(extra_args);
                cli
            };
            cmd.arg(path)
                .spawn()
                .map(|_| ())
                .map_err(|err| format!("Failed to launch IDE: {err}"))
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (target, path, extra_args);
            Err("IDE launch is only supported on macOS".to_string())
        }
    }

    fn kill_process_tree(&self, pid: u32) {
        // sysinfo's `system` feature reuses the process table already present
        // on macOS/Linux; no `taskkill` equivalent or shelling out needed.
        let Ok(mut sys) = System::new_all() else {
            return;
        };
        sys.refresh_processes(ProcessesToUpdate::All, false);
        let mut children: HashMap<Pid, Vec<Pid>> = HashMap::new();
        for (child, process) in sys.processes() {
            if let Some(parent) = process.parent() {
                children.entry(parent).or_default().push(*child);
            }
        }
        let tree = collect_process_tree(&children, Pid::from_u32(pid));
        // Leaf-first: killing a parent first can reparent/orphan its children.
        for target in tree.into_iter().rev() {
            if let Some(process) = sys.processes().get(&target) {
                let _ = process.kill();
            }
        }
    }

    fn python_command(&self) -> &'static str {
        // macOS ships `python3`; bare `python` may be absent or Python 2.
        "python3"
    }

    fn node_command(&self) -> &'static str {
        // A GUI launch from Finder does not inherit the shell PATH. Node is
        // usually installed under Homebrew, so probe the standard prefixes;
        // fall back to the bare name for nvm/fnm setups that add it to PATH.
        for candidate in ["/opt/homebrew/bin/node", "/usr/local/bin/node"] {
            if Path::new(candidate).is_file() {
                return candidate;
            }
        }
        "node"
    }

    fn make_executable(&self, path: &Path, mode: Option<u32>) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)
            .map_err(|err| format!("stat for chmod {}: {err}", path.display()))?
            .permissions();
        let mode = match mode {
            // Zip entries carry the original Unix mode; keep it verbatim.
            Some(raw) => raw & 0o7777,
            // Downloaded artifacts: add execute bits for owner/group/other,
            // preserving whatever read/write bits the writer produced.
            None => permissions.mode() | 0o111,
        };
        permissions.set_mode(mode);
        std::fs::set_permissions(path, permissions)
            .map_err(|err| format!("chmod {}: {err}", path.display()))
    }

    fn file_icon(&self, path: &Path, size: u16) -> Result<RgbaIcon, String> {
        // `file_icon_provider` wraps NSWorkspace (UTI) on macOS and the
        // freedesktop icon theme elsewhere; no ObjC leaks out of this adapter.
        let icon = file_icon_provider::get_file_icon(path, size)
            .map_err(|err| format!("file icon failed: {err:?}"))?;
        Ok(RgbaIcon {
            width: icon.width,
            height: icon.height,
            pixels: icon.pixels,
        })
    }

    fn system_thumbnail(&self, _path: &Path, _size: u16) -> Result<RgbaIcon, String> {
        // Deliberate downgrade: qlmanage failed with `sandbox initialization
        // failed` on the dev host (7.23), and QLThumbnailGenerator needs an
        // async ObjC run loop. Callers fall back to the NSWorkspace type icon.
        Err("System thumbnails are unavailable on macOS; using the file-type icon".to_string())
    }

    fn global_search_engine(&self) -> GlobalSearchEngine {
        #[cfg(target_os = "macos")]
        {
            GlobalSearchEngine::Spotlight
        }
        #[cfg(not(target_os = "macos"))]
        {
            GlobalSearchEngine::None
        }
    }

    fn probe_global_search(&self) -> SearchProbe {
        #[cfg(target_os = "macos")]
        {
            let available = Path::new(MDFIND_PATH).is_file();
            SearchProbe {
                available,
                detail: if available {
                    "Spotlight index available (/usr/bin/mdfind)".to_string()
                } else {
                    "Spotlight CLI /usr/bin/mdfind not found".to_string()
                },
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            SearchProbe {
                available: false,
                detail: "No host-wide index on this Unix target".to_string(),
            }
        }
    }

    fn global_search(
        &self,
        query: &str,
        max_results: u32,
        offset: u32,
    ) -> Result<SearchQueryResult, String> {
        #[cfg(target_os = "macos")]
        {
            let query = query.trim();
            // `mdfind -name ""` prints "Failed to create query for ''" yet
            // exits 0. The empty query is only used by Everything's totitems
            // probe; Spotlight has no cheap equivalent, so report zero.
            if query.is_empty() {
                return Ok(SearchQueryResult {
                    total_count: 0,
                    offset: offset as u64,
                    hits: Vec::new(),
                });
            }
            let name_query = spotlight_name_query(query);
            let output = std::process::Command::new(MDFIND_PATH)
                .arg("-0")
                .arg("-name")
                .arg(&name_query)
                .output()
                .map_err(|err| format!("Failed to run mdfind: {err}"))?;
            if !output.status.success() {
                return Err(format!(
                    "mdfind failed ({}): {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            Ok(parse_mdfind_output(&output.stdout, max_results, offset))
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (query, max_results, offset);
            Err("Spotlight search is only supported on macOS".to_string())
        }
    }
}

/// 7.21: `/usr/bin/mdfind` is the command-line front end of the same Spotlight
/// index Finder uses; `NSMetadataQuery` would need AppKit + a run loop for no
/// extra coverage here. Reuse the OS index instead of building a self-index.
#[cfg(target_os = "macos")]
const MDFIND_PATH: &str = "/usr/bin/mdfind";

/// `-name` still parses its argument as a Spotlight query expression: a bare
/// `a(b)!?` matches unrelated files. Wrapping the term in quotes makes the
/// whole user input a literal phrase (verified on macOS 15.x), while escaping
/// `\` and `"` keeps the user from breaking out of that phrase.
#[cfg(target_os = "macos")]
fn spotlight_name_query(query: &str) -> String {
    let escaped = query.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// `-0` prints NUL-separated absolute paths, so filenames containing newlines
/// survive. `mdfind` has no paging flags; slice the full result the same way
/// Everything's offset/max_results contract does.
#[cfg(target_os = "macos")]
fn parse_mdfind_output(raw: &[u8], max_results: u32, offset: u32) -> SearchQueryResult {
    let paths: Vec<String> = String::from_utf8_lossy(raw)
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect();
    let total_count = paths.len() as u64;
    let hits: Vec<SearchHit> = paths
        .into_iter()
        .skip(offset as usize)
        .take(max_results as usize)
        .map(|path| {
            let entry = Path::new(&path);
            let name = entry
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone());
            SearchHit {
                name,
                is_dir: entry.is_dir(),
                path,
            }
        })
        .collect();
    SearchQueryResult {
        total_count,
        offset: offset as u64,
        hits,
    }
}

/// 7.20: macOS launcher presets. `~` is expanded to `$HOME`; bare names are
/// resolved through `PATH` (the same helper the Windows adapter uses).
const TERMINAL_APPS: &[(&str, &str, &[&str])] = &[
    (
        "terminal",
        "Terminal",
        &[
            "/System/Applications/Utilities/Terminal.app",
            "/Applications/Utilities/Terminal.app",
        ],
    ),
    (
        "iterm",
        "iTerm2",
        &[
            "/Applications/iTerm.app",
            "/Applications/iTerm2.app",
            "~/Applications/iTerm.app",
        ],
    ),
    (
        "warp",
        "Warp",
        &["/Applications/Warp.app", "~/Applications/Warp.app"],
    ),
];

const IDE_APPS: &[(&str, &str, &[&str])] = &[
    (
        "vscode",
        "Visual Studio Code",
        &[
            "/Applications/Visual Studio Code.app",
            "~/Applications/Visual Studio Code.app",
            "code",
        ],
    ),
    (
        "cursor",
        "Cursor",
        &[
            "/Applications/Cursor.app",
            "~/Applications/Cursor.app",
            "cursor",
        ],
    ),
    (
        "trae",
        "Trae",
        &["/Applications/Trae.app", "~/Applications/Trae.app", "trae"],
    ),
    (
        "qoder",
        "Qoder CN",
        &[
            "/Applications/Qoder CN.app",
            "/Applications/Coder CN.app",
            "/Applications/Corder CN.app",
            "~/Applications/Qoder CN.app",
            "qoder",
        ],
    ),
    (
        "qwenwork",
        "QwenWork",
        &[
            "/Applications/QwenWork.app",
            "~/Applications/QwenWork.app",
            "qwenwork",
        ],
    ),
];

fn ide_name(id: &str) -> String {
    IDE_APPS
        .iter()
        .find(|(ide_id, _, _)| *ide_id == id)
        .map(|(_, name, _)| (*name).to_string())
        .unwrap_or_else(|| id.to_string())
}

fn targets_from(defs: &[(&str, &str, &[&str])], supports_elevation: bool) -> Vec<LaunchTarget> {
    defs.iter()
        .map(|(id, name, candidates)| LaunchTarget {
            id: (*id).to_string(),
            name: (*name).to_string(),
            resolved_path: resolve_candidates(candidates),
            supports_elevation,
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn expand_home(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(rest)
                .to_string_lossy()
                .into_owned();
        }
    }
    value.to_string()
}

#[cfg(target_os = "macos")]
fn resolve_candidates(candidates: &[&str]) -> Option<String> {
    candidates.iter().find_map(|candidate| {
        let expanded = expand_home(candidate);
        let probe = Path::new(&expanded);
        if expanded.ends_with(".app") {
            return probe.is_dir().then_some(expanded);
        }
        crate::win_process::resolve_on_path(candidate).map(|p| p.to_string_lossy().into_owned())
    })
}

/// Linux reuses this adapter but has no `/usr/bin/open`; report presets as
/// unavailable instead of pretending the launch would work.
#[cfg(not(target_os = "macos"))]
fn resolve_candidates(_candidates: &[&str]) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_adapter_honours_the_shared_provider_contract() {
        super::super::assert_provider_capability_contract(&MacosPlatform);
    }

    #[test]
    fn unix_paths_are_not_rewritten() {
        let p = MacosPlatform;
        assert_eq!(
            p.to_native_fs_path("/Users/demo/Docs/"),
            "/Users/demo/Docs"
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
        // trashListRestore is now checked against the registered provider by
        // `assert_provider_capability_contract`, not duplicated here (7.24).
        assert!(!caps.system_thumbnails);
        assert!(!caps.webview2_required);
        assert_eq!(caps.path_separator, "/");
        #[cfg(target_os = "macos")]
        assert_eq!(caps.primary_modifier, "cmd");
    }

    #[test]
    fn launch_targets_are_neutral_and_elevation_free() {
        let terminals = MacosPlatform.terminal_targets();
        assert!(terminals.iter().any(|t| t.id == "terminal"));
        assert!(terminals.iter().all(|t| !t.supports_elevation));
        assert!(terminals.iter().all(|t| t
            .resolved_path
            .as_deref()
            .map_or(true, |p| !p.contains(":\\"))));

        let ides = MacosPlatform.ide_targets();
        let ids: Vec<_> = ides.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["vscode", "cursor", "trae", "qoder", "qwenwork"]);
        assert!(ides.iter().all(|t| !t.supports_elevation));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn discovery_only_reports_apps_that_really_exist() {
        let terminals = MacosPlatform.terminal_targets();
        assert!(!terminals.iter().any(|t| t.id == "wt" || t.id == "cmd"));
        if Path::new("/System/Applications/Utilities/Terminal.app").is_dir() {
            let system_terminal = terminals.iter().find(|t| t.id == "terminal").unwrap();
            let resolved = system_terminal.resolved_path.as_deref().unwrap();
            assert!(Path::new(resolved).is_dir());
            assert!(!resolved.contains('\\'));
        }
        for target in MacosPlatform.ide_targets() {
            if let Some(path) = target.resolved_path.as_deref() {
                assert!(Path::new(path).exists(), "{} resolved to {path}", target.id);
                assert!(!path.contains("C:\\"), "Windows preset leaked into macOS");
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn spotlight_name_is_a_literal_quoted_phrase() {
        // A bare `a(b)!?` is parsed by mdfind as query syntax and matches
        // unrelated files; wrapping it in quotes keeps it literal.
        assert_eq!(spotlight_name_query("readme"), "\"readme\"");
        assert_eq!(spotlight_name_query("a(b)!?"), "\"a(b)!?\"");
        assert_eq!(spotlight_name_query("a\"b"), "\"a\\\"b\"");
        assert_eq!(spotlight_name_query("a\\b"), "\"a\\\\b\"");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn spotlight_output_pages_nul_paths_and_detects_directories() {
        let dir = std::env::temp_dir();
        let root = dir.to_string_lossy().trim_end_matches('/').to_string();
        let raw = format!("{root}/a.txt\0{root}\0{root}/b.txt\0");
        let result = parse_mdfind_output(raw.as_bytes(), 2, 0);
        assert_eq!(result.total_count, 3);
        assert_eq!(result.offset, 0);
        assert_eq!(result.hits.len(), 2);
        assert_eq!(result.hits[0].name, "a.txt");
        assert!(result.hits[1].is_dir, "temp dir is a real directory");

        let page = parse_mdfind_output(raw.as_bytes(), 1, 2);
        assert_eq!(page.offset, 2);
        assert_eq!(page.hits[0].name, "b.txt");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn spotlight_probe_and_empty_query_are_safe() {
        assert!(Path::new(MDFIND_PATH).is_file());
        assert_eq!(
            MacosPlatform.global_search_engine(),
            GlobalSearchEngine::Spotlight
        );
        assert!(MacosPlatform.probe_global_search().available);
        // `mdfind -name ""` fails to create a query yet exits 0; short-circuit.
        let empty = MacosPlatform.global_search("", 5, 0).unwrap();
        assert_eq!(empty.total_count, 0);
        assert!(empty.hits.is_empty());
    }

    /// Manual live check: `cargo test --lib live_spotlight_query -- --ignored`.
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires Spotlight indexing enabled on the host"]
    fn live_spotlight_query_returns_absolute_paths() {
        let result = MacosPlatform.global_search("Info.plist", 5, 0).unwrap();
        assert!(
            !result.hits.is_empty(),
            "Spotlight returned no Info.plist hits"
        );
        assert!(result.hits.iter().all(|hit| hit.path.starts_with('/')));
        assert_eq!(result.hits[0].name, "Info.plist");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn configured_app_bundle_is_accepted_as_a_directory() {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let app = std::env::temp_dir().join(format!("afm-launch-{stamp}/Fixture.app"));
        fs::create_dir_all(&app).unwrap();
        let raw = app.to_str().unwrap();

        let target = MacosPlatform
            .configured_launch_target("cursor", raw)
            .expect("an .app bundle is a valid macOS launcher");
        assert_eq!(target.resolved_path.as_deref(), Some(raw));
        assert!(MacosPlatform
            .configured_launch_target("cursor", &format!("{raw}/missing.app"))
            .is_none());

        let _ = fs::remove_dir_all(std::env::temp_dir().join(format!("afm-launch-{stamp}")));
    }

    #[test]
    fn python_command_is_python3_on_unix() {
        assert_eq!(MacosPlatform.python_command(), "python3");
    }

    #[test]
    fn node_command_resolves_to_an_existing_binary_or_a_bare_name() {
        let node = MacosPlatform.node_command();
        // Either a probed absolute Homebrew path that really exists, or the
        // bare name fallback when Node is managed via nvm/fnm on PATH.
        assert!(
            node == "node"
                || (node.starts_with('/') && Path::new(node).is_file()),
            "unexpected node command: {node}"
        );
    }

    #[test]
    fn make_executable_adds_exec_bits_and_honours_zip_mode() {
        use std::fs;
        let dir = std::env::temp_dir().join(format!("afm-chmod-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("sidecar");
        fs::write(&file, b"#!/bin/sh\n").unwrap();

        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
        MacosPlatform.make_executable(&file, None).unwrap();
        assert_eq!(fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o755);

        // A zip entry with an explicit mode (e.g. 0o700) is applied verbatim.
        MacosPlatform.make_executable(&file, Some(0o700)).unwrap();
        assert_eq!(fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o700);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn post_port_host_facts_are_honest_on_macos() {
        let caps = MacosPlatform.capabilities();
        // macOS asks via TCC at device-open time: no synchronous pre-check.
        assert!(!caps.microphone_permission_precheck);
        // macOS provisions native sidecars; platform-scoped feed lookup keeps
        // it from ever selecting the Windows .exe.
        assert!(caps.sidecar_native_artifacts);
    }

    #[test]
    fn process_tree_collection_is_cycle_safe_and_includes_descendants() {
        let mut children: HashMap<Pid, Vec<Pid>> = HashMap::new();
        children.insert(
            Pid::from_u32(10),
            vec![Pid::from_u32(11), Pid::from_u32(12)],
        );
        children.insert(Pid::from_u32(11), vec![Pid::from_u32(13)]);
        children.insert(Pid::from_u32(13), vec![Pid::from_u32(10)]); // cycle guard
        let tree = collect_process_tree(&children, Pid::from_u32(10));
        assert_eq!(tree.len(), 4, "each pid visited exactly once");
        assert_eq!(tree[0].as_u32(), 10, "root must come first (killed last)");
        let ids: HashSet<u32> = tree.iter().map(|pid| pid.as_u32()).collect();
        assert_eq!(ids, HashSet::from([10, 11, 12, 13]));
    }

    #[test]
    fn system_thumbnail_downgrade_is_explicit() {
        let err = MacosPlatform
            .system_thumbnail(Path::new("/tmp/afm-no-such-file"), 32)
            .unwrap_err();
        assert!(err.contains("unavailable"), "{err}");
    }
}
