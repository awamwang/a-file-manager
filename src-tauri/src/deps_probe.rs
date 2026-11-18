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

#[cfg_attr(not(windows), allow(dead_code))]
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

    const GUID: &str =
        r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BCC-447A6A3E3E1C}";
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
            "启动本机 awam-file 网关，或在 设置 → 高级 修改地址（默认 http://127.0.0.1:18787）。离线时仍使用内置 rclone；浏览器书签需网关在线。",
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
        // 单一判定来自 preview_bridge::quicklook_available()（管道存在性，无副作用，
        // 与桥接 auto 回退共用）。busy / 拒绝访问已在该函数内归为 Running，不再误报
        // 缺失；仅「管道不存在」可能是 QuickLook 正在启动的瞬间，做短暂重试消抖。
        use crate::preview_bridge::quicklook_available;
        use crate::preview_bridge::QuickLookStatus;
        let mut status = quicklook_available();
        for _ in 0..2u8 {
            // 仅对「未运行 / 进程在但管道未就绪」消抖重试；Running / Indeterminate 立即定论。
            if matches!(
                status,
                QuickLookStatus::Running | QuickLookStatus::Indeterminate
            ) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(40));
            status = quicklook_available();
        }
        match status {
            QuickLookStatus::Running => item(
                "quicklook",
                "optional",
                "ok",
                "QuickLook",
                "检测到 QuickLook 正在运行（命名管道存在）。空格预览会交给它。",
                "无需处理；可预览的文件类型取决于 QuickLook 插件，管道瞬时繁忙时会自动回退 Peek / 内置。",
            ),
            QuickLookStatus::PresentUnbridged => item(
                "quicklook",
                "optional",
                "unbridged",
                "QuickLook",
                "检测到 QuickLook 进程在运行，但没有 AFM 可桥接的命名管道（AFM 仅支持经典 QuickLook 的管道协议；商店版 QuickLook 不提供该管道）。",
                "无需处理：当前空格预览由 PowerToys Peek（或内置预览）提供。要让 AFM 直接驱动 QuickLook，请改用经典 poizan42 QuickLook。",
            ),
            QuickLookStatus::NotRunning => item(
                "quicklook",
                "optional",
                "missing",
                "QuickLook",
                "未检测到 QuickLook 在运行（命名管道不存在；已安装但未启动时同样如此）。",
                "启动 QuickLook 后即可接管预览；当前空格会自动尝试 Peek，再回退内置预览。",
            ),
            QuickLookStatus::Indeterminate => item(
                "quicklook",
                "optional",
                "warn",
                "QuickLook",
                "无法确定 QuickLook 状态：未能解析当前用户 SID。",
                "在常规用户会话下运行；若仍不确定，可依赖 Peek 或内置预览。",
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

fn browser_bookmarks_status() -> DependencyItem {
    // Host roots come from the platform contract, never a hardcoded %VAR%.
    let (roots, names): (Vec<PathBuf>, Vec<&str>) = if cfg!(windows) {
        let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
            return item(
                "browser_bookmarks",
                "optional",
                "warn",
                "Browser bookmarks",
                "LOCALAPPDATA 不可用。",
                "在常规用户会话下运行。",
            );
        };
        (
            vec![
                local.join(r"Microsoft\Edge\User Data"),
                local.join(r"Google\Chrome\User Data"),
            ],
            vec!["Default", "Profile 1", "Profile 2"],
        )
    } else {
        let Some(home) = crate::platform::home_dir() else {
            return item(
                "browser_bookmarks",
                "optional",
                "warn",
                "Browser bookmarks",
                "无法解析用户主目录。",
                "在常规用户会话下运行。",
            );
        };
        let support = home.join("Library").join("Application Support");
        (
            vec![
                support.join("Google").join("Chrome"),
                support.join("Microsoft").join("Edge"),
            ],
            vec!["Default", "Profile 1", "Profile 2"],
        )
    };
    let mut hits = Vec::new();
    for root in &roots {
        if !root.is_dir() {
            continue;
        }
        // 浅扫常见 profile
        for name in &names {
            let bookmarks = root.join(name).join("Bookmarks");
            if bookmarks.is_file() {
                hits.push(bookmarks);
            }
        }
    }
    // Safari keeps a single plist outside the Chrome-style profile layout.
    if !cfg!(windows) {
        if let Some(home) = crate::platform::home_dir() {
            let safari = home.join("Library").join("Safari").join("Bookmarks.plist");
            if safari.is_file() {
                hits.push(safari);
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
            "安装并登录 Edge 或 Chrome；书签由 awam-file 网关读取，缺失时 Provider 为空，不影响本地盘浏览。",
        )
    } else {
        item(
            "browser_bookmarks",
            "optional",
            "ok",
            "Browser bookmarks",
            format!("找到 {} 个书签文件（由 awam-file 网关提供给 AFM）。", hits.len()),
            "保持 awam-file 网关在线即可在 Providers 中浏览。",
        )
    }
}

fn terminal_status() -> DependencyItem {
    let apps = terminal::terminal_list_apps();
    // 7.20: the preset ids are platform-specific (`wt`/`cmd` vs
    // `terminal`/`iterm`), so show display names and keep the hint neutral.
    let available: Vec<_> = apps
        .iter()
        .filter(|a| a.available)
        .map(|a| a.name.as_str())
        .collect();
    if available.is_empty() {
        item(
            "terminals",
            "optional",
            "warn",
            "Terminals",
            "未探测到可用终端。",
            "确认系统终端应用可用，或在设置页为其配置启动路径。",
        )
    } else {
        item(
            "terminals",
            "optional",
            "ok",
            "Terminals",
            format!("可用：{}", available.join(", ")),
            "无需处理。",
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

    // 必装：Windows 的 WebView2（安装包会自动拉起 bootstrapper）；
    // macOS 用系统自带 WKWebView，不能把这个 Windows 专属项永远报成「缺失」。
    #[cfg(windows)]
    {
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
    }
    #[cfg(not(windows))]
    {
        items.push(item(
            "webview",
            "required",
            "ok",
            "系统 WebView",
            "WKWebView 由 macOS 系统提供，无需额外安装运行时。",
            "无需处理。",
        ));
    }

    items.push(gateway_status());
    items.push(rclone_status());
    // Capability-driven dependency list: macOS has system Quick Look and no
    // PowerToys Peek. Generate only checks that can exist on this host so the
    // health summary cannot be polluted by permanently-missing Windows items.
    let caps = crate::platform::current().capabilities();
    if caps.quick_look_preview && crate::platform::preview_provider().is_some() {
        items.push(item(
            "quicklook",
            "optional",
            "ok",
            "Quick Look",
            "Quick Look 由 macOS 系统提供。",
            "无需处理；空格即可预览选中文件。",
        ));
    } else if caps.quick_look_preview {
        items.push(quicklook_status());
    }
    if caps.peek_preview {
        items.push(peek_status());
    }
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
        #[cfg(windows)]
        assert!(report.items.iter().any(|i| i.id == "webview2"));
        #[cfg(not(windows))]
        assert!(report.items.iter().any(|i| i.id == "webview"));
        assert!(report.items.iter().any(|i| i.id == "rclone"));
        assert!(report.items.iter().any(|i| i.id == "awam_gateway"));
    }
}
