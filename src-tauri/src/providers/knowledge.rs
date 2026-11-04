// SPDX-License-Identifier: GPL-3.0-or-later

use super::provider::Provider;
use super::types::{
    AwamUri, BackendInfo, DirListing, ProviderCapability, ProviderEntry, ResolveResult, StatResult,
};
use super::util::{metadata_modified_time_unix_ms, normalize_path};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CAPABILITIES: &[ProviderCapability] = &[
    ProviderCapability::List,
    ProviderCapability::Stat,
    ProviderCapability::Read,
    ProviderCapability::Resolve,
    ProviderCapability::Search,
];

pub struct KnowledgeProvider {
    root: PathBuf,
}

impl KnowledgeProvider {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn from_env_or_default() -> Self {
        let root = std::env::var("AFM_KNOWLEDGE_ROOT")
            .or_else(|_| std::env::var("AWAM_KNOWLEDGE_ROOT"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(r"G:\Knowledge\data"));
        Self::new(root)
    }

    fn resolve_fs(&self, uri: &AwamUri) -> PathBuf {
        if uri.path.is_empty() {
            return self.root.clone();
        }
        let parts: Vec<&str> = uri.path.split('/').filter(|part| !part.is_empty()).collect();
        let mut path = self.root.clone();
        for part in parts {
            path.push(part);
        }
        path
    }

    fn entry_for(&self, path: &Path, uri: &AwamUri) -> ProviderEntry {
        let metadata = fs::metadata(path).ok();
        let is_dir = metadata
            .as_ref()
            .map(|meta| meta.is_dir())
            .unwrap_or(path.is_dir());
        let is_file = metadata
            .as_ref()
            .map(|meta| meta.is_file())
            .unwrap_or(path.is_file());
        let size = metadata
            .as_ref()
            .map(|meta| if meta.is_file() { meta.len() } else { 0 })
            .unwrap_or(0);
        let modified_time = metadata
            .as_ref()
            .map(metadata_modified_time_unix_ms)
            .unwrap_or(0);
        ProviderEntry {
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| uri.backend.clone()),
            uri: uri.to_string_uri(),
            is_dir,
            is_file,
            size,
            modified_time,
            local_path: Some(normalize_path(&path.to_string_lossy())),
            extra: HashMap::new(),
        }
    }

    fn graph_dirs(&self) -> Vec<PathBuf> {
        let Ok(read_dir) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut graphs = Vec::new();
        for item in read_dir.flatten() {
            let path = item.path();
            let name = item.file_name().to_string_lossy().to_string();
            if path.is_dir() && !name.starts_with('.') {
                graphs.push(path);
            }
        }
        graphs.sort_by(|a, b| {
            a.file_name()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .cmp(&b.file_name().unwrap_or_default().to_ascii_lowercase())
        });
        graphs
    }

    fn list_flat(&self, uri: &AwamUri, graph_name: &str) -> DirListing {
        let graph = self.root.join(graph_name);
        let mut files: Vec<PathBuf> = Vec::new();
        let pages = graph.join("pages");
        let journals = graph.join("journals");
        if pages.is_dir() {
            if let Ok(read_dir) = fs::read_dir(pages) {
                files.extend(
                    read_dir
                        .flatten()
                        .map(|entry| entry.path())
                        .filter(|path| {
                            path.extension()
                                .and_then(|ext| ext.to_str())
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
                        }),
                );
            }
        }
        if journals.is_dir() {
            if let Ok(read_dir) = fs::read_dir(journals) {
                files.extend(
                    read_dir
                        .flatten()
                        .map(|entry| entry.path())
                        .filter(|path| {
                            path.extension()
                                .and_then(|ext| ext.to_str())
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
                        }),
                );
            }
        }
        files.sort_by(|a, b| {
            a.file_name()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .cmp(&b.file_name().unwrap_or_default().to_ascii_lowercase())
        });

        let entries = files
            .into_iter()
            .map(|path| {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();
                let child_uri = uri.child(&name);
                let mut entry = self.entry_for(&path, &child_uri);
                entry
                    .extra
                    .insert("virtual".to_string(), serde_json::Value::Bool(true));
                entry.extra.insert(
                    "flatSource".to_string(),
                    serde_json::Value::String(normalize_path(&path.to_string_lossy())),
                );
                entry
            })
            .collect();

        DirListing {
            uri: uri.to_string_uri(),
            backend: "knowledge".to_string(),
            entries,
            error: None,
        }
    }
}

impl Provider for KnowledgeProvider {
    fn backend_id(&self) -> &str {
        "knowledge"
    }
    fn name(&self) -> &str {
        "Knowledge"
    }
    fn description(&self) -> &str {
        "Graphs under knowledge root (Logseq-style tree + optional _flat)"
    }
    fn capabilities(&self) -> &[ProviderCapability] {
        CAPABILITIES
    }

    fn health(&self) -> BackendInfo {
        let ok = self.root.is_dir();
        BackendInfo {
            id: "knowledge".to_string(),
            name: self.name().to_string(),
            description: self.description().to_string(),
            healthy: ok,
            capabilities: CAPABILITIES
                .iter()
                .map(|capability| capability.as_str().to_string())
                .collect(),
            detail: if ok {
                Some(normalize_path(&self.root.to_string_lossy()))
            } else {
                Some(format!(
                    "Missing knowledge root: {}",
                    self.root.to_string_lossy()
                ))
            },
            interface_only: false,
        }
    }

    fn list(&self, uri: &AwamUri) -> DirListing {
        if !self.root.is_dir() {
            return DirListing {
                uri: uri.to_string_uri(),
                backend: "knowledge".to_string(),
                entries: Vec::new(),
                error: Some(format!(
                    "Knowledge root missing: {}",
                    self.root.to_string_lossy()
                )),
            };
        }

        if uri.path.is_empty() {
            let entries = self
                .graph_dirs()
                .into_iter()
                .map(|graph| {
                    let name = graph
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_default();
                    self.entry_for(&graph, &uri.child(&name))
                })
                .collect();
            return DirListing {
                uri: uri.to_string_uri(),
                backend: "knowledge".to_string(),
                entries,
                error: None,
            };
        }

        let parts: Vec<&str> = uri.path.split('/').filter(|part| !part.is_empty()).collect();
        if parts.len() == 2 && parts[1] == "_flat" {
            return self.list_flat(uri, parts[0]);
        }

        let path = self.resolve_fs(uri);
        if !path.exists() {
            return DirListing {
                uri: uri.to_string_uri(),
                backend: "knowledge".to_string(),
                entries: Vec::new(),
                error: Some(format!("Not found: {}", path.to_string_lossy())),
            };
        }
        if !path.is_dir() {
            return DirListing {
                uri: uri.to_string_uri(),
                backend: "knowledge".to_string(),
                entries: Vec::new(),
                error: Some(format!("Not a directory: {}", path.to_string_lossy())),
            };
        }

        let mut entries = Vec::new();
        if parts.len() == 1 {
            let mut extra = HashMap::new();
            extra.insert("virtual".to_string(), serde_json::Value::Bool(true));
            extra.insert(
                "description".to_string(),
                serde_json::Value::String("All pages flattened".to_string()),
            );
            entries.push(ProviderEntry {
                name: "_flat".to_string(),
                uri: uri.child("_flat").to_string_uri(),
                is_dir: true,
                is_file: false,
                size: 0,
                modified_time: system_now_ms(),
                local_path: None,
                extra,
            });
        }

        let Ok(read_dir) = fs::read_dir(&path) else {
            return DirListing {
                uri: uri.to_string_uri(),
                backend: "knowledge".to_string(),
                entries,
                error: Some(format!("Failed to read {}", path.to_string_lossy())),
            };
        };

        let mut children: Vec<PathBuf> = read_dir.flatten().map(|entry| entry.path()).collect();
        children.sort_by(|a, b| {
            let a_dir = a.is_dir();
            let b_dir = b.is_dir();
            match (a_dir, b_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a
                    .file_name()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .cmp(&b.file_name().unwrap_or_default().to_ascii_lowercase()),
            }
        });

        for child in children {
            let name = child
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with('.') {
                continue;
            }
            entries.push(self.entry_for(&child, &uri.child(&name)));
        }

        DirListing {
            uri: uri.to_string_uri(),
            backend: "knowledge".to_string(),
            entries,
            error: None,
        }
    }

    fn stat(&self, uri: &AwamUri) -> StatResult {
        if uri.path.ends_with("/_flat") || uri.path == "_flat" {
            let mut extra = HashMap::new();
            extra.insert("virtual".to_string(), serde_json::Value::Bool(true));
            return StatResult {
                entry: Some(ProviderEntry {
                    name: "_flat".to_string(),
                    uri: uri.to_string_uri(),
                    is_dir: true,
                    is_file: false,
                    size: 0,
                    modified_time: 0,
                    local_path: None,
                    extra,
                }),
                error: None,
            };
        }
        let path = self.resolve_fs(uri);
        if !path.exists() {
            return StatResult {
                entry: None,
                error: Some(format!("Not found: {}", path.to_string_lossy())),
            };
        }
        StatResult {
            entry: Some(self.entry_for(&path, uri)),
            error: None,
        }
    }

    fn resolve(&self, uri: &AwamUri) -> ResolveResult {
        if uri.path.ends_with("/_flat") || uri.path == "_flat" {
            return ResolveResult {
                uri: uri.to_string_uri(),
                local_path: None,
                action: None,
                action_args: HashMap::new(),
                error: Some("_flat is a virtual collection".to_string()),
            };
        }
        let path = self.resolve_fs(uri);
        if path.exists() {
            ResolveResult {
                uri: uri.to_string_uri(),
                local_path: Some(normalize_path(&path.to_string_lossy())),
                action: None,
                action_args: HashMap::new(),
                error: None,
            }
        } else {
            ResolveResult {
                uri: uri.to_string_uri(),
                local_path: None,
                action: None,
                action_args: HashMap::new(),
                error: Some(format!("Not found: {}", path.to_string_lossy())),
            }
        }
    }
}

fn system_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    #[test]
    fn lists_graphs_and_flat_virtual_view() {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-knowledge-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let graph = dir.join("logseq-main");
        fs::create_dir_all(graph.join("pages")).unwrap();
        fs::create_dir_all(graph.join("journals")).unwrap();
        File::create(graph.join("pages").join("Hello.md"))
            .unwrap()
            .write_all(b"# hi")
            .unwrap();
        File::create(graph.join("journals").join("2026-11-03.md"))
            .unwrap()
            .write_all(b"day")
            .unwrap();

        let provider = KnowledgeProvider::new(dir.clone());
        assert!(provider.health().healthy);

        let root = provider.list(&AwamUri {
            backend: "knowledge".to_string(),
            path: String::new(),
            query: HashMap::new(),
        });
        assert_eq!(root.entries.len(), 1);
        assert_eq!(root.entries[0].name, "logseq-main");
        assert!(root.entries[0].local_path.is_some());

        let graph_listing = provider.list(&AwamUri {
            backend: "knowledge".to_string(),
            path: "logseq-main".to_string(),
            query: HashMap::new(),
        });
        assert!(graph_listing.entries.iter().any(|entry| entry.name == "_flat"));
        assert!(graph_listing.entries.iter().any(|entry| entry.name == "pages"));

        let flat = provider.list(&AwamUri {
            backend: "knowledge".to_string(),
            path: "logseq-main/_flat".to_string(),
            query: HashMap::new(),
        });
        assert_eq!(flat.entries.len(), 2);
        assert!(flat
            .entries
            .iter()
            .all(|entry| entry.extra.get("virtual") == Some(&serde_json::Value::Bool(true))));
    }
}
