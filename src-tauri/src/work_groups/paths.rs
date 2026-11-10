// SPDX-License-Identifier: GPL-3.0-or-later
//! Work Group 路径助手 — 委托共享 `crate::paths`。

pub use crate::paths::{normalize_path_for_comparison, to_native_fs_path};

pub fn is_exact_member_path(path: &str, member_paths: &[String]) -> bool {
    let comparison = normalize_path_for_comparison(path);
    if comparison.is_empty() {
        return false;
    }
    member_paths
        .iter()
        .any(|member| normalize_path_for_comparison(member) == comparison)
}

pub fn unique_member_paths(paths: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::new();
    for path in paths {
        let native = to_native_fs_path(path);
        let comparable = normalize_path_for_comparison(&native);
        if comparable.is_empty() || !seen.insert(comparable) {
            continue;
        }
        unique.push(native);
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_and_exact_match_use_backslash() {
        let paths = vec![
            r"D:\Projects".to_string(),
            "d:/projects".to_string(),
            r"D:\Other".to_string(),
        ];
        let unique = unique_member_paths(&paths);
        assert_eq!(unique.len(), 2);
        assert_eq!(unique[0], r"D:\Projects");
        assert_eq!(unique[1], r"D:\Other");
        assert!(is_exact_member_path(r"D:\Projects", &unique));
        assert!(is_exact_member_path("d:/projects", &unique));
    }

    #[test]
    fn guide_uri_not_rewritten() {
        assert_eq!(to_native_fs_path("afm://guide"), "afm://guide");
        assert_eq!(normalize_path_for_comparison("afm://guide"), "afm://guide");
    }
}
