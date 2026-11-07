// SPDX-License-Identifier: GPL-3.0-or-later
//! AIA Preview Bridge — push selected local paths to QuickLook pipe / Peek CLI,
//! or signal the frontend to open the experimental built-in preview pane.
//!
//! This is an AFM-side bridge (one feature surface + thin protocol backends),
//! not a claim that every Quick Look–class tool works with zero adapter.
//!
//! `auto` (AIA-254): try classic QuickLook Named Pipe first (resident → fast),
//! then PowerToys Peek spawn, then built-in. Missing QL must not hard-fail.
//!
//! Selection follow (AIA-174): while a Space-opened session is active, AFM can
//! push path updates — QuickLook `Switch|path`, Peek replace-by-PID (CLI has no
//! single-instance / follow-file API; dual spawn opens a second window), built-in
//! pane via frontend computed path.
//!
//! AIA-244: dismissing via the preview pane × (or an explicit dismiss) clears the
//! session so selection changes alone must not re-open until Space / preview again.

use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tauri::AppHandle;

const SETTING_KEY: &str = "preview.externalProvider";
const QL_PIPE_PREFIX: &str = r"\\.\pipe\QuickLook.App.Pipe.";
const QL_TOGGLE_PREFIX: &str = "QuickLook.App.PipeMessages.Toggle|";
const QL_SWITCH_PREFIX: &str = "QuickLook.App.PipeMessages.Switch|";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewBridgeResult {
    pub success: bool,
    /// `peek` | `quicklook` | `builtin` | `off` | `none`
    pub backend: String,
    pub error: Option<String>,
    /// Frontend should show/focus experimental PreviewPane.
    pub fallback_builtin: bool,
    /// True when a Space session is active and selection follow should run.
    pub session_active: bool,
}

#[derive(Debug, Clone)]
struct BridgeSession {
    backend: String,
    /// PID of the Peek.UI process AFM spawned (for replace-on-follow / Space close).
    peek_pid: Option<u32>,
}

static SESSION: Mutex<Option<BridgeSession>> = Mutex::new(None);

fn result_ok(backend: &str, session_active: bool) -> PreviewBridgeResult {
    PreviewBridgeResult {
        success: true,
        backend: backend.to_string(),
        error: None,
        // Only force the experimental pane while a built-in session is active.
        fallback_builtin: backend == "builtin" && session_active,
        session_active,
    }
}

fn result_builtin(reason: Option<String>, session_active: bool) -> PreviewBridgeResult {
    PreviewBridgeResult {
        success: true,
        backend: "builtin".into(),
        error: reason,
        fallback_builtin: session_active,
        session_active,
    }
}

fn result_off() -> PreviewBridgeResult {
    PreviewBridgeResult {
        success: true,
        backend: "off".into(),
        error: None,
        fallback_builtin: false,
        session_active: false,
    }
}

fn result_none(error: impl Into<String>) -> PreviewBridgeResult {
    PreviewBridgeResult {
        success: false,
        backend: "none".into(),
        error: Some(error.into()),
        fallback_builtin: false,
        session_active: session_is_active(),
    }
}

fn session_is_active() -> bool {
    SESSION
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false)
}

fn clear_session() {
    if let Ok(mut g) = SESSION.lock() {
        *g = None;
    }
}

fn set_session(backend: &str, peek_pid: Option<u32>) {
    if let Ok(mut g) = SESSION.lock() {
        *g = Some(BridgeSession {
            backend: backend.to_string(),
            peek_pid,
        });
    }
}

fn provider_mode(app: &AppHandle) -> String {
    let raw = crate::settings::read_string_setting(app, SETTING_KEY);
    let mode = raw.trim().to_ascii_lowercase();
    match mode.as_str() {
        "peek" | "quicklook" | "builtin" | "off" | "auto" => mode,
        _ => "auto".into(),
    }
}

/// Local filesystem paths only — skip virtual providers / empty / missing.
fn filter_local_paths(paths: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for raw in paths {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Virtual / URI providers (awam:, rclone:, etc.) are not Peek/QL CLI targets.
        if trimmed.contains("://") {
            continue;
        }
        let path = Path::new(trimmed);
        if path.exists() {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn peek_exe_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    out.push(PathBuf::from(
        r"C:\Program Files\PowerToys\WinUI3Apps\PowerToys.Peek.UI.exe",
    ));
    out.push(PathBuf::from(
        r"C:\Program Files (x86)\PowerToys\WinUI3Apps\PowerToys.Peek.UI.exe",
    ));
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        out.push(
            PathBuf::from(local)
                .join("PowerToys")
                .join("WinUI3Apps")
                .join("PowerToys.Peek.UI.exe"),
        );
    }
    out
}

fn resolve_peek_exe() -> Option<PathBuf> {
    peek_exe_candidates().into_iter().find(|p| p.is_file())
}

#[cfg(windows)]
fn process_still_alive(pid: u32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };
        // Wait 0 ms: signaled ⇒ already exited.
        let status = WaitForSingleObject(handle, 0);
        let _ = CloseHandle(handle);
        status != WAIT_OBJECT_0
    }
}

#[cfg(not(windows))]
fn process_still_alive(_pid: u32) -> bool {
    false
}

#[cfg(windows)]
fn terminate_pid(pid: u32) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid) {
            let _ = TerminateProcess(handle, 1);
            let _ = CloseHandle(handle);
        }
    }
}

#[cfg(not(windows))]
fn terminate_pid(_pid: u32) {}

fn invoke_peek(paths: &[String]) -> Result<u32, String> {
    let exe = resolve_peek_exe().ok_or_else(|| "PowerToys Peek not found".to_string())?;
    #[cfg(windows)]
    {
        // Peek is a UI process — do not hide the window with CREATE_NO_WINDOW.
        let child = Command::new(&exe)
            .args(paths)
            .spawn()
            .map_err(|err| format!("Failed to launch Peek: {err}"))?;
        Ok(child.id())
    }
    #[cfg(not(windows))]
    {
        let _ = (exe, paths);
        Err("Peek bridge is Windows-only".into())
    }
}

#[cfg(windows)]
fn current_user_sid_string() -> Result<String, String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL};
    use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)
            .map_err(|err| format!("OpenProcessToken: {err}"))?;

        let mut needed = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        if needed == 0 {
            let _ = CloseHandle(token);
            return Err("GetTokenInformation size failed".into());
        }

        let mut buf = vec![0u8; needed as usize];
        if let Err(err) = GetTokenInformation(
            token,
            TokenUser,
            Some(buf.as_mut_ptr().cast()),
            needed,
            &mut needed,
        ) {
            let _ = CloseHandle(token);
            return Err(format!("GetTokenInformation: {err}"));
        }

        let token_user = &*(buf.as_ptr() as *const TOKEN_USER);
        let mut sid_pwstr = PWSTR::null();
        if let Err(err) = ConvertSidToStringSidW(token_user.User.Sid, &mut sid_pwstr) {
            let _ = CloseHandle(token);
            return Err(format!("ConvertSidToStringSidW: {err}"));
        }

        let sid = match sid_pwstr.to_string() {
            Ok(s) => s,
            Err(err) => {
                let _ = LocalFree(Some(HLOCAL(sid_pwstr.as_ptr().cast())));
                let _ = CloseHandle(token);
                return Err(format!("SID UTF-16 decode: {err}"));
            }
        };
        let _ = LocalFree(Some(HLOCAL(sid_pwstr.as_ptr().cast())));
        let _ = CloseHandle(token);
        Ok(sid)
    }
}

#[cfg(not(windows))]
fn current_user_sid_string() -> Result<String, String> {
    Err("QuickLook pipe is Windows-only".into())
}

fn quicklook_pipe_path() -> Result<String, String> {
    let sid = current_user_sid_string()?;
    Ok(format!("{QL_PIPE_PREFIX}{sid}"))
}

fn write_quicklook_message(prefix: &str, paths: &[String]) -> Result<(), String> {
    let pipe = quicklook_pipe_path()?;
    let target = paths
        .first()
        .ok_or_else(|| "No path for QuickLook".to_string())?;
    let line = format!("{prefix}{target}\n");
    let mut file = OpenOptions::new()
        .write(true)
        .open(&pipe)
        .map_err(|err| format!("QuickLook pipe unavailable ({pipe}): {err}"))?;
    file.write_all(line.as_bytes())
        .map_err(|err| format!("QuickLook pipe write failed: {err}"))?;
    let _ = file.flush();
    Ok(())
}

/// Write Toggle|path to QuickLook Named Pipe (IPC string only — no QL code linked).
fn invoke_quicklook_toggle(paths: &[String]) -> Result<(), String> {
    write_quicklook_message(QL_TOGGLE_PREFIX, paths)
}

/// Write Switch|path — updates open QL window; no-op if QL preview is closed.
fn invoke_quicklook_switch(paths: &[String]) -> Result<(), String> {
    write_quicklook_message(QL_SWITCH_PREFIX, paths)
}

fn try_peek(paths: &[String]) -> Result<u32, String> {
    invoke_peek(paths)
}

fn try_quicklook_toggle(paths: &[String]) -> Result<(), String> {
    invoke_quicklook_toggle(paths)
}

/// Ordered outcome for `preview.externalProvider = auto`.
/// QuickLook wins when its pipe accepts Toggle; else Peek; else built-in.
/// Callers treat a failed QL probe as `quicklook_ok = false` (silent fallback).
fn pick_auto_backend(quicklook_ok: bool, peek_ok: bool) -> &'static str {
    if quicklook_ok {
        "quicklook"
    } else if peek_ok {
        "peek"
    } else {
        "builtin"
    }
}

fn close_peek_session_if_alive() -> bool {
    let pid = SESSION
        .lock()
        .ok()
        .and_then(|g| g.as_ref().and_then(|s| s.peek_pid));
    let Some(pid) = pid else {
        return false;
    };
    if !process_still_alive(pid) {
        clear_session();
        return false;
    }
    terminate_pid(pid);
    clear_session();
    true
}

/// Space / open: resolve provider preference and invoke the best available backend.
/// If a Peek/QL/builtin session is already open, Space closes it (toggle).
pub fn invoke_bridge(app: &AppHandle, paths: &[String]) -> PreviewBridgeResult {
    let mode = provider_mode(app);
    if mode == "off" {
        clear_session();
        return result_off();
    }

    // Space-toggle close while an AFM session is already open.
    let active_backend = SESSION
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.backend.clone()));
    match active_backend.as_deref() {
        Some("peek") if matches!(mode.as_str(), "auto" | "peek") => {
            if close_peek_session_if_alive() {
                return result_ok("peek", false);
            }
            // PID already gone — fall through to open again.
        }
        Some("quicklook") if matches!(mode.as_str(), "auto" | "quicklook") => {
            let local_for_close = filter_local_paths(paths);
            if !local_for_close.is_empty() {
                let _ = try_quicklook_toggle(&local_for_close);
            }
            clear_session();
            return result_ok("quicklook", false);
        }
        Some("builtin") if matches!(mode.as_str(), "auto" | "builtin" | "peek" | "quicklook") => {
            // Space again dismisses the force-shown built-in pane.
            clear_session();
            return result_ok("builtin", false);
        }
        _ => {}
    }

    let local = filter_local_paths(paths);
    if local.is_empty() {
        return match mode.as_str() {
            "builtin" => {
                set_session("builtin", None);
                result_builtin(Some("No local file selected".into()), true)
            }
            _ => result_none("No existing local path in selection"),
        };
    }

    match mode.as_str() {
        "builtin" => {
            set_session("builtin", None);
            result_builtin(None, true)
        }
        "peek" => match try_peek(&local) {
            Ok(pid) => {
                set_session("peek", Some(pid));
                result_ok("peek", true)
            }
            Err(err) => {
                set_session("builtin", None);
                result_builtin(Some(err), true)
            }
        },
        "quicklook" => match try_quicklook_toggle(&local) {
            Ok(()) => {
                set_session("quicklook", None);
                result_ok("quicklook", true)
            }
            Err(err) => {
                set_session("builtin", None);
                result_builtin(Some(err), true)
            }
        },
        // auto (default): QuickLook pipe → Peek CLI → built-in pane (AIA-254).
        // QL failure is silent here — do not surface pipe-missing as a hard error.
        _ => {
            let ql_ok = try_quicklook_toggle(&local).is_ok();
            let peek_pid = if ql_ok {
                None
            } else {
                try_peek(&local).ok()
            };
            match pick_auto_backend(ql_ok, peek_pid.is_some()) {
                "quicklook" => {
                    set_session("quicklook", None);
                    result_ok("quicklook", true)
                }
                "peek" => {
                    let pid = peek_pid.expect("peek_ok implies Some(pid)");
                    set_session("peek", Some(pid));
                    result_ok("peek", true)
                }
                _ => {
                    set_session("builtin", None);
                    result_builtin(
                        Some(
                            "No QuickLook/Peek bridge available; using built-in preview"
                                .into(),
                        ),
                        true,
                    )
                }
            }
        }
    }
}

/// Selection follow while a Space-opened preview session is active.
/// Peek: terminate AFM-spawned PID then launch with the new path (CLI has no update API).
/// QuickLook: `Switch|path` (no-op if QL window closed; rematches plugin on type change).
/// Built-in: session stays active; frontend PreviewPane already watches selection.
pub fn switch_bridge(app: &AppHandle, paths: &[String]) -> PreviewBridgeResult {
    let mode = provider_mode(app);
    if mode == "off" {
        clear_session();
        return result_off();
    }

    let session = match SESSION.lock() {
        Ok(g) => g.clone(),
        Err(_) => None,
    };
    let Some(session) = session else {
        return result_none("No active preview bridge session");
    };

    let local = filter_local_paths(paths);
    if local.is_empty() {
        return result_none("No existing local path in selection");
    }

    match session.backend.as_str() {
        "peek" => {
            if let Some(pid) = session.peek_pid {
                if !process_still_alive(pid) {
                    clear_session();
                    return result_none("Peek session closed");
                }
                terminate_pid(pid);
            }
            match try_peek(&local) {
                Ok(pid) => {
                    set_session("peek", Some(pid));
                    result_ok("peek", true)
                }
                Err(err) => {
                    clear_session();
                    result_none(err)
                }
            }
        }
        "quicklook" => match invoke_quicklook_switch(&local) {
            Ok(()) => result_ok("quicklook", true),
            Err(err) => {
                // Pipe gone ⇒ QL quit; drop session so we stop spamming.
                clear_session();
                result_none(err)
            }
        },
        "builtin" => result_ok("builtin", true),
        other => result_none(format!("Unknown session backend: {other}")),
    }
}

/// AIA-244: user dismissed preview (pane ×). End the Space session and tear down
/// Peek/QL if AFM still owns them. Selection follow must not resurrect after this.
pub fn dismiss_bridge() -> PreviewBridgeResult {
    let backend = SESSION
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.backend.clone()));
    match backend.as_deref() {
        Some("peek") => {
            let _ = close_peek_session_if_alive();
            clear_session();
            result_ok("peek", false)
        }
        Some("quicklook") => {
            // Best-effort close: Toggle on empty/no path is unreliable; clearing the
            // session is what stops selection follow from calling Switch again.
            clear_session();
            result_ok("quicklook", false)
        }
        Some("builtin") => {
            clear_session();
            result_ok("builtin", false)
        }
        _ => {
            clear_session();
            result_ok("none", false)
        }
    }
}

#[tauri::command]
pub fn preview_bridge_invoke(app: AppHandle, paths: Vec<String>) -> PreviewBridgeResult {
    invoke_bridge(&app, &paths)
}

#[tauri::command]
pub fn preview_bridge_switch(app: AppHandle, paths: Vec<String>) -> PreviewBridgeResult {
    switch_bridge(&app, &paths)
}

#[tauri::command]
pub fn preview_bridge_dismiss() -> PreviewBridgeResult {
    dismiss_bridge()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_skips_virtual_and_missing() {
        let tmp = std::env::temp_dir().join(format!(
            "afm-preview-bridge-{}",
            std::process::id()
        ));
        std::fs::write(&tmp, b"x").unwrap();
        let paths = vec![
            tmp.to_string_lossy().into_owned(),
            "awam://knowledge/foo".into(),
            String::new(),
            r"Z:\definitely-missing-afm-preview-bridge-xyz".into(),
        ];
        let filtered = filter_local_paths(&paths);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0], tmp.to_string_lossy());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn peek_candidates_include_program_files() {
        let c = peek_exe_candidates();
        assert!(c
            .iter()
            .any(|p| p.to_string_lossy().contains("PowerToys.Peek.UI.exe")));
    }

    #[test]
    fn ql_prefixes_distinguish_toggle_and_switch() {
        assert!(QL_TOGGLE_PREFIX.contains("Toggle|"));
        assert!(QL_SWITCH_PREFIX.contains("Switch|"));
        assert_ne!(QL_TOGGLE_PREFIX, QL_SWITCH_PREFIX);
    }

    #[test]
    fn dismiss_clears_builtin_session_and_stays_inactive() {
        set_session("builtin", None);
        assert!(session_is_active());
        let closed = dismiss_bridge();
        assert_eq!(closed.backend, "builtin");
        assert!(!closed.session_active);
        assert!(!closed.fallback_builtin);
        assert!(!session_is_active());
        // Selection follow with no session must not resurrect.
        assert!(!session_is_active());
        let again = dismiss_bridge();
        assert!(!again.session_active);
        assert!(!session_is_active());
    }

    #[test]
    fn auto_prefers_quicklook_before_peek() {
        assert_eq!(pick_auto_backend(true, true), "quicklook");
        assert_eq!(pick_auto_backend(true, false), "quicklook");
    }

    #[test]
    fn auto_falls_back_to_peek_when_quicklook_missing() {
        // Missing / unreachable QL must not hard-fail — Peek is next.
        assert_eq!(pick_auto_backend(false, true), "peek");
    }

    #[test]
    fn auto_falls_back_to_builtin_when_both_external_missing() {
        assert_eq!(pick_auto_backend(false, false), "builtin");
    }
}
