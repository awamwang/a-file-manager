// SPDX-License-Identifier: GPL-3.0-or-later

use super::types::AwamUri;
use std::collections::HashMap;

/// Parse `awam://<backend>[/<path>][?<query>]`.
/// Bare OS paths are **rejected** here — Kernel browse stays on NavigatorService (ADR 0001).
pub fn parse_awam_uri(input: &str) -> Result<AwamUri, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(AwamUri {
            backend: "root".to_string(),
            path: String::new(),
            query: HashMap::new(),
        });
    }

    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("awam://") {
        return Err(
            "ProviderRegistry expects awam:// URIs; OS paths belong to NavigatorService (ADR 0001)"
                .to_string(),
        );
    }

    let rest = &trimmed["awam://".len()..];
    let (authority_path, query_str) = match rest.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (rest, None),
    };

    let authority_path = authority_path.trim_start_matches('/');
    let (backend, path) = if authority_path.is_empty() {
        ("root".to_string(), String::new())
    } else if let Some((backend, path)) = authority_path.split_once('/') {
        (backend.to_ascii_lowercase(), path.to_string())
    } else {
        (authority_path.to_ascii_lowercase(), String::new())
    };

    if backend.is_empty() {
        return Err("awam:// URI missing backend".to_string());
    }

    let mut query = HashMap::new();
    if let Some(query_str) = query_str {
        for pair in query_str.split('&') {
            if pair.is_empty() {
                continue;
            }
            if let Some((key, value)) = pair.split_once('=') {
                query.insert(key.to_string(), value.to_string());
            } else {
                query.insert(pair.to_string(), String::new());
            }
        }
    }

    Ok(AwamUri {
        backend,
        path,
        query,
    })
}

impl AwamUri {
    pub fn to_string_uri(&self) -> String {
        let mut uri = if self.path.is_empty() {
            format!("awam://{}", self.backend)
        } else {
            format!("awam://{}/{}", self.backend, self.path)
        };
        if !self.query.is_empty() {
            let mut pairs: Vec<String> = self
                .query
                .iter()
                .map(|(k, v)| {
                    if v.is_empty() {
                        k.clone()
                    } else {
                        format!("{k}={v}")
                    }
                })
                .collect();
            pairs.sort();
            uri.push('?');
            uri.push_str(&pairs.join("&"));
        }
        uri
    }

    pub fn child(&self, name: &str) -> Self {
        let path = if self.path.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", self.path.trim_end_matches('/'), name)
        };
        Self {
            backend: self.backend.clone(),
            path,
            query: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rclone_and_knowledge() {
        let uri = parse_awam_uri("awam://rclone/mysftp/var").unwrap();
        assert_eq!(uri.backend, "rclone");
        assert_eq!(uri.path, "mysftp/var");

        let uri = parse_awam_uri("awam://knowledge/logseq-main/_flat").unwrap();
        assert_eq!(uri.backend, "knowledge");
        assert_eq!(uri.path, "logseq-main/_flat");
    }

    #[test]
    fn root_aliases() {
        assert_eq!(parse_awam_uri("").unwrap().backend, "root");
        assert_eq!(parse_awam_uri("awam://").unwrap().backend, "root");
        assert_eq!(parse_awam_uri("awam://root/").unwrap().backend, "root");
    }

    #[test]
    fn rejects_os_paths() {
        let err = parse_awam_uri(r"G:\Projects").unwrap_err();
        assert!(err.contains("NavigatorService"));
    }
}
