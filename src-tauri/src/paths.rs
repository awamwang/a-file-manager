// SPDX-License-Identifier: GPL-3.0-or-later
//! 共享路径键：Windows 原生 `\`；URI 方案不改分隔符。

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
            && scheme.bytes().enumerate().all(|(i, b)| {
                b.is_ascii_alphanumeric()
                    || b == b'+'
                    || b == b'-'
                    || b == b'.'
                    || (i > 0 && b == b'_')
            })
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
