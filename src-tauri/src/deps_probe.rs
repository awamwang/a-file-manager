// SPDX-License-Identifier: GPL-3.0-or-later
//! 运行期依赖轻量探测：必装项 + 可选增强组件。
//! 供设置 → 健康页展示「缺什么 / 怎么补」。

use crate::ide_launch;
use crate::providers::gateway;
use crate::providers::rclone::BlockingRcloneRunner;
use crate::terminal;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DependencyItem {
    /// 稳定 id，供 i18n / 测试引用。
    pub id: String,
    /// required | optional
    pub kind: String,
    /// ok | missing | warn
    pub status: String,
    /// 用户可读标题（英文键由前端 i18n 覆盖；此处为后端兜底文案）。
    pub title: String,
    /// 缺什么 / 当前状态说明。
    pub detail: String,
    /// 建议处理方式。
    pub fix: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DependencyReport {
    pub generated_at_ms: u64,
    pub items: Vec<DependencyItem>,
    pub required_ok: bool,
    pub missing_required: usize,
    pub missing_optional: usize,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn item(
    id: &str,
    kind: &str,
    status: &str,
    title: impl Into<String>,
    detail: impl Into<String>,
    fix: impl Into<String>,
) -> DependencyItem {
    DependencyItem {
        id: id.to_string(),
        kind: kind.to_string(),
        status: status.to_string(),
        title: title.into(),
        detail: detail.into(),
        fix: fix.into(),
    }
}

fn webview2_installed() -> bool {
    #[cfg(windows)]
    {
        let candidates = [
            PathBuf::from(r"C:\Program Files (x86)\Microsoft\EdgeWebView\Application"),
            PathBuf::from(r"C:\Program Files\Microsoft\EdgeWebView\Application"),
        ];
        if candidates.iter().any(|p| p.is_dir()) {
            return true;
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local)
                .join("Microsoft")
                .join("EdgeWebView")
                .join("Application");
            if p.is_dir() {
                return true;
            }
        }
        // 能跑到本命令通常已有 WebView2；注册表再兜一层。
        webview2_registry_present()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(windows)]
fn webview2_registry_present() -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ,
    };

    const GUID: &str = r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BCC-447A6A3E3E1C}";
    const GUID_WOW: &str =
        r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BCC-447A6A3E3E1C}";

    fn open(root: windows::Win32::System::Registry::HKEY, sub: &str) -> bool {
        let wide: Vec<u16> = sub.encode_utf16().chain(std::iter::once(0)).collect();
        let mut key = windows::Win32::System::Registry::HKEY::default();
        let status =
            unsafe { RegOpenKeyExW(root, PCWSTR(wide.as_ptr()), Some(0), KEY_READ, &mut key) };
        if status == ERROR_SUCCESS {
            unsafe {
                let _ = RegCloseKey(key);
            }
            true
        } else {
            false
        }
    }

    open(HKEY_LOCAL_MACHINE, GUID)
        || open(HKEY_LOCAL_MACHINE, GUID_WOW)
        || open(HKEY_CURRENT_USER, GUID)
}

fn knowledge_root_status() -> DependencyItem {
    let configured = std::env::var("AFM_KNOWLEDGE_ROOT")
        .or_else(|_| std::env::var("AWAM_KNOWLEDGE_ROOT"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    match configured {
        None => item(
            "knowledge_root",
            "optional",
            "missing",
            "Knowledge root",
            "未配置 AFM_KNOWLEDGE_ROOT / AWAM_KNOWLEDGE_ROOT；内置 knowledge 目录不会绑定本机盘符。",
            "若要用本地知识库，设置环境变量指向数据目录后重启；或通过 awam-file 网关提供 knowledge 后端。",
        ),
        Some(path) => {
            let p = PathBuf::from(&path);
            if p.is_dir() {
                item(
                    "knowledge_root",
                    "optional",
                    "ok",
                    "Knowledge root",
                    format!("已配置且目录存在：{path}"),
                    "无需处理。",
                )
            } else {
                item(
                    "knowledge_root",
                    "optional",
                    "warn",
                    "Knowledge root",
                    format!("已配置但目录不存在：{path}"),
                    "创建该目录，或修正环境变量路径后重启应用。",
                )
            }
        }
    }
}

fn rclone_status() -> DependencyItem {
    let runner = BlockingRcloneRunner::new();
    match (&runner.exe, &runner.missing_message) {
        (Some(exe), _) => item(
            "rclone",
            "optional",
            "ok",
            "rclone CLI",
            format!("已找到：{}", exe.display()),
            "无需处理。也可用 AWAM_RCLONE_EXE / RCLONE_EXE 指定路径。",
        ),
        (_, Some(msg)) => item(
            "rclone",
            "optional",
            "missing",
            "rclone CLI",
            msg.clone(),
            "安装 rclone 并加入 PATH，或设置 AWAM_RCLONE_EXE；然后执行 rclone config。",
        ),
        _ => item(
            "rclone",
            "optional",
            "missing",
            "rclone CLI",
            "未找到 rclone。",
            "安装 rclone 并加入 PATH。",
        ),
    }
}

fn gateway_status() -> DependencyItem {
    let st = gateway::status();
    if st.online {
        item(
            "awam_gateway",
            "optional",
            "ok",
            "awam-file gateway",
            format!(
                "在线 · {} · backends={}",
                st.base_url,
                st.backend_count.unwrap_or(0)
            ),
            "无需处理。可在 设置 → 高级 修改网关地址，或用 AWAM_GATEWAY_URL 覆盖。",
        )
    } else {
        item(
            "awam_gateway",
            "optional",
            "missing",
            "awam-file gateway",
            format!(
                "离线 · {} · {}",
                st.base_url,
                st.detail.unwrap_or_else(|| "unreachable".to_string())
            ),
            "启动本机 awam-file 网关，或在 设置 → 高级 修改地址（默认 http://127.0.0.1:18787）。离线时仍使用内置 rclone/browser。",
        )
    }
}

fn peek_status() -> DependencyItem {
    let candidates = [
        PathBuf::from(r"C:\Program Files\PowerToys\WinUI3Apps\PowerToys.Peek.UI.exe"),
        PathBuf::from(r"C:\Program Files (x86)\PowerToys\WinUI3Apps\PowerToys.Peek.UI.exe"),
    ];
    let mut found = candidates.into_iter().find(|p| p.is_file());
    if found.is_none() {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local)
                .join("PowerToys")
                .join("WinUI3Apps")
                .join("PowerToys.Peek.UI.exe");
            if p.is_file() {
                found = Some(p);
            }
        }
    }
    match found {
        Some(p) => item(
            "powertoys_peek",
            "optional",
            "ok",
            "PowerToys Peek",
            format!("已找到：{}", p.display()),
            "无需处理。",
        ),
        None => item(
            "powertoys_peek",
            "optional",
            "missing",
            "PowerToys Peek",
            "未安装 PowerToys Peek（外部预览第二顺位）。",
            "安装 Microsoft PowerToys 并启用 Peek；缺失时 Space 预览会回退到内置预览。",
        ),
    }
}

fn quicklook_status() -> DependencyItem {
    #[cfg(windows)]
    {
        // 轻量探测：尝试打开命名管道（不写消息）。管道名与 preview_bridge 一致。
        use std::fs::OpenOptions;
        let sid = match current_user_sid_string() {
            Ok(s) => s,
            Err(err) => {
                return item(
                    "quicklook",
                    "optional",
                    "warn",
                    "QuickLook",
                    format!("无法解析用户 SID：{err}"),
                    "安装并运行 QuickLook；或依赖 Peek / 内置预览。",
                );
            }
        };
        let pipe = format!(r"\\.\pipe\QuickLook.App.Pipe.{sid}");
        match OpenOptions::new().write(true).open(&pipe) {
            Ok(_) => item(
                "quicklook",
                "optional",
                "ok",
                "QuickLook",
                format!("命名管道可用：{pipe}"),
                "无需处理。",
            ),
            Err(_) => item(
                "quicklook",
                "optional",
                "missing",
                "QuickLook",
                "未检测到 QuickLook 命名管道（通常表示未运行）。",
                "安装并启动 QuickLook；缺失时会尝试 Peek，再回退内置预览。",
            ),
        }
    }
    #[cfg(not(windows))]
    {
        item(
            "quicklook",
            "optional",
            "missing",
            "QuickLook",
            "仅 Windows 支持。",
            "在 Windows 上安装 QuickLook。",
        )
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
            .map_err(|e| format!("OpenProcessToken: {e}"))?;
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
                return Err(format!("SID decode: {err}"));
            }
        };
        let _ = LocalFree(Some(HLOCAL(sid_pwstr.as_ptr().cast())));
        let _ = CloseHandle(token);
        Ok(sid)
    }
}

fn browser_bookmarks_status() -> DependencyItem {
    let local = match std::env::var_os("LOCALAPPDATA") {
        Some(v) => PathBuf::from(v),
        None => {
            return item(
                "browser_bookmarks",
                "optional",
                "warn",
                "Browser bookmarks",
                "LOCALAPPDATA 不可用。",
                "在常规用户会话下运行。",
            );
        }
    };
    let roots = [
        local.join(r"Microsoft\Edge\User Data"),
        local.join(r"Google\Chrome\User Data"),
    ];
    let mut hits = Vec::new();
    for root in &roots {
        if !root.is_dir() {
            continue;
        }
        // 浅扫常见 profile
        for name in ["Default", "Profile 1", "Profile 2"] {
            let bookmarks = root.join(name).join("Bookmarks");
            if bookmarks.is_file() {
                hits.push(bookmarks);
            }
        }
    }
    if hits.is_empty() {
        item(
            "browser_bookmarks",
            "optional",
            "missing",
            "Browser bookmarks",
            "未找到 Edge/Chrome Bookmarks 文件。",
            "安装并登录 Edge 或 Chrome；缺失时浏览器收藏 Provider 为空，不影响本地盘浏览。",
        )
    } else {
        item(
            "browser_bookmarks",
            "optional",
            "ok",
            "Browser bookmarks",
            format!("找到 {} 个书签文件。", hits.len()),
            "无需处理。",
        )
    }
}

fn terminal_status() -> DependencyItem {
    let apps = terminal::terminal_list_apps();
    let available: Vec<_> = apps.iter().filter(|a| a.available).map(|a| a.id.as_str()).collect();
    if available.is_empty() {
        item(
            "terminals",
            "optional",
            "warn",
            "Terminals",
            "未探测到可用终端（异常；Windows 通常至少有 cmd/powershell）。",
            "确认 cmd.exe / powershell.exe 在系统目录。",
        )
    } else {
        item(
            "terminals",
            "optional",
            "ok",
            "Terminals",
            format!("可用：{}", available.join(", ")),
            "无需处理。可选安装 Windows Terminal / Git Bash。",
        )
    }
}

fn ide_status() -> DependencyItem {
    let apps = ide_launch::ide_probe_auto();
    let available: Vec<_> = apps
        .iter()
        .filter(|(_, ok, _)| *ok)
        .map(|(id, _, _)| id.as_str())
        .collect();
    if available.is_empty() {
        item(
            "ides",
            "optional",
            "missing",
            "Editors / IDEs",
            "未找到 VS Code / Cursor / Trae / Qoder。",
            "安装对应 IDE，或在设置 → 打开工具 中填写绝对路径。仅影响「用编辑器打开」。",
        )
    } else {
        item(
            "ides",
            "optional",
            "ok",
            "Editors / IDEs",
            format!("可用：{}", available.join(", ")),
            "无需处理。",
        )
    }
}

/// 汇总探测结果。
pub fn probe_dependencies() -> DependencyReport {
    let mut items = Vec::new();

    // 必装：WebView2（安装包会自动拉起 bootstrapper；此处供体检页）
    if webview2_installed() {
        items.push(item(
            "webview2",
            "required",
            "ok",
            "WebView2 Runtime",
            "已检测到 Microsoft Edge WebView2 Runtime。",
            "无需处理。NSIS 安装包在缺失时会自动运行官方 bootstrapper。",
        ));
    } else {
        items.push(item(
            "webview2",
            "required",
            "missing",
            "WebView2 Runtime",
            "未检测到 WebView2 Runtime；界面宿主不可用。",
            "重新运行安装包（会自动安装），或手动安装 Evergreen WebView2 Runtime：https://developer.microsoft.com/microsoft-edge/webview2/",
        ));
    }

    items.push(gateway_status());
    items.push(rclone_status());
    items.push(knowledge_root_status());
    items.push(quicklook_status());
    items.push(peek_status());
    items.push(browser_bookmarks_status());
    items.push(terminal_status());
    items.push(ide_status());

    if cfg!(feature = "ai-addon") {
        items.push(item(
            "ai_addon",
            "optional",
            "ok",
            "AI Add-on package",
            "当前为 AI 包构建（含 Agent / MCP / Skills 面板）。",
            "不需要这些能力可用 Core 包构建（--no-default-features / npm run build:core）。MCP 请自行导入，不再植入本机绝对路径。",
        ));
    } else {
        items.push(item(
            "ai_addon",
            "optional",
            "missing",
            "AI Add-on package",
            "当前为 Core 包（未编译 AI Add-on）。",
            "需要 AI 功能请使用默认（AI）构建。",
        ));
    }

    let missing_required = items
        .iter()
        .filter(|i| i.kind == "required" && i.status != "ok")
        .count();
    let missing_optional = items
        .iter()
        .filter(|i| i.kind == "optional" && i.status == "missing")
        .count();

    DependencyReport {
        generated_at_ms: now_ms(),
        items,
        required_ok: missing_required == 0,
        missing_required,
        missing_optional,
    }
}

#[tauri::command]
pub fn deps_probe_get() -> DependencyReport {
    probe_dependencies()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_returns_webview_and_optionals() {
        let report = probe_dependencies();
        assert!(!report.items.is_empty());
        assert!(report.items.iter().any(|i| i.id == "webview2"));
        assert!(report.items.iter().any(|i| i.id == "rclone"));
        assert!(report.items.iter().any(|i| i.id == "awam_gateway"));
        assert!(report.items.iter().any(|i| i.id == "knowledge_root"));
    }
}
