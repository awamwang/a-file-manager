// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows adapter. Semantics are identical to the pre-port code: native `\`,
//! drive letters / UNC, case-insensitive comparison, HKCU Run, Win32 icons.

use super::{
    has_uri_scheme, ClipboardProvider, GlobalSearchEngine, LaunchTarget, Platform,
    PlatformCapabilities, PreviewProvider, RgbaIcon, SearchProbe, SearchQueryResult,
    TerminalLaunchError, TrashProvider, Volume, VolumeKind, VolumeSpace, WindowChrome,
};
use std::ffi::c_void;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use windows::{
    core::HSTRING,
    Win32::Foundation::SIZE,
    Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
    },
    Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED},
    Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK,
        SIIGBF_INCACHEONLY, SIIGBF_RESIZETOFIT, SIIGBF_THUMBNAILONLY,
    },
};

pub struct WindowsPlatform;

/// COM is thread-apartment scoped; do not CoUninitialize (the host may own it).
fn ensure_com_sta() {
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
}

fn hbitmap_to_rgba(hbitmap: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    unsafe {
        let mut bmp: BITMAP = std::mem::zeroed();
        if GetObjectW(
            hbitmap.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some((&raw mut bmp).cast::<c_void>()),
        ) == 0
        {
            let _ = DeleteObject(hbitmap.into());
            return None;
        }

        let width = bmp.bmWidth.max(0) as u32;
        let height = bmp.bmHeight.max(0) as u32;
        if width == 0 || height == 0 {
            let _ = DeleteObject(hbitmap.into());
            return None;
        }

        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = bmp.bmWidth;
        bi.bmiHeader.biHeight = -bmp.bmHeight;
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB.0;

        let mut pixels_bgra = vec![0u8; (width * height * 4) as usize];
        let hdc = CreateCompatibleDC(None);
        if hdc.is_invalid() {
            let _ = DeleteObject(hbitmap.into());
            return None;
        }
        let lines = GetDIBits(
            hdc,
            hbitmap,
            0,
            height,
            Some(pixels_bgra.as_mut_ptr().cast::<c_void>()),
            &mut bi,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(hdc);
        let _ = DeleteObject(hbitmap.into());
        if lines == 0 {
            return None;
        }

        for chunk in pixels_bgra.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }
        Some((width, height, pixels_bgra))
    }
}

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
        caps.native_open_with_dialog = true;
        caps.native_context_menu = true;
        caps.os_shell_verb = true;
        caps.registry_autostart = true;
        caps.single_instance = true;
        // Provider-backed bits (trash / clipboard / search) come from the
        // registered providers, not hand-written booleans (7.24).
        caps.apply_provider_capabilities(self);
        caps.global_search_engine = "everything".to_string();
        caps.quick_look_preview = true;
        caps.peek_preview = true;
        caps.ole_drag_out = true;
        caps.hidden_file_attributes = true;
        caps.drive_letters = true;
        caps.volume_list = true; // volumes() enumerates the drive-letter bitmap
        caps.system_tray = true;
        caps.webview2_required = true;
        // Windows exposes a synchronous ConsentStore pre-check for the
        // microphone and the managed feed publishes native .exe sidecars.
        caps.microphone_permission_precheck = true;
        caps.sidecar_native_artifacts = true;
        caps
    }

    fn trash_provider(&self) -> Option<&dyn TrashProvider> {
        Some(&super::NATIVE_TRASH)
    }

    fn clipboard_provider(&self) -> Option<&dyn ClipboardProvider> {
        Some(&super::NATIVE_CLIPBOARD)
    }

    fn preview_provider(&self) -> Option<&dyn PreviewProvider> {
        // The Windows bridge talks to resident QuickLook via a named pipe and
        // spawns Peek itself; there is no single host-owned preview process.
        None
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

    /// ADR 0026: live computers from the Shell Network folder (WS-Discovery),
    /// the same source Explorer's 网络 pane reads. Delegates to the module-level
    /// `network_computers` COM helper.
    fn network_computers(&self) -> Vec<String> {
        network_computers()
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

    /// Moved from `terminal/mod.rs` (7.20): all five presets are returned so
    /// the Settings / context menu can still show unavailable rows.
    fn terminal_targets(&self) -> Vec<LaunchTarget> {
        TERMINAL_DEFS
            .iter()
            .map(|(id, name)| LaunchTarget {
                id: (*id).to_string(),
                name: (*name).to_string(),
                resolved_path: resolve_terminal_exe(id).map(|p| p.to_string_lossy().into_owned()),
                supports_elevation: true,
            })
            .collect()
    }

    /// Moved from `ide_launch/mod.rs` (7.20).
    fn ide_targets(&self) -> Vec<LaunchTarget> {
        IDE_DEFS
            .iter()
            .map(|(id, name, _)| LaunchTarget {
                id: (*id).to_string(),
                name: (*name).to_string(),
                resolved_path: resolve_auto(id).map(|p| p.to_string_lossy().into_owned()),
                supports_elevation: false,
            })
            .collect()
    }

    fn configured_launch_target(&self, id: &str, path: &str) -> Option<LaunchTarget> {
        let trimmed = path.trim();
        if trimmed.is_empty() || !Path::new(trimmed).is_file() {
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
        elevated: bool,
    ) -> Result<(), TerminalLaunchError> {
        let exe = target
            .resolved_path
            .as_ref()
            .map(PathBuf::from)
            .ok_or_else(|| {
                TerminalLaunchError::Failed(format!("Terminal not available: {}", target.id))
            })?;
        let dir_str = dir.to_string_lossy().replace('/', "\\");
        let args = terminal_args(&target.id, &dir_str);
        // Terminal UIs must remain visible — do not apply CREATE_NO_WINDOW here.
        let launched = if elevated {
            match spawn_elevated(&exe, &args, dir) {
                ElevateOutcome::Launched => true,
                ElevateOutcome::Cancelled => return Err(TerminalLaunchError::Cancelled),
                ElevateOutcome::Failed => false,
            }
        } else {
            std::process::Command::new(&exe)
                .args(&args)
                .current_dir(dir)
                .spawn()
                .is_ok()
        };
        if launched {
            Ok(())
        } else {
            Err(TerminalLaunchError::Failed(format!(
                "Could not launch {} for {}",
                target.name,
                dir.display()
            )))
        }
    }

    fn launch_ide(
        &self,
        target: &LaunchTarget,
        path: &Path,
        extra_args: &[String],
    ) -> Result<(), String> {
        let exe = target
            .resolved_path
            .as_ref()
            .ok_or_else(|| format!("IDE not found: {}", target.id))?;
        // .cmd/.bat shims (code.cmd, cursor.cmd, …) are console hosts — hide them.
        // Project-specific flags precede the target path (`prog [flags] <path>`).
        let mut cmd = crate::win_process::command(exe);
        cmd.args(extra_args);
        cmd.arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|err| format!("Failed to launch IDE: {err}"))
    }

    fn kill_process_tree(&self, pid: u32) {
        // `/T` kills descendants too; CREATE_NO_WINDOW keeps the console hidden.
        let _ = crate::win_process::command("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    fn python_command(&self) -> &'static str {
        "python"
    }

    fn node_command(&self) -> &'static str {
        // Windows installers put node.exe on PATH; CREATE_NO_WINDOW is applied
        // by the shared spawn helper in tool_runtime::sidecar.
        "node"
    }

    fn make_executable(&self, _path: &Path, _mode: Option<u32>) -> Result<(), String> {
        // Windows has no execute bit; PE artifacts need no chmod.
        Ok(())
    }

    fn file_icon(&self, path: &Path, size: u16) -> Result<RgbaIcon, String> {
        ensure_com_sta();
        let icon = file_icon_provider::get_file_icon(path, size)
            .map_err(|err| format!("shell icon failed: {err:?}"))?;
        Ok(RgbaIcon {
            width: icon.width,
            height: icon.height,
            pixels: icon.pixels,
        })
    }

    /// Prefer Explorer's thumbnail cache / installed handlers (AIA-143), then
    /// let callers fall back to `file_icon`. Never self-decode file bytes.
    fn system_thumbnail(&self, path: &Path, size: u16) -> Result<RgbaIcon, String> {
        let hstring = HSTRING::from(path.as_os_str());
        let edge = i32::from(size);
        let sz = SIZE { cx: edge, cy: edge };

        unsafe {
            ensure_com_sta();
            let factory: IShellItemImageFactory = SHCreateItemFromParsingName(&hstring, None)
                .map_err(|err| format!("SHCreateItemFromParsingName: {err}"))?;
            let attempts = [
                SIIGBF_INCACHEONLY | SIIGBF_THUMBNAILONLY | SIIGBF_RESIZETOFIT,
                SIIGBF_THUMBNAILONLY | SIIGBF_BIGGERSIZEOK | SIIGBF_RESIZETOFIT,
            ];
            let mut last_err = String::from("no thumbnail");
            for flags in attempts {
                match factory.GetImage(sz, flags) {
                    Ok(hbmp) => {
                        if let Some((w, h, pixels)) = hbitmap_to_rgba(hbmp) {
                            return Ok(RgbaIcon {
                                width: w,
                                height: h,
                                pixels,
                            });
                        }
                        last_err = "HBITMAP convert failed".into();
                    }
                    Err(err) => last_err = format!("GetImage: {err}"),
                }
            }
            Err(last_err)
        }
    }

    fn global_search_engine(&self) -> GlobalSearchEngine {
        // Everything IPC stays in `discovery/everything_ipc.rs`; this adapter
        // only declares the choice, it does not implement a second index.
        GlobalSearchEngine::EverythingIpc
    }

    fn probe_global_search(&self) -> SearchProbe {
        SearchProbe {
            available: true,
            detail: "Everything IPC backend (see discovery::everything_ipc)".to_string(),
        }
    }

    fn global_search(
        &self,
        _query: &str,
        _max_results: u32,
        _offset: u32,
    ) -> Result<SearchQueryResult, String> {
        Err(
            "Windows global search goes through Everything IPC, not the platform adapter"
                .to_string(),
        )
    }
}

/// 7.20: moved verbatim from `terminal/mod.rs` / `ide_launch/mod.rs`. No new
/// Windows behaviour — the business modules now only map neutral targets.
const TERMINAL_DEFS: &[(&str, &str)] = &[
    ("wt", "Windows Terminal"),
    ("powershell", "Windows PowerShell"),
    ("pwsh", "PowerShell"),
    ("cmd", "Command Prompt"),
    ("gitbash", "Git Bash"),
];

/// Preset IDE/Agent launchers. Candidates mirror Sigma-style discovery:
/// PATH shims first, then per-user Local\Programs, then Program Files.
/// Qoder CN also accepts Coder CN / Corder CN install-folder spellings.
const IDE_DEFS: &[(&str, &str, &[&str])] = &[
    (
        "vscode",
        "Visual Studio Code",
        &[
            "code.cmd",
            "code",
            r"C:\Users\{user}\AppData\Local\Programs\Microsoft VS Code\Code.exe",
            r"C:\Program Files\Microsoft VS Code\Code.exe",
        ],
    ),
    (
        "cursor",
        "Cursor",
        &[
            "cursor.cmd",
            "cursor",
            r"C:\Users\{user}\AppData\Local\Programs\cursor\Cursor.exe",
            r"C:\Program Files\Cursor\Cursor.exe",
        ],
    ),
    (
        "trae",
        "Trae",
        &[
            "trae.cmd",
            "trae",
            r"C:\Users\{user}\AppData\Local\Programs\Trae\Trae.exe",
            r"C:\Program Files\Trae\Trae.exe",
        ],
    ),
    (
        "qoder",
        "Qoder CN",
        &[
            "qoder.cmd",
            "qoder",
            "coder.cmd",
            "corder.cmd",
            r"C:\Users\{user}\AppData\Local\Programs\Qoder CN\Qoder CN.exe",
            r"C:\Users\{user}\AppData\Local\Programs\Coder CN\Coder CN.exe",
            r"C:\Users\{user}\AppData\Local\Programs\Corder CN\Corder CN.exe",
            r"C:\Users\{user}\AppData\Local\Programs\Qoder\Qoder.exe",
            r"C:\Program Files\Qoder CN\Qoder CN.exe",
            r"C:\Program Files\Coder CN\Coder CN.exe",
            r"C:\Program Files\Corder CN\Corder CN.exe",
        ],
    ),
    (
        // QwenWork installs into a versioned folder (C:\Program Files\QwenWorkCN\<ver>\);
        // the candidate list only covers an optional PATH shim. The version scan in
        // `resolve_auto` handles the real per-version executable.
        "qwenwork",
        "QwenWork",
        &["qwenwork.cmd", "qwenwork"],
    ),
];

fn ide_name(id: &str) -> String {
    IDE_DEFS
        .iter()
        .find(|(ide_id, _, _)| *ide_id == id)
        .map(|(_, name, _)| (*name).to_string())
        .unwrap_or_else(|| id.to_string())
}

fn expand_user(path: &str) -> String {
    if let Ok(user) = std::env::var("USERNAME") {
        return path.replace("{user}", &user);
    }
    path.to_string()
}

/// PATH lookup without spawning where.exe (avoids console flashes on GUI builds).
fn resolve_candidate(candidate: &str) -> Option<PathBuf> {
    let expanded = expand_user(candidate);
    crate::win_process::resolve_on_path(&expanded)
}

fn resolve_terminal_exe(id: &str) -> Option<PathBuf> {
    match id {
        "wt" => resolve_candidate("wt.exe").or_else(|| resolve_candidate("wt")),
        "powershell" => resolve_candidate("powershell.exe"),
        "pwsh" => resolve_candidate("pwsh.exe").or_else(|| {
            resolve_candidate(r"C:\Program Files\PowerShell\7\pwsh.exe")
                .or_else(|| resolve_candidate(r"C:\Program Files\PowerShell\7-preview\pwsh.exe"))
        }),
        "cmd" => resolve_candidate("cmd.exe"),
        "gitbash" => [
            r"C:\Program Files\Git\git-bash.exe",
            r"C:\Program Files (x86)\Git\git-bash.exe",
            r"C:\Users\{user}\AppData\Local\Programs\Git\git-bash.exe",
        ]
        .into_iter()
        .find_map(resolve_candidate),
        _ => None,
    }
}

fn resolve_auto(id: &str) -> Option<PathBuf> {
    let def = IDE_DEFS.iter().find(|(ide_id, _, _)| *ide_id == id)?;
    let from_candidates = def.2.iter().find_map(|candidate| resolve_candidate(candidate));
    if id == "qwenwork" {
        // Prefer an explicit PATH shim, else the newest versioned install.
        return from_candidates.or_else(resolve_qwenwork_exe);
    }
    from_candidates
}

/// QwenWork install root — `C:\Program Files\QwenWorkCN` (or the relocated
/// `PROGRAMFILES` sibling). The optional override lets a non-default install be
/// discovered without a configured path.
fn qwenwork_install_base() -> Option<PathBuf> {
    let base = match std::env::var("PROGRAMFILES") {
        Ok(root) if !root.trim().is_empty() => PathBuf::from(root).join("QwenWorkCN"),
        _ => PathBuf::from(r"C:\Program Files\QwenWorkCN"),
    };
    if base.is_dir() {
        Some(base)
    } else {
        None
    }
}

/// Resolve the executable used to LAUNCH QwenWork with a path argument.
///
/// QwenWork uses an NSIS + `updater.cfg` versioned layout:
/// `…\QwenWorkCN\Launcher.exe` reads `updater.cfg` (`currentVersion`,
/// `appExecutableName`) and starts the current version's `QwenWorkCN.exe`.
/// We launch the version-stable `Launcher.exe`, NOT the versioned app exe
/// directly: an already-running Electron instance forwards a second process's
/// argv over its single-instance IPC, so the versioned exe only appears to work
/// when the app is already open and does nothing on a cold start. `Launcher.exe`
/// is the supported entry that bootstraps a new instance either way and still
/// passes the folder/file path through. Falls back to the highest-versioned
/// `QwenWorkCN.exe` only when no Launcher is present.
fn resolve_qwenwork_exe() -> Option<PathBuf> {
    let base = qwenwork_install_base()?;
    let launcher = base.join("Launcher.exe");
    if launcher.is_file() {
        return Some(launcher);
    }
    let entries = std::fs::read_dir(base).ok()?;
    let mut best: Option<(Vec<u64>, PathBuf)> = None;
    for entry in entries.flatten() {
        let dir = entry.path();
        let exe = dir.join("QwenWorkCN.exe");
        if !exe.is_file() {
            continue;
        }
        let folder = dir.file_name()?.to_string_lossy().to_string();
        let version = parse_version_key(&folder);
        let better = match &best {
            Some((current, _)) => version > *current,
            None => true,
        };
        if better {
            best = Some((version, exe));
        }
    }
    best.map(|(_, exe)| exe)
}

/// Turn a release folder name like `2.1.0-26111315` into a comparable numeric
/// key `[2, 1, 0, 26111315]`. Non-numeric segments are skipped; a longer key
/// with equal prefix sorts higher (`2.1.0-26111315` > `2.1.0`).
fn parse_version_key(raw: &str) -> Vec<u64> {
    raw.split(|c| c == '.' || c == '-' || c == '_')
        .filter_map(|token| token.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<u64>().ok())
        .collect()
}

fn terminal_args(terminal_id: &str, dir_str: &str) -> Vec<String> {
    match terminal_id {
        "wt" => vec!["-d".into(), dir_str.into()],
        "powershell" | "pwsh" => vec![
            "-NoExit".into(),
            "-Command".into(),
            format!(
                "Set-Location -LiteralPath '{}'",
                dir_str.replace('\'', "''")
            ),
        ],
        "cmd" => vec!["/K".into(), format!("cd /d {dir_str}")],
        "gitbash" => vec![format!("--cd={dir_str}")],
        _ => Vec::new(),
    }
}

/// `spawn_elevated` 的三种结局：成功 / 用户在 UAC 弹窗取消 / 失败。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElevateOutcome {
    Launched,
    Cancelled,
    Failed,
}

fn spawn_elevated(exe: &Path, args: &[String], dir: &Path) -> ElevateOutcome {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{GetLastError, ERROR_CANCELLED};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let verb: Vec<u16> = OsStr::new("runas")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let file: Vec<u16> = exe
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // lpParameters 引号规则：参数内的 `"` 翻倍转义；含空格/引号的参数整体加引号。
    let params_owned = args
        .iter()
        .map(|a| {
            let escaped = a.replace('"', "\"\"");
            if escaped.contains(' ') || escaped.contains('"') {
                format!("\"{escaped}\"")
            } else {
                escaped
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let params: Vec<u16> = OsStr::new(&params_owned)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let dir_wide: Vec<u16> = dir
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
        sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        sei.lpVerb = PCWSTR(verb.as_ptr());
        sei.lpFile = PCWSTR(file.as_ptr());
        sei.lpParameters = PCWSTR(params.as_ptr());
        sei.lpDirectory = PCWSTR(dir_wide.as_ptr());
        sei.nShow = SW_SHOWNORMAL.0;
        // 不传 SEE_MASK_NOCLOSEPROCESS：返回的 hProcess 无人关闭，只会泄漏句柄。
        if ShellExecuteExW(&mut sei).is_ok() {
            return ElevateOutcome::Launched;
        }
        // runas 被拒绝时 GetLastError() == ERROR_CANCELLED（用户点了“否”）。
        if GetLastError() == ERROR_CANCELLED {
            ElevateOutcome::Cancelled
        } else {
            ElevateOutcome::Failed
        }
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

/// DEL-003: only NTFS exposes a per-drive Recycle Bin on Windows. FAT32 / exFAT
/// volumes (common on USB) have none, so `trash::delete` there erases the file
/// outright with no confirmation. Kept pure over the file-system name so it is
/// cargo-testable without mounting a real FAT volume; an unknown name defers to
/// `trash::delete`'s own best effort.
pub(crate) fn filesystem_supports_recycle_bin(file_system_name: Option<&str>) -> bool {
    match file_system_name {
        Some(fs) => fs.eq_ignore_ascii_case("NTFS"),
        None => true,
    }
}

/// Read the volume file-system name for `path` and fold it into the
/// recyclability decision. `path` is expected to be a non-UNC local path (the
/// platform entry filters UNC before calling here).
pub(crate) fn volume_supports_recycle_bin(path: &str) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetVolumeInformationW;

    let Some(root) = volume_root(path) else {
        return filesystem_supports_recycle_bin(None);
    };
    let wide: Vec<u16> = std::ffi::OsStr::new(&root)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut fs_name = [0u16; 32];
    let ok = unsafe {
        GetVolumeInformationW(
            PCWSTR(wide.as_ptr()),
            None,
            None,
            None,
            None,
            Some(&mut fs_name),
        )
    };
    if ok.is_err() {
        return filesystem_supports_recycle_bin(None);
    }
    let end = fs_name.iter().position(|&c| c == 0).unwrap_or(fs_name.len());
    let fs = String::from_utf16_lossy(&fs_name[..end]);
    filesystem_supports_recycle_bin(if fs.is_empty() { None } else { Some(&fs) })
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

/// Enumerate live computers from the Shell **Network** folder
/// (`FOLDERID_NetworkFolder`, populated by WS-Discovery / Function Discovery —
/// the same source Windows Explorer's 网络 pane reads). This replaces the legacy
/// SMB1 `net view` browser, which is disabled by default on Win10/11 and hangs
/// (ADR 0026). Returns bare server names (no `\\` prefix); empty on any failure.
pub(crate) fn network_computers() -> Vec<String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{CoTaskMemFree, IBindCtx};
    use windows::Win32::UI::Shell::{
        SHBindToObject, SHCONTF_FOLDERS, SHCONTF_NONFOLDERS, SHCreateShellItem,
        SHGetKnownFolderIDList, SIGDN_DESKTOPABSOLUTEPARSING, FOLDERID_NetworkFolder, IEnumIDList,
        IShellFolder, KF_FLAG_DEFAULT,
    };
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;

    ensure_com_sta();
    let mut servers: Vec<String> = Vec::new();
    unsafe {
        let Ok(root_pidl) =
            SHGetKnownFolderIDList(&FOLDERID_NetworkFolder, KF_FLAG_DEFAULT.0 as u32, None)
        else {
            return servers;
        };
        let folder: IShellFolder = match SHBindToObject(
            None::<&IShellFolder>,
            root_pidl as *const ITEMIDLIST,
            None::<&IBindCtx>,
        ) {
            Ok(folder) => folder,
            Err(_) => {
                CoTaskMemFree(Some(root_pidl as *const _));
                return servers;
            }
        };

        let mut enumerator: Option<IEnumIDList> = None;
        let flags = (SHCONTF_FOLDERS.0 | SHCONTF_NONFOLDERS.0) as u32;
        if folder.EnumObjects(HWND::default(), flags, &mut enumerator).is_err() {
            CoTaskMemFree(Some(root_pidl as *const _));
            return servers;
        }
        let Some(enumerator) = enumerator else {
            CoTaskMemFree(Some(root_pidl as *const _));
            return servers;
        };

        loop {
            let mut child: *mut ITEMIDLIST = std::ptr::null_mut();
            let mut fetched: u32 = 0;
            if enumerator
                .Next(std::slice::from_mut(&mut child), Some(&mut fetched))
                .is_err()
                || fetched == 0
                || child.is_null()
            {
                break;
            }
            if let Ok(item) =
                SHCreateShellItem(None, Some(&folder), child as *const ITEMIDLIST)
            {
                if let Ok(PWSTR(buf)) = item.GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING) {
                    if !buf.is_null() {
                        let text = PWSTR(buf).to_string().unwrap_or_default();
                        CoTaskMemFree(Some(buf as *const _));
                        if let Some(rest) = text.strip_prefix(r"\\") {
                            if let Some(server) = rest.split('\\').next() {
                                let server = server.trim();
                                if !server.is_empty() {
                                    servers.push(server.to_string());
                                }
                            }
                        }
                    }
                }
            }
            CoTaskMemFree(Some(child as *const _));
        }
        CoTaskMemFree(Some(root_pidl as *const _));
    }
    servers.sort();
    servers.dedup();
    servers
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0026 host probe (run manually): `cargo test -- --ignored --nocapture
    /// probe_network_computers`. Enumerates the Shell Network folder
    /// (`::{Network}` / WS-Discovery, the same source Explorer's 网络 pane uses)
    /// to confirm it returns the live computers instead of hanging like the
    /// legacy SMB1 `net view` browser.
    #[test]
    #[ignore]
    fn probe_network_computers() {
        println!("network_computers = {:?}", network_computers());
    }

    #[test]
    fn windows_adapter_honours_the_shared_provider_contract() {
        super::super::assert_provider_capability_contract(&WindowsPlatform);
    }

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
        assert_eq!(
            volume_root(r"C:\Temp\afm-fixture\a").as_deref(),
            Some(r"C:\")
        );
        assert_eq!(
            volume_root(r"\\server\share\folder").as_deref(),
            Some(r"\\server\share\")
        );
        assert_eq!(volume_root("afm://guide"), None);
    }

    #[test]
    fn only_ntfs_recycles() {
        assert!(filesystem_supports_recycle_bin(Some("NTFS")));
        assert!(filesystem_supports_recycle_bin(Some("ntfs")));
    }

    #[test]
    fn fat_and_exfat_do_not_recycle() {
        for fs in ["FAT", "FAT32", "FAT16", "exFAT", "EXFAT", "ReFS"] {
            assert!(
                !filesystem_supports_recycle_bin(Some(fs)),
                "{fs} must not report a recycle bin"
            );
        }
    }

    #[test]
    fn unknown_filesystem_defers_to_trash() {
        assert!(filesystem_supports_recycle_bin(None));
    }
}
