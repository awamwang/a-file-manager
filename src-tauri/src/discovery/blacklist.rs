// SPDX-License-Identifier: GPL-3.0-or-later
//! Directory blacklist helpers for Discovery (AIA-61).

use super::types::DiscoveryHit;

/// 7.21: this value is persisted and shown back in the Settings textarea, so
/// it is a real host path, not just a comparison key. Delegate separator
/// handling to the platform adapter; POSIX paths must not become `\Users\…`.
pub fn normalize_blacklist_path(path: &str) -> String {
    let trimmed = path.trim().trim_end_matches(['\\', '/']);
    crate::platform::to_native_fs_path(trimmed)
}

pub fn sanitize_blacklist(entries: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for entry in entries {
        let normalized = normalize_blacklist_path(entry);
        if normalized.is_empty() {
            continue;
        }
        if out
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&normalized))
        {
            continue;
        }
        out.push(normalized);
    }
    out
}

pub fn path_is_blacklisted(path: &str, blacklist: &[String]) -> bool {
    if blacklist.is_empty() {
        return false;
    }
    let candidate = normalize_blacklist_path(path).to_ascii_lowercase();
    blacklist.iter().any(|entry| {
        let prefix = normalize_blacklist_path(entry).to_ascii_lowercase();
        if !candidate.starts_with(&prefix) {
            return false;
        }
        // A folder prefix only matches on a path boundary. Accept both
        // separators so Windows `C:\…` still matches `C:/…` (legacy tests)
        // while POSIX `/Users/…` works with `/`.
        if candidate.len() == prefix.len() {
            return true;
        }
        matches!(candidate.as_bytes()[prefix.len()], b'/' | b'\\')
    })
}

pub fn filter_hits(hits: Vec<DiscoveryHit>, blacklist: &[String]) -> Vec<DiscoveryHit> {
    if blacklist.is_empty() {
        return hits;
    }
    hits.into_iter()
        .filter(|hit| !path_is_blacklisted(&hit.path, blacklist))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 7.21: Windows accepts `/` as an alias for `\`; POSIX treats a
    /// backslash as a legal filename character, so the fixtures must follow
    /// the host (same approach as the visit_stats test in 7.20).
    #[test]
    fn blacklist_matches_prefix() {
        let (folder, child, alias, unrelated) = if cfg!(windows) {
            (
                r"C:\Windows",
                r"C:\Windows\System32\cmd.exe",
                r"C:/Windows",
                r"C:\Temp\afm-fixture\demo",
            )
        } else {
            (
                "/Users/me/Library",
                "/Users/me/Library/Caches/a.txt",
                "/Users/me/Library/",
                "/Users/me/Documents",
            )
        };
        let list = sanitize_blacklist(&[folder.into(), "  ".into()]);
        assert_eq!(list, vec![folder.to_string()]);
        assert!(path_is_blacklisted(child, &list));
        assert!(path_is_blacklisted(alias, &list));
        assert!(!path_is_blacklisted(unrelated, &list));
    }

    #[cfg(not(windows))]
    #[test]
    fn posix_blacklist_stays_posix_and_matches_on_boundary() {
        let list = sanitize_blacklist(&["/Users/wangmeng/Library".into()]);
        assert_eq!(list, vec!["/Users/wangmeng/Library".to_string()]);
        assert!(path_is_blacklisted(
            "/Users/wangmeng/Library/Caches/a.txt",
            &list
        ));
        assert!(!path_is_blacklisted(
            "/Users/wangmeng/LibraryOther/a.txt",
            &list
        ));
        assert!(!path_is_blacklisted("/Users/wangmeng/Documents", &list));
    }

    #[test]
    fn filter_hits_drops_blacklisted() {
        let (blocked, blocked_hit, allowed_hit) = if cfg!(windows) {
            (r"C:\Windows", r"C:\Windows\a.txt", r"D:\Work\b.txt")
        } else {
            (
                "/Users/me/Library",
                "/Users/me/Library/a.txt",
                "/Users/me/Documents/b.txt",
            )
        };
        let hits = vec![
            DiscoveryHit {
                name: "a.txt".into(),
                path: blocked_hit.into(),
                is_dir: false,
            },
            DiscoveryHit {
                name: "b.txt".into(),
                path: allowed_hit.into(),
                is_dir: false,
            },
        ];
        let list = sanitize_blacklist(&[blocked.into()]);
        let filtered = filter_hits(hits, &list);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "b.txt");
    }
}
