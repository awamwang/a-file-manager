// SPDX-License-Identifier: GPL-3.0-or-later
//! AIA Preview Bridge — push selected local paths to Peek CLI / QuickLook pipe,
//! or signal the frontend to open the experimental built-in preview pane.
//!
//! This is an AFM-side bridge (one feature surface + thin protocol backends),
//! not a claim that every Quick Look–class tool works with zero adapter.

use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::AppHandle;

const SETTING_KEY: &str = "preview.externalProvider";
const QL_PIPE_PREFIX: &str = r"\\.\pipe\QuickLook.App.Pipe.";
const QL_TOGGLE_PREFIX: &str = "QuickLook.App.PipeMessages.Toggle|";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewBridgeResult {
    pub success: bool,
    /// `peek` | `quicklook` | `builtin` | `off` | `none`
    pub backend: String,
    pub error: Option<String>,
    /// Frontend should show/focus experimental PreviewPane.
    pub fallback_builtin: bool,
}

fn result_ok(backend: &str) -> PreviewBridgeResult {
    PreviewBridgeResult {
        success: true,
        backend: backend.to_string(),
        error: None,
        fallback_builtin: backend == "builtin",
    }
}

fn result_builtin(reason: Option<String>) -> PreviewBridgeResult {
    PreviewBridgeResult {
        success: true,
        backend: "builtin".into(),
        error: reason,
        fallback_builtin: true,
    }
}

fn result_off() -> PreviewBridgeResult {
    PreviewBridgeResult {
        success: true,
        backend: "off".into(),
        error: None,
        fallback_builtin: false,
    }
}

fn result_none(error: impl Into<String>) -> PreviewBridgeResult {
    PreviewBridgeResult {
        success: false,
        backend: "none".into(),
        error: Some(error.into()),
        fallback_builtin: false,
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

fn invoke_peek(paths: &[String]) -> Result<(), String> {
    let exe = resolve_peek_exe().ok_or_else(|| "PowerToys Peek not found".to_string())?;
    #[cfg(windows)]
    {
        // Peek is a UI process — do not hide the window with CREATE_NO_WINDOW.
        Command::new(&exe)
            .args(paths)
            .spawn()
            .map_err(|err| format!("Failed to launch Peek: {err}"))?;
        Ok(())
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

/// Write Toggle|path to QuickLook Named Pipe (IPC string only — no QL code linked).
fn invoke_quicklook(paths: &[String]) -> Result<(), String> {
    let pipe = quicklook_pipe_path()?;
    let target = paths
        .first()
        .ok_or_else(|| "No path for QuickLook".to_string())?;
    let line = format!("{QL_TOGGLE_PREFIX}{target}\n");
    let mut file = OpenOptions::new()
        .write(true)
        .open(&pipe)
        .map_err(|err| format!("QuickLook pipe unavailable ({pipe}): {err}"))?;
    file.write_all(line.as_bytes())
        .map_err(|err| format!("QuickLook pipe write failed: {err}"))?;
    let _ = file.flush();
    Ok(())
}

fn try_peek(paths: &[String]) -> Result<(), String> {
    invoke_peek(paths)
}

fn try_quicklook(paths: &[String]) -> Result<(), String> {
    invoke_quicklook(paths)
}

/// Resolve provider preference and invoke the best available backend.
pub fn invoke_bridge(app: &AppHandle, paths: &[String]) -> PreviewBridgeResult {
    let mode = provider_mode(app);
    if mode == "off" {
        return result_off();
    }

    let local = filter_local_paths(paths);
    if local.is_empty() {
        return match mode.as_str() {
            "builtin" => result_builtin(Some("No local file selected".into())),
            _ => result_none("No existing local path in selection"),
        };
    }

    match mode.as_str() {
        "builtin" => result_builtin(None),
        "peek" => match try_peek(&local) {
            Ok(()) => result_ok("peek"),
            Err(err) => result_builtin(Some(err)),
        },
        "quicklook" => match try_quicklook(&local) {
            Ok(()) => result_ok("quicklook"),
            Err(err) => result_builtin(Some(err)),
        },
        // auto (default): Peek CLI → QuickLook pipe → built-in pane
        _ => {
            if try_peek(&local).is_ok() {
                return result_ok("peek");
            }
            if try_quicklook(&local).is_ok() {
                return result_ok("quicklook");
            }
            result_builtin(Some(
                "No Peek/QuickLook bridge available; using built-in preview".into(),
            ))
        }
    }
}

#[tauri::command]
pub fn preview_bridge_invoke(app: AppHandle, paths: Vec<String>) -> PreviewBridgeResult {
    invoke_bridge(&app, &paths)
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
        assert!(c.iter().any(|p| p.to_string_lossy().contains("PowerToys.Peek.UI.exe")));
    }
}
