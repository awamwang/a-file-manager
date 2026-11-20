// SPDX-License-Identifier: GPL-3.0-or-later
//! 共享路径键：委托给 `platform` 层，由宿主 OS 决定分隔符与比较语义；URI 方案不改。
//!
//! 之前这里无条件把 `/` 换成 `\` 并 lowercase，macOS 上会把 `/Users/...` 变成
//! `\Users\...`。现在所有调用方仍用同一组函数，但实现由 platform adapter 提供。

pub fn to_native_fs_path(path: &str) -> String {
    crate::platform::to_native_fs_path(path)
}

pub fn normalize_path_for_comparison(path: &str) -> String {
    crate::platform::normalize_for_comparison(path)
}

/// Render a `Path` for the frontend using the host OS separator.
/// Single entry point: both `navigator::list` and `navigator::places` use it.
pub fn display_path(path: &std::path::Path) -> String {
    to_native_fs_path(path.to_string_lossy().as_ref())
}

/// Join a config-supplied relative path (may contain `/` or `\`) onto a root.
/// Replaces the per-module `rel.replace('/', "\\")` copies that turn POSIX
/// absolute paths into relative ones outside Windows.
// Only the AI addon consumes this today; Lite (core flavor) builds drop those modules. See ADR 0024.
#[cfg_attr(not(feature = "ai-addon"), allow(dead_code))]
pub fn join_relative(root: &std::path::Path, rel: &str) -> std::path::PathBuf {
    let mut out = root.to_path_buf();
    for part in rel.split(['/', '\\']) {
        if part.is_empty() || part == "." {
            continue;
        }
        out.push(part);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn drive_and_uri() {
        assert_eq!(to_native_fs_path("G:/Projects/A"), r"G:\Projects\A");
        assert_eq!(to_native_fs_path(r"C:\"), r"C:\");
        assert_eq!(to_native_fs_path("afm://guide"), "afm://guide");
        assert_eq!(
            normalize_path_for_comparison("g:/projects/a"),
            r"g:\projects\a"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn posix_and_uri() {
        assert_eq!(
            to_native_fs_path("/Users/demo/Docs/"),
            "/Users/demo/Docs"
        );
        assert_eq!(to_native_fs_path("/"), "/");
        assert_eq!(to_native_fs_path("afm://guide"), "afm://guide");
        assert_eq!(
            normalize_path_for_comparison("/Users/A/File.txt"),
            "/Users/A/File.txt"
        );
    }
}
