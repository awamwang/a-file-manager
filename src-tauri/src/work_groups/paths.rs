// SPDX-License-Identifier: GPL-3.0-or-later

/// Windows 本地路径统一为 `\`；`afm://` 等 URI 方案保持原样。
pub fn to_native_fs_path(path: &str) -> String {
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
    // `afm://guide` / `http://…` — 不改分隔符
    if let Some(idx) = trimmed.find(':') {
        let scheme = &trimmed[..idx];
        if !scheme.is_empty()
            && scheme
                .bytes()
                .enumerate()
                .all(|(i, b)| b.is_ascii_alphanumeric() || b == b'+' || b == b'-' || b == b'.' || (i > 0 && b == b'_'))
            && trimmed[idx + 1..].starts_with("//")
        {
            return trimmed.trim_end_matches('/').to_string();
        }
    }
    let mut native = trimmed.replace('/', "\\");
    while native.ends_with('\\') {
        native.pop();
    }
    native
}

pub fn normalize_path_for_comparison(path: &str) -> String {
    to_native_fs_path(path).to_ascii_lowercase()
}

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
