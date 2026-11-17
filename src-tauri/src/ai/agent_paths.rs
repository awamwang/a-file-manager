// SPDX-License-Identifier: GPL-3.0-or-later
//! Agent 路径兼容层（AIA-215 / A2 · macOS port fix）。
//!
//! 背景：内置 Agent 预设（`agents::BUILTIN_DEFS`）历史上以 Windows 约定记录
//! ——`%USERPROFILE%\.cursor\skills`——反斜杠在 Windows 是分隔符，但在 POSIX
//! 是**合法文件名字符**。平台通用的 `paths::to_native_fs_path()` 故意不改写它，
//! 直接展开会得到 `/Users/x\.cursor\skills` 这种不存在的路径，于是 macOS 上
//! 所有内置 Agent 的 `discovered` 恒为 false，Skill/MCP 部署也写到错位置。
//!
//! 这一层只服务「AI 配置里的 Agent 路径」这一类受控字符串：
//! - 允许混用 `\` / `/` 两种分隔符，按宿主 `path_separator()` 归一；
//! - 允许 `%USERPROFILE%` / `%APPDATA%` / `%LOCALAPPDATA%` / `~` 四类 token；
//! - 保留 URI 与 Windows 盘符 / UNC 语义（委托 `to_native_fs_path` 处理）。
//!
//! 不要在这里改写通用文件系统路径语义——那是 `crate::paths` / `platform` 的职责。

use std::path::Path;

/// 大小写不敏感地替换 token 的首次出现；未找到时原样返回。
fn replace_token_once(text: &str, token: &str, value: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let needle = token.to_ascii_lowercase();
    if let Some(idx) = lower.find(&needle) {
        format!("{}{}{}", &text[..idx], value, &text[idx + token.len()..])
    } else {
        text.to_string()
    }
}

/// 把配置中的逻辑分隔符（`/` 与 `\`）统一成宿主分隔符。
///
/// 注意：这**不是**通用路径归一化。POSIX 上反斜杠本可以是文件名的一部分，
/// 但 Agent 预设/user-supplied 配置里的反斜杠一律按分隔符解释（否则 Windows
/// 预设在 Mac 上必然失效）。URI 保持原样，交由 `to_native_fs_path` 收尾。
fn normalize_separators(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || crate::platform::has_uri_scheme(trimmed) {
        return trimmed.to_string();
    }
    let host_sep = crate::platform::current().path_separator();
    trimmed
        .chars()
        .map(|c| if c == '/' || c == '\\' { host_sep } else { c })
        .collect()
}

/// 展开 Agent 配置路径为宿主原生路径。
///
/// 处理顺序：环境变量 token → `~` → 逻辑分隔符归一 → `to_native_fs_path`。
/// 展开后为空字符串表示「未配置」，调用方据此跳过（与旧行为一致）。
pub fn expand_agent_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut out = trimmed.to_string();
    let home = crate::platform::home_dir().map(|p| p.to_string_lossy().into_owned());
    let base = directories::BaseDirs::new();

    for (key, env_key, fallback) in [
        ("%USERPROFILE%", "USERPROFILE", home.clone()),
        (
            "%APPDATA%",
            "APPDATA",
            base.as_ref()
                .map(|b| b.config_dir().to_string_lossy().into_owned()),
        ),
        (
            "%LOCALAPPDATA%",
            "LOCALAPPDATA",
            base.as_ref()
                .map(|b| b.cache_dir().to_string_lossy().into_owned()),
        ),
    ] {
        if out.to_ascii_uppercase().contains(key) {
            if let Some(val) = std::env::var(env_key).ok().or(fallback) {
                out = replace_token_once(&out, key, &val);
            }
        }
    }

    if let Some(rest) = out.strip_prefix('~') {
        if let Some(home) = home.as_ref() {
            let rest = rest.trim_start_matches(['/', '\\']);
            out = if rest.is_empty() {
                home.clone()
            } else {
                let sep = crate::platform::current().path_separator();
                format!("{home}{sep}{rest}")
            };
        }
    }

    let normalized = normalize_separators(&out);
    crate::paths::to_native_fs_path(&normalized)
}

/// 路径存在性探测（展开后判断）；空路径视为不存在。
pub fn path_exists_expanded(raw: &str) -> bool {
    let expanded = expand_agent_path(raw);
    !expanded.is_empty() && Path::new(&expanded).exists()
}

/// 把配置给出的相对路径拼到工作区根。统一走这一层，避免各调用点各自
/// `replace('/', "\\")`——那会把 POSIX 绝对路径变成错误的相对路径（7.20）。
pub fn join_agent_relative(root: &Path, rel: &str) -> std::path::PathBuf {
    crate::paths::join_relative(root, rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host_join(parts: &[&str]) -> String {
        let sep = crate::platform::current().path_separator();
        parts.join(&sep.to_string())
    }

    #[test]
    fn expands_windows_style_builtin_to_host_separators() {
        let expanded = expand_agent_path(r"%USERPROFILE%\.cursor\skills");
        assert!(!expanded.contains('%'), "token must be expanded");
        assert!(
            !expanded.contains('\\') || cfg!(windows),
            "backslash must not leak onto POSIX: {expanded}"
        );
        assert!(expanded.ends_with(&host_join(&[".cursor", "skills"])));
    }

    #[test]
    fn expands_posix_style_builtin_on_any_host() {
        let expanded = expand_agent_path("%USERPROFILE%/.claude/skills");
        assert!(!expanded.contains('%'));
        assert!(expanded.ends_with(&host_join(&[".claude", "skills"])));
    }

    #[test]
    fn expands_tilde_with_windows_style_tail() {
        let expanded = expand_agent_path(r"~\.workbuddy\mcp.json");
        assert!(!expanded.contains('~'));
        assert!(expanded.ends_with(&host_join(&[".workbuddy", "mcp.json"])));
    }

    #[test]
    fn empty_and_uri_are_preserved() {
        assert_eq!(expand_agent_path("  "), "");
        assert_eq!(expand_agent_path("afm://guide"), "afm://guide");
    }

    #[test]
    fn join_relative_splits_both_separator_styles() {
        let root = Path::new("/tmp/afm-ws");
        let joined = join_agent_relative(root, r".cursor\skills");
        let tail = host_join(&[".cursor", "skills"]);
        assert!(
            joined.to_string_lossy().ends_with(&tail),
            "joined={}",
            joined.display()
        );
    }
}
