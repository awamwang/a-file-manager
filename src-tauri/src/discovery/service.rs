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

    /// Empty Everything query for totitems — can take up to QUERY_TIMEOUT (~5s).
    /// Kept off the light `status()` path so Discovery first paint stays snappy (AIA-182).
    /// Spotlight has no cheap "total indexed items" equivalent (`mdfind -name ""`
    /// fails to build a query), so it deliberately reports unknown (7.21).
    pub fn estimate_item_count(&self) -> Option<u64> {
        if !self.everything.probe().available
            || self.everything.engine() == DiscoveryEngine::Spotlight
        {
            return None;
        }
        match self.everything.query("", 1, 0) {
            Ok(result) => Some(result.total_count),
            Err(_) => None,
        }
    }

    /// Light status: FindWindow probe + engine flags only (no empty-query estimate).
    pub fn status(&self) -> DiscoveryStatus {
        let probe = self.everything.probe();
        let engine = self.everything.engine();
        let engine_label = engine_label(engine);
        let fallback_enabled = self.fallback_enabled();
        let blacklist = self.blacklist();
        let (active_engine, degraded, detail, index_state) = if probe.available {
            (engine, false, probe.detail, DiscoveryIndexState::Ready)
        } else if fallback_enabled {
            (
                DiscoveryEngine::Fallback,
                true,
                format!(
                    "{engine_label} unavailable; fallback index stub active (degraded). {}",
                    probe.detail
                ),
                DiscoveryIndexState::Degraded,
            )
        } else {
            (
                DiscoveryEngine::Unavailable,
                true,
                format!(
                    "{engine_label} unavailable; fallback disabled. {}",
                    probe.detail
                ),
                DiscoveryIndexState::Unavailable,
            )
        };

        DiscoveryStatus {
            everything_available: probe.available,
            fallback_enabled,
            active_engine,
            degraded,
            detail,
            index_state,
            estimated_item_count: None,
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
            DiscoveryEngine::Everything | DiscoveryEngine::Spotlight => {
                // Over-fetch a bit so blacklist filtering still fills the page.
                let fetch_limit = limit.saturating_mul(3).clamp(limit, MAX_SEARCH_LIMIT) as u32;
                let result = self.everything.query(&query, fetch_limit, offset as u32)?;
                let filtered = filter_hits(result.hits, &blacklist);
                let hits: Vec<_> = filtered.into_iter().take(limit as usize).collect();
                Ok(DiscoverySearchResult {
                    query,
                    engine: self.everything.engine(),
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
            DiscoveryEngine::Unavailable => Err(format!(
                "Discovery unavailable: {} is not running and fallback is disabled",
                engine_label(self.everything.engine())
            )),
        }
    }
}

/// Neutral label for status/error text; the UI uses the i18n engine name
/// instead of showing raw `detail` (AIA-61).
fn engine_label(engine: DiscoveryEngine) -> &'static str {
    match engine {
        DiscoveryEngine::Everything => "Everything",
        DiscoveryEngine::Spotlight => "Spotlight",
        DiscoveryEngine::Fallback => "Fallback index",
        DiscoveryEngine::Unavailable => "Host index",
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
        assert!(status.estimated_item_count.is_none());
        assert!(service.estimate_item_count().is_some());
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
        // 7.21: blacklist paths are host-native now, so the fixture follows
        // the platform instead of assuming a Windows drive letter.
        let (blocked, hit_path) = if cfg!(windows) {
            (r"C:\Windows", r"C:\Windows\secret.txt")
        } else {
            ("/Users/me/Library", "/Users/me/Library/secret.txt")
        };
        let backend = Arc::new(MockEverythingBackend::new(
            true,
            vec![DiscoveryHit {
                name: "secret.txt".into(),
                path: hit_path.into(),
                is_dir: false,
            }],
        ));
        let service = DiscoveryService::new(backend, false);
        service.set_blacklist(vec![blocked.into()]);
        let result = service.search("secret".to_string(), None).unwrap();
        assert!(result.hits.is_empty());
        assert_eq!(service.status().blacklist, vec![blocked.to_string()]);
    }

    #[test]
    fn spotlight_engine_serializes_for_the_frontend() {
        assert_eq!(
            serde_json::to_string(&DiscoveryEngine::Spotlight).unwrap(),
            "\"spotlight\""
        );
    }

    #[test]
    fn spotlight_engine_is_reported_in_status_and_results() {
        struct SpotlightStub;
        impl EverythingBackend for SpotlightStub {
            fn engine(&self) -> DiscoveryEngine {
                DiscoveryEngine::Spotlight
            }
            fn probe(&self) -> crate::discovery::types::EverythingProbe {
                crate::discovery::types::EverythingProbe {
                    available: true,
                    detail: "Spotlight index available".into(),
                }
            }
            fn query(
                &self,
                _search: &str,
                _max_results: u32,
                _offset: u32,
            ) -> Result<crate::discovery::types::EverythingQueryResult, String> {
                Ok(crate::discovery::types::EverythingQueryResult {
                    total_count: 1,
                    offset: 0,
                    hits: vec![DiscoveryHit {
                        name: "readme.md".into(),
                        path: "/Users/me/readme.md".into(),
                        is_dir: false,
                    }],
                })
            }
        }
        let service = DiscoveryService::new(Arc::new(SpotlightStub), false);
        assert_eq!(service.status().active_engine, DiscoveryEngine::Spotlight);
        // Spotlight has no cheap "total indexed items" query.
        assert!(service.estimate_item_count().is_none());
        let result = service.search("readme".to_string(), None).unwrap();
        assert_eq!(result.engine, DiscoveryEngine::Spotlight);
        assert_eq!(result.hits[0].path, "/Users/me/readme.md");
    }
}
