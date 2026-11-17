// SPDX-License-Identifier: GPL-3.0-or-later
//! Native Windows "Open with" dialog (AIA-58). Day-1: OS dialog only (no app list submenu).

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenWithResult {
    pub success: bool,
    pub error: Option<String>,
}

/// Open a file/folder with the OS default association (ShellExecute "open").
/// Windows keeps its ShellExecute/explorer helpers (opener path ACL, FIL-30);
/// other platforms reuse the already-registered `tauri-plugin-opener`.
/// 文件夹走 explorer 直启；整体放阻塞线程，避免与 Skill 导入等 FS 重活互相卡住 UI（FIL-30）。
#[tauri::command]
pub async fn open_path_default(path: String) -> OpenWithResult {
    tauri::async_runtime::spawn_blocking(move || open_path_default_inner(path))
        .await
        .unwrap_or_else(|err| OpenWithResult {
            success: false,
            error: Some(format!("open_path_default worker failed: {err}")),
        })
}

fn open_path_default_inner(path: String) -> OpenWithResult {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return OpenWithResult {
            success: false,
            error: Some("Path is empty".to_string()),
        };
    }
    let p = Path::new(trimmed);
    if !p.exists() {
        return OpenWithResult {
            success: false,
            error: Some(format!("Path not found: {trimmed}")),
        };
    }
    #[cfg(windows)]
    {
        // 目录用 explorer 进程直启，不经 ShellExecute（壳扩展/DDE 仍可能拖住 ASYNCOK）。
        if p.is_dir() {
            return windows_impl::open_folder_in_explorer(trimmed);
        }
        windows_impl::open_default(trimmed)
    }
    #[cfg(not(windows))]
    {
        // `tauri-plugin-opener` is registered in lib.rs and already used by the
        // frontend. Its `open_path` free function wraps the platform opener
        // (`open` on macOS / `xdg-open` on Linux) behind one cross-platform API,
        // so there is no reason to spawn a shell command by hand here.
        match tauri_plugin_opener::open_path(p, None::<&str>) {
            Ok(()) => OpenWithResult {
                success: true,
                error: None,
            },
            Err(err) => OpenWithResult {
                success: false,
                error: Some(format!("Failed to open path: {err}")),
            },
        }
    }
}

/// 把 Prowlarr 搜索结果交给系统默认程序：magnet 走协议关联，.torrent 下载链接
/// 先落地为临时 `.torrent` 再走文件关联。AFM 不猜具体下载器，交给系统关联。
const TORRENT_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_TORRENT_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TorrentTargetKind {
    Magnet,
    Http,
    Unsupported,
}

fn classify_torrent_target(raw: &str) -> TorrentTargetKind {
    let value = raw.trim().to_ascii_lowercase();
    if value.starts_with("magnet:") {
        TorrentTargetKind::Magnet
    } else if value.starts_with("http://") || value.starts_with("https://") {
        TorrentTargetKind::Http
    } else {
        TorrentTargetKind::Unsupported
    }
}

/// 用作临时种子文件名：去掉路径分隔符等非法字符，超长截断。
fn sanitize_torrent_stem(raw: Option<&str>) -> String {
    let mut stem = String::new();
    for ch in raw.unwrap_or("").trim().chars().take(80) {
        if ch.is_control() || matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            stem.push('_');
        } else {
            stem.push(ch);
        }
    }
    let mut stem = stem.trim().trim_matches(|c| c == '.' || c == ' ').to_string();
    // 调用方可能直接给 "Title.torrent"；临时文件名末尾还会补 -时间戳.torrent。
    if stem.to_ascii_lowercase().ends_with(".torrent") {
        stem.truncate(stem.len() - ".torrent".len());
        stem = stem.trim().trim_matches(|c| c == '.' || c == ' ').to_string();
    }
    if stem.is_empty() {
        "torrent".to_string()
    } else {
        stem
    }
}

fn torrent_temp_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir()
        .join("a-file-manager-torrents")
        .join(format!("{stem}-{nanos}.torrent"))
}

fn download_torrent_to_temp(url: &str, title: Option<&str>) -> Result<PathBuf, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TORRENT_DOWNLOAD_TIMEOUT))
        .build()
        .into();
    let mut response = agent
        .get(url)
        .call()
        .map_err(|err| format!("下载种子失败: {err}"))?;
    if let Some(content_type) = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
    {
        if content_type.to_ascii_lowercase().contains("text/html") {
            return Err("下载地址返回的是网页而不是种子文件（可能需要登录或 API Key）".to_string());
        }
    }
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_TORRENT_BYTES)
        .read_to_vec()
        .map_err(|err| format!("读取种子文件失败: {err}"))?;
    write_torrent_temp(&bytes, title)
}

/// 把种子字节写到临时目录（`%TEMP%/a-file-manager-torrents`），返回落盘路径。
fn write_torrent_temp(bytes: &[u8], title: Option<&str>) -> Result<PathBuf, String> {
    if bytes.is_empty() {
        return Err("种子文件为空".to_string());
    }
    // bencode 字典以 'd' 开头；'<' 说明拿到的是 XML/HTML（登录页、索引器错误页）。
    if bytes[0] == b'<' {
        return Err("下载地址返回的是网页而不是种子文件（可能需要登录或 API Key）".to_string());
    }
    let path = torrent_temp_path(&sanitize_torrent_stem(title));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| format!("创建种子临时目录失败: {err}"))?;
    }
    std::fs::write(&path, bytes).map_err(|err| format!("写入种子文件失败: {err}"))?;
    Ok(path)
}

/// 网关（awam-file）已经用 Prowlarr 的 API Key 取回种子字节，这里解码落盘后
/// 交给系统 `.torrent` 关联——省掉二次 HTTP，也不再依赖白令牌地址。
fn write_torrent_from_base64(name: Option<&str>, data: &str) -> Result<PathBuf, String> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    let bytes = B64
        .decode(data.trim())
        .map_err(|err| format!("种子数据解码失败: {err}"))?;
    write_torrent_temp(&bytes, name)
}

fn torrent_failure(error: impl Into<String>) -> OpenWithResult {
    OpenWithResult {
        success: false,
        error: Some(error.into()),
    }
}

fn open_torrent_target_inner(url: String, title: Option<String>) -> OpenWithResult {
    let target = url.trim();
    match classify_torrent_target(target) {
        TorrentTargetKind::Magnet => {
            #[cfg(windows)]
            {
                // ShellExecute 的 "open" 会按 magnet: 协议关联启动默认下载器。
                windows_impl::open_default(target)
            }
            #[cfg(not(windows))]
            {
                match tauri_plugin_opener::open_url(target, None::<&str>) {
                    Ok(()) => OpenWithResult {
                        success: true,
                        error: None,
                    },
                    Err(err) => torrent_failure(format!("打开 magnet 失败: {err}")),
                }
            }
        }
        TorrentTargetKind::Http => {
            let path = match download_torrent_to_temp(target, title.as_deref()) {
                Ok(path) => path,
                Err(err) => return torrent_failure(err),
            };
            // 复用本地文件默认关联（Windows ShellExecute / 其它平台 opener）。
            open_path_default_inner(path.to_string_lossy().into_owned())
        }
        TorrentTargetKind::Unsupported => {
            torrent_failure("只支持 magnet: 链接或 http(s) 种子下载地址")
        }
    }
}

/// Provider 结果（如 Prowlarr）投递到系统默认下载器。
#[tauri::command]
pub async fn open_torrent_target(url: String, title: Option<String>) -> OpenWithResult {
    tauri::async_runtime::spawn_blocking(move || open_torrent_target_inner(url, title))
        .await
        .unwrap_or_else(|err| torrent_failure(format!("open_torrent_target worker failed: {err}")))
}

/// 种子的原始字节已经由网关取回（base64 编码）：落盘后交给系统 `.torrent` 关联。
#[tauri::command]
pub async fn open_torrent_data(name: Option<String>, data: String) -> OpenWithResult {
    tauri::async_runtime::spawn_blocking(move || open_torrent_data_inner(name, data))
        .await
        .unwrap_or_else(|err| torrent_failure(format!("open_torrent_data worker failed: {err}")))
}

fn open_torrent_data_inner(name: Option<String>, data: String) -> OpenWithResult {
    if data.trim().is_empty() {
        return torrent_failure("种子数据为空");
    }
    let path = match write_torrent_from_base64(name.as_deref(), &data) {
        Ok(path) => path,
        Err(err) => return torrent_failure(err),
    };
    open_path_default_inner(path.to_string_lossy().into_owned())
}

/// 用系统资源管理器打开回收站（`shell:RecycleBinFolder`），避免 AFM 内慢路径枚举。
#[tauri::command]
pub fn open_recycle_bin_system() -> OpenWithResult {
    #[cfg(windows)]
    {
        windows_impl::open_recycle_bin_in_explorer()
    }
    #[cfg(not(windows))]
    {
        // macOS has no `shell:RecycleBinFolder`; open the user's Trash folder.
        let Some(trash) = crate::platform::home_dir().map(|home| home.join(".Trash")) else {
            return OpenWithResult {
                success: false,
                error: Some("Could not resolve the Trash folder".to_string()),
            };
        };
        match tauri_plugin_opener::open_path(&trash, None::<&str>) {
            Ok(()) => OpenWithResult {
                success: true,
                error: None,
            },
            Err(err) => OpenWithResult {
                success: false,
                error: Some(format!("Failed to open Trash: {err}")),
            },
        }
    }
}

#[tauri::command]
pub fn open_native_open_with_dialog(path: String) -> OpenWithResult {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return OpenWithResult {
            success: false,
            error: Some("Path is empty".to_string()),
        };
    }
    let p = Path::new(trimmed);
    if !p.exists() {
        return OpenWithResult {
            success: false,
            error: Some(format!("Path not found: {trimmed}")),
        };
    }
    // Files use the native Open With dialog; folders are accepted so callers can
    // fall back to IDE launchers / Explorer when the OS dialog is unsuitable.
    #[cfg(windows)]
    {
        if p.is_dir() {
            return windows_impl::open_folder_in_explorer(trimmed);
        }
        windows_impl::open_as(trimmed)
    }
    #[cfg(not(windows))]
    {
        OpenWithResult {
            success: false,
            error: Some("Native Open With dialog is only supported on Windows".to_string()),
        }
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::OpenWithResult;
    use crate::win_process;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_ASYNCOK, SEE_MASK_INVOKEIDLIST, SHELLEXECUTEINFOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    #[cfg(test)]
    thread_local! {
        /// 单元测试记录目录分支命中次数，验证分支时无需真的启动 Explorer。
        pub(crate) static EXPLORER_OPEN_CALLS: std::cell::Cell<usize> = std::cell::Cell::new(0);
    }

    pub fn open_folder_in_explorer(path: &str) -> OpenWithResult {
        // 单元测试只验证「目录走 explorer 分支」，不启动真实进程：否则每次
        // cargo test 都会用 std::env::temp_dir() 弹出 %TEMP% 资源管理器窗口。
        #[cfg(test)]
        {
            let _ = path;
            EXPLORER_OPEN_CALLS.with(|calls| calls.set(calls.get() + 1));
            return OpenWithResult {
                success: true,
                error: None,
            };
        }
        #[cfg(not(test))]
        {
            let absolute = path.replace('/', "\\");
            match win_process::command("explorer.exe").arg(&absolute).spawn() {
                Ok(_) => OpenWithResult {
                    success: true,
                    error: None,
                },
                Err(err) => OpenWithResult {
                    success: false,
                    error: Some(format!("Failed to open folder: {err}")),
                },
            }
        }
    }

    pub fn open_recycle_bin_in_explorer() -> OpenWithResult {
        // explorer 认识 shell: 命名空间；比 AFM 内 PowerShell/COM 枚举整仓快得多。
        match win_process::command("explorer.exe")
            .arg("shell:RecycleBinFolder")
            .spawn()
        {
            Ok(_) => OpenWithResult {
                success: true,
                error: None,
            },
            Err(err) => OpenWithResult {
                success: false,
                error: Some(format!("Failed to open Recycle Bin: {err}")),
            },
        }
    }

    pub fn open_default(path: &str) -> OpenWithResult {
        let absolute = path.replace('/', "\\");
        let file_wide: Vec<u16> = OsStr::new(&absolute)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let verb_wide: Vec<u16> = OsStr::new("open")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // Tauri sync commands run off the UI apartment (no message pump).
        // ShellExecuteEx without SEE_MASK_ASYNCOK waits for DDE / async
        // activation to finish — MKV player associations commonly DDE, and a
        // pump-less wait times out around ~30s before returning. Ask the shell
        // to return immediately; Explorer-like open latency.
        unsafe {
            let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
            sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            sei.fMask = SEE_MASK_ASYNCOK;
            sei.lpVerb = PCWSTR(verb_wide.as_ptr());
            sei.lpFile = PCWSTR(file_wide.as_ptr());
            sei.nShow = SW_SHOWNORMAL.0;

            if ShellExecuteExW(&mut sei).is_ok() {
                return OpenWithResult {
                    success: true,
                    error: None,
                };
            }
        }

        // Fallback: cmd start "" "path"
        match win_process::command("cmd.exe")
            .args(["/C", "start", "", &absolute])
            .spawn()
        {
            Ok(_) => OpenWithResult {
                success: true,
                error: None,
            },
            Err(err) => OpenWithResult {
                success: false,
                error: Some(format!("Failed to open path: {err}")),
            },
        }
    }

    pub fn open_as(path: &str) -> OpenWithResult {
        let absolute = path.replace('/', "\\");
        let verb_wide: Vec<u16> = OsStr::new("openas")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let file_wide: Vec<u16> = OsStr::new(&absolute)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
            sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            sei.lpVerb = PCWSTR(verb_wide.as_ptr());
            sei.lpFile = PCWSTR(file_wide.as_ptr());
            sei.nShow = SW_SHOWNORMAL.0;
            sei.fMask = SEE_MASK_INVOKEIDLIST;

            if ShellExecuteExW(&mut sei).is_ok() {
                return OpenWithResult {
                    success: true,
                    error: None,
                };
            }
        }

        match win_process::command("rundll32.exe")
            .args(["shell32.dll,OpenAs_RunDLL", &absolute])
            .spawn()
        {
            Ok(_) => OpenWithResult {
                success: true,
                error: None,
            },
            Err(err) => OpenWithResult {
                success: false,
                error: Some(format!("Failed to open Open With dialog: {err}")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_path_fails() {
        let result = open_native_open_with_dialog(String::new());
        assert!(!result.success);
    }

    #[test]
    fn open_path_default_empty_fails() {
        let result = open_path_default_inner(String::new());
        assert!(!result.success);
    }

    #[test]
    fn open_path_default_missing_fails() {
        let result = open_path_default_inner("Z:/missing-afm-open-default-xyz.bin".into());
        assert!(!result.success);
    }

    #[cfg(windows)]
    #[test]
    fn open_path_default_directory_uses_explorer_branch() {
        use super::windows_impl::EXPLORER_OPEN_CALLS;
        let before = EXPLORER_OPEN_CALLS.with(|calls| calls.get());
        let dir = std::env::temp_dir();
        let result = open_path_default_inner(dir.to_string_lossy().into_owned());
        assert!(result.success);
        assert_eq!(
            EXPLORER_OPEN_CALLS.with(|calls| calls.get()),
            before + 1,
            "directory must take the explorer branch without spawning a window",
        );
    }

    #[test]
    fn missing_path_fails() {
        let result = open_native_open_with_dialog("Z:/missing-afm-open-with-xyz.bin".to_string());
        assert!(!result.success);
    }

    #[test]
    fn directory_accepted_on_windows() {
        #[cfg(windows)]
        let before = super::windows_impl::EXPLORER_OPEN_CALLS.with(|calls| calls.get());
        let dir = std::env::temp_dir();
        let result = open_native_open_with_dialog(dir.to_string_lossy().into_owned());
        #[cfg(windows)]
        {
            assert!(result.success);
            assert_eq!(
                super::windows_impl::EXPLORER_OPEN_CALLS.with(|calls| calls.get()),
                before + 1,
                "directory must take the explorer branch without spawning a window",
            );
        }
        #[cfg(not(windows))]
        assert!(!result.success);
    }

    #[test]
    fn torrent_target_kind_detects_magnet_and_http() {
        assert_eq!(
            classify_torrent_target("  MAGNET:?xt=urn:btih:abc "),
            TorrentTargetKind::Magnet
        );
        assert_eq!(
            classify_torrent_target("https://indexer.example/download/1.torrent"),
            TorrentTargetKind::Http
        );
        assert_eq!(classify_torrent_target("ftp://x/y"), TorrentTargetKind::Unsupported);
    }

    #[test]
    fn torrent_stem_is_safe_and_non_empty() {
        assert_eq!(
            sanitize_torrent_stem(Some(r"Ubuntu 24.04:desktop?<>.torrent")),
            "Ubuntu 24.04_desktop___"
        );
        assert_eq!(sanitize_torrent_stem(Some("Ubuntu 24.04_desktop___.TORRENT")), "Ubuntu 24.04_desktop___");
        assert_eq!(sanitize_torrent_stem(None), "torrent");
        assert_eq!(sanitize_torrent_stem(Some("  ...  ")), "torrent");
        assert_eq!(sanitize_torrent_stem(Some(".torrent")), "torrent");
    }

    #[test]
    fn torrent_temp_path_keeps_torrent_extension() {
        let path = torrent_temp_path("demo");
        assert!(path.to_string_lossy().ends_with(".torrent"));
        assert!(path.file_name().unwrap().to_string_lossy().starts_with("demo-"));
    }

    #[test]
    fn base64_torrent_is_written_with_torrent_suffix() {
        use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
        let body = b"d8:announce14:http://tracker4:infod4:name4:testee";
        let path = write_torrent_from_base64(Some("Some Release.torrent"), &B64.encode(body))
            .expect("base64 torrent should be written");
        assert!(path.to_string_lossy().ends_with(".torrent"));
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("Some Release-"));
        assert_eq!(std::fs::read(&path).unwrap(), body);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn base64_garbage_and_html_are_rejected() {
        assert!(write_torrent_from_base64(Some("x"), "not base64!!").is_err());
        use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
        let html = B64.encode(b"<!doctype html><title>login</title>");
        let err = write_torrent_from_base64(Some("x"), &html).expect_err("html must not pass");
        assert!(err.contains("网页"));
    }

    #[test]
    fn empty_base64_torrent_fails() {
        let result = open_torrent_data_inner(Some("x".into()), "   ".into());
        assert!(!result.success);
    }
}
