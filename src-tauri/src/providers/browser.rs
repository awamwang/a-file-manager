// SPDX-License-Identifier: GPL-3.0-or-later
//! Shared `browser` provider — Edge + Chrome favorites (Bookmarks JSON only; no history).

use super::provider::Provider;
use super::types::{
    AwamUri, BackendInfo, DirListing, ProviderCapability, ProviderEntry, ResolveResult, StatResult,
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const CAPABILITIES: &[ProviderCapability] = &[
    ProviderCapability::List,
    ProviderCapability::Stat,
    ProviderCapability::Resolve,
];

const ROOT_KEYS: &[(&str, &str)] = &[
    ("bookmark_bar", "Bookmarks bar"),
    ("other", "Other bookmarks"),
    ("synced", "Mobile bookmarks"),
];

#[derive(Debug, Clone)]
struct BrowserProduct {
    id: &'static str,
    display: &'static str,
    user_data: PathBuf,
}

#[derive(Debug, Clone)]
struct ProfileInfo {
    /// Disk folder name under User Data (`Default`, `Profile 1`, …).
    dir_name: String,
    /// Friendly name from Local State when available.
    display_name: String,
    bookmarks_path: PathBuf,
}

pub struct BrowserProvider {
    products: Vec<BrowserProduct>,
}

impl BrowserProvider {
    pub fn system_default() -> Self {
        let local = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Users\Public"));
        Self::with_user_data_roots(
            local.join(r"Microsoft\Edge\User Data"),
            local.join(r"Google\Chrome\User Data"),
        )
    }

    pub fn with_user_data_roots(edge: PathBuf, chrome: PathBuf) -> Self {
        Self {
            products: vec![
                BrowserProduct {
                    id: "edge",
                    display: "Microsoft Edge",
                    user_data: edge,
                },
                BrowserProduct {
                    id: "chrome",
                    display: "Google Chrome",
                    user_data: chrome,
                },
            ],
        }
    }

    fn product(&self, id: &str) -> Option<&BrowserProduct> {
        self.products.iter().find(|p| p.id.eq_ignore_ascii_case(id))
    }

    fn discovered_products(&self) -> Vec<&BrowserProduct> {
        self.products
            .iter()
            .filter(|p| !discover_profiles(&p.user_data).is_empty())
            .collect()
    }

    fn profile_map(product: &BrowserProduct) -> HashMap<String, ProfileInfo> {
        discover_profiles(&product.user_data)
            .into_iter()
            .map(|p| (p.dir_name.clone(), p))
            .collect()
    }

    fn split_path(path: &str) -> Vec<String> {
        path.split('/')
            .filter(|part| !part.is_empty())
            .map(|part| percent_decode(part))
            .collect()
    }

    fn entry_dir(name: &str, uri: String, extra: HashMap<String, Value>) -> ProviderEntry {
        ProviderEntry {
            name: name.to_string(),
            uri,
            is_dir: true,
            is_file: false,
            size: 0,
            modified_time: 0,
            local_path: None,
            extra,
        }
    }

    fn entry_url(name: &str, uri: String, url: &str, extra: HashMap<String, Value>) -> ProviderEntry {
        let mut extra = extra;
        extra.insert("url".to_string(), Value::String(url.to_string()));
        extra.insert("kind".to_string(), Value::String("bookmark".to_string()));
        ProviderEntry {
            name: name.to_string(),
            uri,
            is_dir: false,
            is_file: true,
            size: 0,
            modified_time: 0,
            local_path: None,
            extra,
        }
    }

    fn list_products(&self, uri: &AwamUri) -> DirListing {
        let entries = self
            .discovered_products()
            .into_iter()
            .map(|product| {
                let mut extra = HashMap::new();
                extra.insert(
                    "product".to_string(),
                    Value::String(product.id.to_string()),
                );
                Self::entry_dir(
                    product.display,
                    uri.child(product.id).to_string_uri(),
                    extra,
                )
            })
            .collect();
        DirListing {
            uri: uri.to_string_uri(),
            backend: "browser".to_string(),
            entries,
            error: None,
        }
    }

    fn list_profiles(&self, uri: &AwamUri, product: &BrowserProduct) -> DirListing {
        let mut profiles = discover_profiles(&product.user_data);
        profiles.sort_by(|a, b| {
            a.display_name
                .to_ascii_lowercase()
                .cmp(&b.display_name.to_ascii_lowercase())
        });
        let entries = profiles
            .into_iter()
            .map(|profile| {
                let mut extra = HashMap::new();
                extra.insert(
                    "profileDir".to_string(),
                    Value::String(profile.dir_name.clone()),
                );
                extra.insert(
                    "bookmarksPath".to_string(),
                    Value::String(profile.bookmarks_path.to_string_lossy().to_string()),
                );
                Self::entry_dir(
                    &profile.display_name,
                    uri.child(&percent_encode(&profile.dir_name)).to_string_uri(),
                    extra,
                )
            })
            .collect();
        DirListing {
            uri: uri.to_string_uri(),
            backend: "browser".to_string(),
            entries,
            error: None,
        }
    }

    fn list_profile_roots(
        &self,
        uri: &AwamUri,
        product: &BrowserProduct,
        profile: &ProfileInfo,
    ) -> DirListing {
        let doc = match read_bookmarks_json(&profile.bookmarks_path) {
            Ok(doc) => doc,
            Err(err) => {
                return DirListing {
                    uri: uri.to_string_uri(),
                    backend: "browser".to_string(),
                    entries: Vec::new(),
                    error: Some(err),
                };
            }
        };
        let roots = doc
            .get("roots")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut entries = Vec::new();
        for (key, fallback_name) in ROOT_KEYS {
            let Some(node) = roots.get(*key) else {
                continue;
            };
            if !node_has_visible_children(node) {
                continue;
            }
            let name = node
                .get("name")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .unwrap_or(fallback_name);
            let mut extra = HashMap::new();
            extra.insert("rootKey".to_string(), Value::String((*key).to_string()));
            extra.insert(
                "product".to_string(),
                Value::String(product.id.to_string()),
            );
            entries.push(Self::entry_dir(
                name,
                uri.child(key).to_string_uri(),
                extra,
            ));
        }
        DirListing {
            uri: uri.to_string_uri(),
            backend: "browser".to_string(),
            entries,
            error: None,
        }
    }

    fn list_folder(
        &self,
        uri: &AwamUri,
        product: &BrowserProduct,
        profile: &ProfileInfo,
        node_path: &[String],
    ) -> DirListing {
        let doc = match read_bookmarks_json(&profile.bookmarks_path) {
            Ok(doc) => doc,
            Err(err) => {
                return DirListing {
                    uri: uri.to_string_uri(),
                    backend: "browser".to_string(),
                    entries: Vec::new(),
                    error: Some(err),
                };
            }
        };
        let Some(node) = find_node(&doc, node_path) else {
            return DirListing {
                uri: uri.to_string_uri(),
                backend: "browser".to_string(),
                entries: Vec::new(),
                error: Some(format!("Bookmark folder not found: {}", uri.to_string_uri())),
            };
        };
        let children = node
            .get("children")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut entries = Vec::new();
        for child in children {
            let typ = child.get("type").and_then(Value::as_str).unwrap_or("");
            let name = child
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("(untitled)");
            let mut extra = HashMap::new();
            extra.insert(
                "product".to_string(),
                Value::String(product.id.to_string()),
            );
            extra.insert(
                "profileDir".to_string(),
                Value::String(profile.dir_name.clone()),
            );
            match typ {
                "folder" => {
                    let id = child
                        .get("id")
                        .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| {
                            v.as_u64().map(|n| n.to_string())
                        }))
                        .unwrap_or_else(|| name.to_string());
                    entries.push(Self::entry_dir(
                        name,
                        uri.child(&percent_encode(&id)).to_string_uri(),
                        extra,
                    ));
                }
                "url" => {
                    let url = child.get("url").and_then(Value::as_str).unwrap_or("");
                    if url.is_empty() {
                        continue;
                    }
                    let id = child
                        .get("id")
                        .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| {
                            v.as_u64().map(|n| n.to_string())
                        }))
                        .unwrap_or_else(|| name.to_string());
                    entries.push(Self::entry_url(
                        name,
                        uri.child(&percent_encode(&id)).to_string_uri(),
                        url,
                        extra,
                    ));
                }
                _ => {}
            }
        }
        DirListing {
            uri: uri.to_string_uri(),
            backend: "browser".to_string(),
            entries,
            error: None,
        }
    }

    fn lookup_url(
        &self,
        profile: &ProfileInfo,
        node_path: &[String],
    ) -> Option<(String, String)> {
        let doc = read_bookmarks_json(&profile.bookmarks_path).ok()?;
        let node = find_node(&doc, node_path)?;
        if node.get("type").and_then(Value::as_str) != Some("url") {
            return None;
        }
        let name = node
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("(untitled)")
            .to_string();
        let url = node.get("url").and_then(Value::as_str)?.to_string();
        Some((name, url))
    }
}

impl Provider for BrowserProvider {
    fn backend_id(&self) -> &str {
        "browser"
    }
    fn name(&self) -> &str {
        "Browser"
    }
    fn description(&self) -> &str {
        "Edge and Chrome favorites (bookmarks only; no history)"
    }
    fn capabilities(&self) -> &[ProviderCapability] {
        CAPABILITIES
    }

    fn health(&self) -> BackendInfo {
        let products = self.discovered_products();
        let profile_count: usize = products
            .iter()
            .map(|p| discover_profiles(&p.user_data).len())
            .sum();
        let healthy = profile_count > 0;
        let detail = if healthy {
            let names: Vec<&str> = products.iter().map(|p| p.display).collect();
            Some(format!(
                "{profile_count} profile(s) · {}",
                names.join(" + ")
            ))
        } else {
            Some("No Edge/Chrome Bookmarks files found under LocalAppData".to_string())
        };
        BackendInfo {
            id: "browser".to_string(),
            name: "Browser".to_string(),
            description: self.description().to_string(),
            healthy,
            capabilities: CAPABILITIES
                .iter()
                .map(|c| c.as_str().to_string())
                .collect(),
            detail,
            interface_only: false,
            source: Some("builtin".to_string()),
        }
    }

    fn list(&self, uri: &AwamUri) -> DirListing {
        let parts = Self::split_path(&uri.path);
        match parts.as_slice() {
            [] => self.list_products(uri),
            [product_id] => {
                let Some(product) = self.product(product_id) else {
                    return DirListing {
                        uri: uri.to_string_uri(),
                        backend: "browser".to_string(),
                        entries: Vec::new(),
                        error: Some(format!("Unknown browser product: {product_id}")),
                    };
                };
                self.list_profiles(uri, product)
            }
            [product_id, profile_dir] => {
                let Some(product) = self.product(product_id) else {
                    return DirListing {
                        uri: uri.to_string_uri(),
                        backend: "browser".to_string(),
                        entries: Vec::new(),
                        error: Some(format!("Unknown browser product: {product_id}")),
                    };
                };
                let profiles = Self::profile_map(product);
                let Some(profile) = profiles.get(profile_dir) else {
                    return DirListing {
                        uri: uri.to_string_uri(),
                        backend: "browser".to_string(),
                        entries: Vec::new(),
                        error: Some(format!("Unknown profile: {profile_dir}")),
                    };
                };
                self.list_profile_roots(uri, product, profile)
            }
            [product_id, profile_dir, rest @ ..] => {
                let Some(product) = self.product(product_id) else {
                    return DirListing {
                        uri: uri.to_string_uri(),
                        backend: "browser".to_string(),
                        entries: Vec::new(),
                        error: Some(format!("Unknown browser product: {product_id}")),
                    };
                };
                let profiles = Self::profile_map(product);
                let Some(profile) = profiles.get(profile_dir) else {
                    return DirListing {
                        uri: uri.to_string_uri(),
                        backend: "browser".to_string(),
                        entries: Vec::new(),
                        error: Some(format!("Unknown profile: {profile_dir}")),
                    };
                };
                self.list_folder(uri, product, profile, rest)
            }
        }
    }

    fn stat(&self, uri: &AwamUri) -> StatResult {
        let listing = self.list(uri);
        if let Some(error) = listing.error.clone() {
            // Still allow stating the virtual folder itself when children fail.
            if uri.path.is_empty() || Self::split_path(&uri.path).len() <= 2 {
                return StatResult {
                    entry: Some(ProviderEntry {
                        name: self.backend_id().to_string(),
                        uri: uri.to_string_uri(),
                        is_dir: true,
                        is_file: false,
                        size: 0,
                        modified_time: 0,
                        local_path: None,
                        extra: HashMap::new(),
                    }),
                    error: Some(error),
                };
            }
        }
        let parts = Self::split_path(&uri.path);
        if parts.len() >= 3 {
            if let (Some(product), Some(profile_dir), rest) =
                (parts.first(), parts.get(1), &parts[2..])
            {
                if let Some(product) = self.product(product) {
                    if let Some(profile) = Self::profile_map(product).get(profile_dir) {
                        if let Some((name, url)) = self.lookup_url(profile, rest) {
                            return StatResult {
                                entry: Some(Self::entry_url(
                                    &name,
                                    uri.to_string_uri(),
                                    &url,
                                    HashMap::new(),
                                )),
                                error: None,
                            };
                        }
                    }
                }
            }
        }
        if listing.error.is_none() {
            return StatResult {
                entry: Some(ProviderEntry {
                    name: parts
                        .last()
                        .cloned()
                        .unwrap_or_else(|| self.backend_id().to_string()),
                    uri: uri.to_string_uri(),
                    is_dir: true,
                    is_file: false,
                    size: 0,
                    modified_time: 0,
                    local_path: None,
                    extra: HashMap::new(),
                }),
                error: None,
            };
        }
        StatResult {
            entry: None,
            error: listing.error,
        }
    }

    fn resolve(&self, uri: &AwamUri) -> ResolveResult {
        let parts = Self::split_path(&uri.path);
        if parts.len() < 3 {
            return ResolveResult {
                uri: uri.to_string_uri(),
                local_path: None,
                action: None,
                action_args: HashMap::new(),
                error: Some("Not a bookmark URL node".to_string()),
            };
        }
        let product_id = &parts[0];
        let profile_dir = &parts[1];
        let rest = &parts[2..];
        let Some(product) = self.product(product_id) else {
            return ResolveResult {
                uri: uri.to_string_uri(),
                local_path: None,
                action: None,
                action_args: HashMap::new(),
                error: Some(format!("Unknown browser product: {product_id}")),
            };
        };
        let profiles = Self::profile_map(product);
        let Some(profile) = profiles.get(profile_dir) else {
            return ResolveResult {
                uri: uri.to_string_uri(),
                local_path: None,
                action: None,
                action_args: HashMap::new(),
                error: Some(format!("Unknown profile: {profile_dir}")),
            };
        };
        match self.lookup_url(profile, rest) {
            Some((_name, url)) => {
                let mut action_args = HashMap::new();
                action_args.insert("url".to_string(), Value::String(url));
                ResolveResult {
                    uri: uri.to_string_uri(),
                    local_path: None,
                    action: Some("open_url".to_string()),
                    action_args,
                    error: None,
                }
            }
            None => ResolveResult {
                uri: uri.to_string_uri(),
                local_path: None,
                action: None,
                action_args: HashMap::new(),
                error: Some("Bookmark URL not found".to_string()),
            },
        }
    }
}

fn discover_profiles(user_data: &Path) -> Vec<ProfileInfo> {
    if !user_data.is_dir() {
        return Vec::new();
    }
    let names = read_profile_display_names(user_data);
    let Ok(read_dir) = fs::read_dir(user_data) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in read_dir.flatten() {
        let path = item.path();
        if !path.is_dir() {
            continue;
        }
        let dir_name = item.file_name().to_string_lossy().to_string();
        if !is_profile_dir_name(&dir_name) {
            continue;
        }
        let bookmarks_path = path.join("Bookmarks");
        // Prefer profiles that currently expose Bookmarks; skip missing quietly.
        if !bookmarks_path.is_file() {
            continue;
        }
        // Probe readability — locked/missing should not abort the whole provider.
        if let Err(_err) = fs::OpenOptions::new().read(true).open(&bookmarks_path) {
            continue;
        }
        let display_name = names
            .get(&dir_name)
            .cloned()
            .unwrap_or_else(|| dir_name.clone());
        out.push(ProfileInfo {
            dir_name,
            display_name,
            bookmarks_path,
        });
    }
    out
}

fn is_profile_dir_name(name: &str) -> bool {
    if name.eq_ignore_ascii_case("Default") {
        return true;
    }
    if let Some(rest) = name.strip_prefix("Profile ") {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit());
    }
    false
}

fn read_profile_display_names(user_data: &Path) -> HashMap<String, String> {
    let local_state = user_data.join("Local State");
    let Ok(raw) = fs::read_to_string(&local_state) else {
        return HashMap::new();
    };
    let Ok(doc) = serde_json::from_str::<Value>(&raw) else {
        return HashMap::new();
    };
    let mut map = HashMap::new();
    if let Some(cache) = doc
        .pointer("/profile/info_cache")
        .and_then(Value::as_object)
    {
        for (dir, info) in cache {
            if let Some(name) = info.get("name").and_then(Value::as_str) {
                if !name.is_empty() {
                    map.insert(dir.clone(), name.to_string());
                }
            }
        }
    }
    map
}

fn read_bookmarks_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path).map_err(|err| {
        format!(
            "Could not read bookmarks ({}): {err}",
            path.to_string_lossy()
        )
    })?;
    serde_json::from_str(&raw).map_err(|err| {
        format!(
            "Could not parse bookmarks ({}): {err}",
            path.to_string_lossy()
        )
    })
}

fn node_has_visible_children(node: &Value) -> bool {
    node.get("children")
        .and_then(Value::as_array)
        .is_some_and(|children| {
            children.iter().any(|child| {
                matches!(
                    child.get("type").and_then(Value::as_str),
                    Some("url" | "folder")
                )
            })
        })
}

fn find_node<'a>(doc: &'a Value, node_path: &[String]) -> Option<&'a Value> {
    if node_path.is_empty() {
        return None;
    }
    let roots = doc.get("roots")?;
    let mut current = roots.get(node_path[0].as_str())?;
    for id in &node_path[1..] {
        let children = current.get("children")?.as_array()?;
        current = children.iter().find(|child| node_id_matches(child, id))?;
    }
    Some(current)
}

fn node_id_matches(node: &Value, needle: &str) -> bool {
    if let Some(id) = node.get("id") {
        if id.as_str() == Some(needle) {
            return true;
        }
        if id.as_u64().map(|n| n.to_string()) == Some(needle.to_string()) {
            return true;
        }
    }
    false
}

fn percent_encode(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h = from_hex(bytes[i + 1]);
            let l = from_hex(bytes[i + 2]);
            if let (Some(h), Some(l)) = (h, l) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_fixture(root: &Path) {
        let edge = root.join("edge").join("User Data");
        let chrome = root.join("chrome").join("User Data");
        fs::create_dir_all(edge.join("Default")).unwrap();
        fs::create_dir_all(chrome.join("Profile 1")).unwrap();
        fs::write(
            edge.join("Local State"),
            r#"{"profile":{"info_cache":{"Default":{"name":"Edge Work"}}}}"#,
        )
        .unwrap();
        fs::write(
            chrome.join("Local State"),
            r#"{"profile":{"info_cache":{"Profile 1":{"name":"Chrome Personal"}}}}"#,
        )
        .unwrap();
        let bookmarks = r#"{
          "roots": {
            "bookmark_bar": {
              "children": [
                {
                  "id": "10",
                  "name": "Example",
                  "type": "url",
                  "url": "https://example.com/"
                },
                {
                  "id": "11",
                  "name": "Dev",
                  "type": "folder",
                  "children": [
                    {
                      "id": "12",
                      "name": "Docs",
                      "type": "url",
                      "url": "https://docs.example.com/"
                    }
                  ]
                }
              ],
              "name": "Bookmarks bar",
              "type": "folder"
            },
            "other": { "children": [], "name": "Other bookmarks", "type": "folder" },
            "synced": { "children": [], "name": "Mobile bookmarks", "type": "folder" }
          }
        }"#;
        fs::write(edge.join("Default").join("Bookmarks"), bookmarks).unwrap();
        fs::write(chrome.join("Profile 1").join("Bookmarks"), bookmarks).unwrap();
        // Unreadable / missing profile should be ignored.
        fs::create_dir_all(edge.join("Profile 9")).unwrap();
    }

    #[test]
    fn merges_edge_and_chrome_under_one_backend() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = std::env::temp_dir().join(format!("afm-browser-{stamp}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        write_fixture(&root);

        let provider = BrowserProvider::with_user_data_roots(
            root.join("edge").join("User Data"),
            root.join("chrome").join("User Data"),
        );
        assert!(provider.health().healthy);

        let root_list = provider.list(&AwamUri {
            backend: "browser".into(),
            path: String::new(),
            query: HashMap::new(),
        });
        assert!(root_list.error.is_none());
        assert_eq!(root_list.entries.len(), 2);
        assert!(root_list.entries.iter().any(|e| e.uri == "awam://browser/edge"));
        assert!(root_list
            .entries
            .iter()
            .any(|e| e.uri == "awam://browser/chrome"));

        let edge_profiles = provider.list(&AwamUri {
            backend: "browser".into(),
            path: "edge".into(),
            query: HashMap::new(),
        });
        assert_eq!(edge_profiles.entries.len(), 1);
        assert_eq!(edge_profiles.entries[0].name, "Edge Work");

        let bar = provider.list(&AwamUri {
            backend: "browser".into(),
            path: "edge/Default/bookmark_bar".into(),
            query: HashMap::new(),
        });
        assert!(bar.entries.iter().any(|e| e.name == "Example" && !e.is_dir));
        assert!(bar.entries.iter().any(|e| e.name == "Dev" && e.is_dir));

        let resolved = provider.resolve(&AwamUri {
            backend: "browser".into(),
            path: "edge/Default/bookmark_bar/10".into(),
            query: HashMap::new(),
        });
        assert_eq!(resolved.action.as_deref(), Some("open_url"));
        assert_eq!(
            resolved.action_args.get("url"),
            Some(&Value::String("https://example.com/".into()))
        );

        let nested = provider.resolve(&AwamUri {
            backend: "browser".into(),
            path: "chrome/Profile%201/bookmark_bar/11/12".into(),
            query: HashMap::new(),
        });
        assert_eq!(nested.action.as_deref(), Some("open_url"));
        assert_eq!(
            nested.action_args.get("url"),
            Some(&Value::String("https://docs.example.com/".into()))
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_user_data_is_healthy_false_not_fatal() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let missing = std::env::temp_dir().join(format!("afm-browser-missing-{stamp}"));
        let provider = BrowserProvider::with_user_data_roots(
            missing.join("edge"),
            missing.join("chrome"),
        );
        assert!(!provider.health().healthy);
        let listing = provider.list(&AwamUri {
            backend: "browser".into(),
            path: String::new(),
            query: HashMap::new(),
        });
        assert!(listing.entries.is_empty());
        assert!(listing.error.is_none());
    }
}
