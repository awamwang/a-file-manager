// SPDX-License-Identifier: GPL-3.0-or-later
//! Depth-1 provider listing cache (AIA-76).
//!
//! Caches only:
//! - `awam://root` (provider roster)
//! - `awam://{backend}` with empty path (first-level listing under each provider)
//!
//! Deeper URIs always bypass the cache. TTL differs for ok vs error listings.

use super::types::{AwamUri, DirListing};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// 成功的 depth-1 列表缓存时长；手动刷新或改网关地址时会主动失效。
pub const TTL_OK: Duration = Duration::from_secs(600);
/// Failed / error listings expire sooner so remotes can recover.
pub const TTL_ERR: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
struct CacheEntry {
    listing: DirListing,
    fetched_at: Instant,
    ttl: Duration,
}

#[derive(Debug, Default)]
pub struct ListingCache {
    entries: HashMap<String, CacheEntry>,
}

impl ListingCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&mut self, key: &str, now: Instant) -> Option<DirListing> {
        let expired = match self.entries.get(key) {
            Some(entry) => now.duration_since(entry.fetched_at) >= entry.ttl,
            None => return None,
        };
        if expired {
            self.entries.remove(key);
            return None;
        }
        self.entries.get(key).map(|entry| entry.listing.clone())
    }

    pub fn put(&mut self, key: String, listing: DirListing, now: Instant) {
        let ttl = if listing.error.is_some() {
            TTL_ERR
        } else {
            TTL_OK
        };
        self.entries.insert(
            key,
            CacheEntry {
                listing,
                fetched_at: now,
                ttl,
            },
        );
    }

    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }

    pub fn invalidate_key(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }
}

/// Depth-1 = empty path under a backend (including `root`).
pub fn is_depth1(uri: &AwamUri) -> bool {
    uri.path.is_empty() || uri.path == "/"
}

/// Stable cache key for a depth-1 URI, or `None` when deeper than depth 1.
pub fn depth1_cache_key(uri: &AwamUri) -> Option<String> {
    if !is_depth1(uri) {
        return None;
    }
    Some(format!("awam://{}", uri.backend.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::uri::parse_awam_uri;

    fn listing(uri: &str, err: Option<&str>) -> DirListing {
        DirListing {
            uri: uri.to_string(),
            backend: "demo".to_string(),
            entries: Vec::new(),
            error: err.map(str::to_string),
        }
    }

    #[test]
    fn depth1_keys_only() {
        let root = parse_awam_uri("awam://root").unwrap();
        assert_eq!(depth1_cache_key(&root).as_deref(), Some("awam://root"));

        let rclone = parse_awam_uri("awam://rclone").unwrap();
        assert_eq!(depth1_cache_key(&rclone).as_deref(), Some("awam://rclone"));

        let deep = parse_awam_uri("awam://rclone/remote/dir").unwrap();
        assert_eq!(depth1_cache_key(&deep), None);

        let knowledge_graph = parse_awam_uri("awam://knowledge/logseq-main").unwrap();
        assert_eq!(depth1_cache_key(&knowledge_graph), None);
    }

    #[test]
    fn hit_miss_and_ttl() {
        let mut cache = ListingCache::new();
        let now = Instant::now();
        cache.put("awam://root".into(), listing("awam://root", None), now);
        assert!(cache.get("awam://root", now).is_some());
        assert!(cache.get("awam://root", now + TTL_OK).is_none());
        assert!(!cache.contains_key("awam://root"));
    }

    #[test]
    fn error_ttl_shorter() {
        let mut cache = ListingCache::new();
        let now = Instant::now();
        cache.put(
            "awam://rclone".into(),
            listing("awam://rclone", Some("down")),
            now,
        );
        assert!(cache.get("awam://rclone", now + TTL_ERR - Duration::from_millis(1)).is_some());
        assert!(cache.get("awam://rclone", now + TTL_ERR).is_none());
    }

    #[test]
    fn invalidate() {
        let mut cache = ListingCache::new();
        let now = Instant::now();
        cache.put("awam://root".into(), listing("awam://root", None), now);
        cache.put("awam://rclone".into(), listing("awam://rclone", None), now);
        assert!(cache.invalidate_key("awam://rclone"));
        assert_eq!(cache.len(), 1);
        cache.invalidate_all();
        assert_eq!(cache.len(), 0);
    }
}
