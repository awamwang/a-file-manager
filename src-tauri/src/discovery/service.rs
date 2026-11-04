// SPDX-License-Identifier: GPL-3.0-or-later

use super::backend::EverythingBackend;
use super::blacklist::{filter_hits, sanitize_blacklist};
use super::fallback::FallbackIndexStub;
use super::types::{
    DiscoveryEngine, DiscoveryIndexState, DiscoverySearchOptions, DiscoverySearchResult,
    DiscoveryStatus, DEFAULT_SEARCH_LIMIT, MAX_SEARCH_LIMIT,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct DiscoveryService {
    everything: Arc<dyn EverythingBackend>,
    fallback: FallbackIndexStub,
    fallback_enabled: AtomicBool,
    blacklist: Mutex<Vec<String>>,
}

impl DiscoveryService {
    pub fn new(everything: Arc<dyn EverythingBackend>, fallback_enabled: bool) -> Self {
        Self {
            everything,
            fallback: FallbackIndexStub,
            fallback_enabled: AtomicBool::new(fallback_enabled),
            blacklist: Mutex::new(Vec::new()),
        }
    }

    pub fn set_fallback_enabled(&self, enabled: bool) {
        self.fallback_enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn fallback_enabled(&self) -> bool {
        self.fallback_enabled.load(Ordering::SeqCst)
    }

    pub fn blacklist(&self) -> Vec<String> {
        self.blacklist
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn set_blacklist(&self, entries: Vec<String>) -> Vec<String> {
        let cleaned = sanitize_blacklist(&entries);
        if let Ok(mut guard) = self.blacklist.lock() {
            *guard = cleaned.clone();
        }
        cleaned
    }

    fn estimated_item_count(&self, everything_available: bool) -> Option<u64> {
        if !everything_available {
            return None;
        }
        // Lightweight probe: empty query still returns Everything's total item count.
        match self.everything.query("", 1, 0) {
            Ok(result) => Some(result.total_count),
            Err(_) => None,
        }
    }

    pub fn status(&self) -> DiscoveryStatus {
        let probe = self.everything.probe();
        let fallback_enabled = self.fallback_enabled();
        let blacklist = self.blacklist();
        let (active_engine, degraded, detail, index_state) = if probe.available {
            (
                DiscoveryEngine::Everything,
                false,
                probe.detail,
                DiscoveryIndexState::Ready,
            )
        } else if fallback_enabled {
            (
                DiscoveryEngine::Fallback,
                true,
                format!(
                    "Everything unavailable; fallback index stub active (degraded). {}",
                    probe.detail
                ),
                DiscoveryIndexState::Degraded,
            )
        } else {
            (
                DiscoveryEngine::Unavailable,
                true,
                format!(
                    "Everything unavailable; fallback disabled. {}",
                    probe.detail
                ),
                DiscoveryIndexState::Unavailable,
            )
        };

        let estimated_item_count = self.estimated_item_count(probe.available);

        DiscoveryStatus {
            everything_available: probe.available,
            fallback_enabled,
            active_engine,
            degraded,
            detail,
            index_state,
            estimated_item_count,
            blacklist,
        }
    }

    pub fn search(
        &self,
        query: String,
        options: Option<DiscoverySearchOptions>,
    ) -> Result<DiscoverySearchResult, String> {
        let options = options.unwrap_or_default();
        let limit = options
            .limit
            .unwrap_or(DEFAULT_SEARCH_LIMIT)
            .clamp(1, MAX_SEARCH_LIMIT);
        let offset = options.offset;
        let status = self.status();
        let blacklist = status.blacklist.clone();

        match status.active_engine {
            DiscoveryEngine::Everything => {
                // Over-fetch a bit so blacklist filtering still fills the page.
                let fetch_limit = limit.saturating_mul(3).clamp(limit, MAX_SEARCH_LIMIT) as u32;
                let result = self
                    .everything
                    .query(&query, fetch_limit, offset as u32)?;
                let filtered = filter_hits(result.hits, &blacklist);
                let hits: Vec<_> = filtered.into_iter().take(limit as usize).collect();
                Ok(DiscoverySearchResult {
                    query,
                    engine: DiscoveryEngine::Everything,
                    total_count: result.total_count,
                    offset: result.offset,
                    hits,
                })
            }
            DiscoveryEngine::Fallback => {
                let result = self.fallback.query(&query, limit as u32, offset as u32);
                let hits = filter_hits(result.hits, &blacklist);
                Ok(DiscoverySearchResult {
                    query,
                    engine: DiscoveryEngine::Fallback,
                    total_count: result.total_count,
                    offset: result.offset,
                    hits,
                })
            }
            DiscoveryEngine::Unavailable => Err(
                "Discovery unavailable: Everything IPC not running and fallback is disabled"
                    .to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::mock::MockEverythingBackend;
    use crate::discovery::types::DiscoveryHit;

    fn sample_hits() -> Vec<DiscoveryHit> {
        vec![
            DiscoveryHit {
                name: "readme.md".to_string(),
                path: "G:/Projects/readme.md".to_string(),
                is_dir: false,
            },
            DiscoveryHit {
                name: "ReadmeFolder".to_string(),
                path: "G:/Projects/ReadmeFolder".to_string(),
                is_dir: true,
            },
            DiscoveryHit {
                name: "other.txt".to_string(),
                path: "G:/other.txt".to_string(),
                is_dir: false,
            },
            DiscoveryHit {
                name: "secret.txt".to_string(),
                path: "C:/Windows/secret.txt".to_string(),
                is_dir: false,
            },
        ]
    }

    #[test]
    fn everything_is_active_when_available() {
        let backend = Arc::new(MockEverythingBackend::new(true, sample_hits()));
        let service = DiscoveryService::new(backend, true);
        let status = service.status();
        assert!(status.everything_available);
        assert_eq!(status.active_engine, DiscoveryEngine::Everything);
        assert_eq!(status.index_state, DiscoveryIndexState::Ready);
        assert!(!status.degraded);
        assert!(status.estimated_item_count.is_some());
    }

    #[test]
    fn fallback_only_when_everything_down_and_flag_on() {
        let backend = Arc::new(MockEverythingBackend::new(false, sample_hits()));
        let service = DiscoveryService::new(backend, true);
        assert_eq!(service.status().active_engine, DiscoveryEngine::Fallback);
        assert_eq!(service.status().index_state, DiscoveryIndexState::Degraded);
        let result = service.search("readme".to_string(), None).unwrap();
        assert_eq!(result.engine, DiscoveryEngine::Fallback);
        assert_eq!(result.total_count, 0);
    }

    #[test]
    fn unavailable_when_everything_down_and_fallback_off() {
        let backend = Arc::new(MockEverythingBackend::new(false, sample_hits()));
        let service = DiscoveryService::new(backend, false);
        assert_eq!(service.status().active_engine, DiscoveryEngine::Unavailable);
        assert_eq!(
            service.status().index_state,
            DiscoveryIndexState::Unavailable
        );
        assert!(service.search("x".to_string(), None).is_err());
    }

    #[test]
    fn mock_search_filters_and_pages() {
        let backend = Arc::new(MockEverythingBackend::new(true, sample_hits()));
        let service = DiscoveryService::new(backend, false);
        let result = service
            .search(
                "readme".to_string(),
                Some(DiscoverySearchOptions {
                    offset: 0,
                    limit: Some(1),
                }),
            )
            .unwrap();
        assert_eq!(result.engine, DiscoveryEngine::Everything);
        assert_eq!(result.total_count, 2);
        assert_eq!(result.hits.len(), 1);
    }

    #[test]
    fn never_merges_everything_and_fallback() {
        let backend = Arc::new(MockEverythingBackend::new(true, sample_hits()));
        let service = DiscoveryService::new(backend, true);
        let result = service.search("readme".to_string(), None).unwrap();
        assert_eq!(result.engine, DiscoveryEngine::Everything);
        assert_eq!(result.total_count, 2);
    }

    #[test]
    fn blacklist_filters_search_hits() {
        let backend = Arc::new(MockEverythingBackend::new(true, sample_hits()));
        let service = DiscoveryService::new(backend, false);
        service.set_blacklist(vec![r"C:\Windows".into()]);
        let result = service.search("secret".to_string(), None).unwrap();
        assert!(result.hits.is_empty());
        assert_eq!(service.status().blacklist, vec![r"C:\Windows".to_string()]);
    }
}
