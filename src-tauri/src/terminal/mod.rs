// SPDX-License-Identifier: GPL-3.0-or-later
//! Enumerate local terminals and open a path's directory (Sigma-aligned).

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalAppInfo {
    pub id: String,
    pub name: String,
    pub available: bool,
    pub supports_elevation: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenInTerminalResult {
    pub success: bool,
    pub error: Option<String>,
    /// Which launcher was used: wt | powershell | pwsh | cmd | gitbash.
    pub launcher: Option<String>,
}

const TERMINAL_DEFS: &[(&str, &str)] = &[
    ("wt", "Windows Terminal"),
    ("powershell", "Windows PowerShell"),
    ("pwsh", "PowerShell"),
    ("cmd", "Command Prompt"),
    ("gitbash", "Git Bash"),
];

fn expand_user(path: &str) -> String {
    if let Ok(user) = std::env::var("USERNAME") {
        return path.replace("{user}", &user);
    }
    path.to_string()
}

fn path_is_file(candidate: &str) -> Option<PathBuf> {
    let expanded = expand_user(candidate);
    let path = PathBuf::from(&expanded);
    if path.is_file() {
        return Some(path);
    }
    if !expanded.contains('\\') && !expanded.contains('/') {
        if let Ok(output) = Command::new("where.exe").arg(&expanded).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = stdout.lines().next() {
                    let found = PathBuf::from(line.trim());
                    if found.is_file() {
                        return Some(found);
                    }
                }
            }
        }
    }
    None
}

fn resolve_terminal_exe(id: &str) -> Option<PathBuf> {
    match id {
        "wt" => path_is_file("wt.exe").or_else(|| path_is_file("wt")),
        "powershell" => path_is_file("powershell.exe"),
        "pwsh" => path_is_file("pwsh.exe").or_else(|| {
            path_is_file(r"C:\Program Files\PowerShell\7\pwsh.exe")
                .or_else(|| path_is_file(r"C:\Program Files\PowerShell\7-preview\pwsh.exe"))
        }),
        "cmd" => path_is_file("cmd.exe"),
        "gitbash" => [
            r"C:\Program Files\Git\git-bash.exe",
            r"C:\Program Files (x86)\Git\git-bash.exe",
            r"C:\Users\{user}\AppData\Local\Programs\Git\git-bash.exe",
        ]
        .into_iter()
        .find_map(path_is_file),
        _ => None,
    }
}

#[tauri::command]
pub fn terminal_list_apps() -> Vec<TerminalAppInfo> {
    TERMINAL_DEFS
        .iter()
        .map(|(id, name)| TerminalAppInfo {
            id: (*id).to_string(),
            name: (*name).to_string(),
            available: resolve_terminal_exe(id).is_some(),
            supports_elevation: true,
        })
        .collect()
}

#[tauri::command]
pub fn open_in_terminal(
    path: String,
    terminal_id: Option<String>,
    elevated: Option<bool>,
) -> OpenInTerminalResult {
    match resolve_directory(&path) {
        Ok(dir) => {
            let elev = elevated.unwrap_or(false);
            if let Some(id) = terminal_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                return spawn_terminal(&dir, id, elev);
            }
            for id in ["wt", "powershell", "pwsh", "cmd", "gitbash"] {
                if resolve_terminal_exe(id).is_none() {
                    continue;
                }
                let result = spawn_terminal(&dir, id, elev);
                if result.success {
                    return result;
                }
            }
            OpenInTerminalResult {
                success: false,
                error: Some(format!(
                    "Could not launch any terminal for {}",
                    dir.display()
                )),
                launcher: None,
            }
        }
        Err(error) => OpenInTerminalResult {
            success: false,
            error: Some(error),
            launcher: None,
        },
    }
}

fn resolve_directory(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Path is empty".to_string());
    }
    let p = Path::new(trimmed);
    if !p.exists() {
        return Err(format!("Path not found: {trimmed}"));
    }
    if p.is_dir() {
        return Ok(p.to_path_buf());
    }
    p.parent()
        .filter(|parent| parent.as_os_str().len() > 0 && parent.exists())
        .map(|parent| parent.to_path_buf())
        .ok_or_else(|| format!("No parent directory for: {trimmed}"))
}

fn spawn_terminal(dir: &Path, terminal_id: &str, elevated: bool) -> OpenInTerminalResult {
    let dir_str = dir.to_string_lossy().replace('/', "\\");
    let Some(exe) = resolve_terminal_exe(terminal_id) else {
        return OpenInTerminalResult {
            success: false,
            error: Some(format!("Terminal not available: {terminal_id}")),
            launcher: None,
        };
    };

    let args = terminal_args(terminal_id, &dir_str);
    let launched = if elevated {
        spawn_elevated(&exe, &args, dir)
    } else {
        let mut cmd = Command::new(&exe);
        cmd.args(&args).current_dir(dir);
        try_spawn(&mut cmd)
    };

    if launched {
        OpenInTerminalResult {
            success: true,
            error: None,
            launcher: Some(terminal_id.to_string()),
        }
    } else {
        OpenInTerminalResult {
            success: false,
            error: Some(format!(
                "Could not launch {terminal_id} for {}",
                dir.display()
            )),
            launcher: None,
        }
    }
}

fn terminal_args(terminal_id: &str, dir_str: &str) -> Vec<String> {
    match terminal_id {
        "wt" => vec!["-d".into(), dir_str.into()],
        "powershell" | "pwsh" => vec![
            "-NoExit".into(),
            "-Command".into(),
            format!("Set-Location -LiteralPath '{}'", dir_str.replace('\'', "''")),
        ],
        "cmd" => vec!["/K".into(), format!("cd /d {dir_str}")],
        "gitbash" => vec![format!("--cd={dir_str}")],
        _ => Vec::new(),
    }
}

fn try_spawn(command: &mut Command) -> bool {
    command.spawn().is_ok()
}

#[cfg(windows)]
fn spawn_elevated(exe: &Path, args: &[String], dir: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
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
    let params_owned = args
        .iter()
        .map(|a| {
            if a.contains(' ') {
                format!("\"{a}\"")
            } else {
                a.clone()
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
        sei.fMask = SEE_MASK_NOCLOSEPROCESS;
        ShellExecuteExW(&mut sei).is_ok()
    }
}

#[cfg(not(windows))]
fn spawn_elevated(_exe: &Path, _args: &[String], _dir: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn empty_path_fails() {
        let result = open_in_terminal(String::new(), None, None);
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn missing_path_fails() {
        let result = open_in_terminal(
            "Z:/definitely-missing-a-file-manager-terminal-xyz".to_string(),
            None,
            None,
        );
        assert!(!result.success);
        assert!(result.launcher.is_none());
    }

    #[test]
    fn resolve_directory_for_file_uses_parent() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let dir = std::env::temp_dir().join(format!("afm-term-{stamp}"));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("note.txt");
        fs::write(&file, b"x").unwrap();

        let resolved = resolve_directory(file.to_str().unwrap()).unwrap();
        assert_eq!(resolved, dir);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_apps_includes_core_ids() {
        let apps = terminal_list_apps();
        let ids: Vec<_> = apps.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"powershell"));
        assert!(ids.contains(&"cmd"));
        assert!(ids.contains(&"wt"));
        assert!(ids.contains(&"gitbash"));
    }
}
