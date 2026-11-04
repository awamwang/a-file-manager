// SPDX-License-Identifier: GPL-3.0-or-later

pub fn normalize_path_for_comparison(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
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
        let comparable = normalize_path_for_comparison(path);
        if comparable.is_empty() || !seen.insert(comparable) {
            continue;
        }
        unique.push(path.clone());
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_and_exact_match_ignore_slash_case() {
        let paths = vec![
            r"D:\Projects".to_string(),
            "d:/projects".to_string(),
            r"D:\Other".to_string(),
        ];
        let unique = unique_member_paths(&paths);
        assert_eq!(unique.len(), 2);
        assert!(is_exact_member_path(r"D:\Projects", &unique));
    }
}
