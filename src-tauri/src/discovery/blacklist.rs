// SPDX-License-Identifier: GPL-3.0-or-later
//! Directory blacklist helpers for Discovery (AIA-61).

use super::types::DiscoveryHit;

pub fn normalize_blacklist_path(path: &str) -> String {
    path.trim().trim_end_matches(['\\', '/']).replace('/', "\\")
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
        let prefix = entry.to_ascii_lowercase();
        candidate == prefix
            || candidate.starts_with(&(prefix.clone() + "\\"))
            || candidate.starts_with(&(prefix + "/"))
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

    #[test]
    fn blacklist_matches_prefix() {
        let list = sanitize_blacklist(&[r"C:\Windows".into(), "  ".into()]);
        assert_eq!(list, vec![r"C:\Windows".to_string()]);
        assert!(path_is_blacklisted(r"C:\Windows\System32\cmd.exe", &list));
        assert!(path_is_blacklisted(r"C:/Windows", &list));
        assert!(!path_is_blacklisted(r"C:\Temp\afm-fixture\demo", &list));
    }

    #[test]
    fn filter_hits_drops_blacklisted() {
        let hits = vec![
            DiscoveryHit {
                name: "a.txt".into(),
                path: r"C:\Windows\a.txt".into(),
                is_dir: false,
            },
            DiscoveryHit {
                name: "b.txt".into(),
                path: r"D:\Work\b.txt".into(),
                is_dir: false,
            },
        ];
        let list = vec![r"C:\Windows".to_string()];
        let filtered = filter_hits(hits, &list);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "b.txt");
    }
}
