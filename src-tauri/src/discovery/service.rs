// SPDX-License-Identifier: GPL-3.0-or-later

use super::backend::EverythingBackend;
use super::fallback::FallbackIndexStub;
use super::types::{
    DiscoveryEngine, DiscoverySearchOptions, DiscoverySearchResult, DiscoveryStatus,
    DEFAULT_SEARCH_LIMIT, MAX_SEARCH_LIMIT,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct DiscoveryService {
    everything: Arc<dyn EverythingBackend>,
    fallback: FallbackIndexStub,
    fallback_enabled: AtomicBool,
}

impl DiscoveryService {
    pub fn new(everything: Arc<dyn EverythingBackend>, fallback_enabled: bool) -> Self {
        Self {
            everything,
            fallback: FallbackIndexStub,
            fallback_enabled: AtomicBool::new(fallback_enabled),
        }
    }

    pub fn set_fallback_enabled(&self, enabled: bool) {
        self.fallback_enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn fallback_enabled(&self) -> bool {
        self.fallback_enabled.load(Ordering::SeqCst)
    }

    pub fn status(&self) -> DiscoveryStatus {
        let probe = self.everything.probe();
        let fallback_enabled = self.fallback_enabled();
        let (active_engine, degraded, detail) = if probe.available {
            (DiscoveryEngine::Everything, false, probe.detail)
        } else if fallback_enabled {
            (
                DiscoveryEngine::Fallback,
                true,
                format!(
                    "Everything unavailable; fallback index stub active (degraded). {}",
                    probe.detail
                ),
            )
        } else {
            (
                DiscoveryEngine::Unavailable,
                true,
                format!(
                    "Everything unavailable; fallback disabled. {}",
                    probe.detail
                ),
            )
        };

        DiscoveryStatus {
            everything_available: probe.available,
            fallback_enabled,
            active_engine,
            degraded,
            detail,
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

        match status.active_engine {
            DiscoveryEngine::Everything => {
                let result = self
                    .everything
                    .query(&query, limit as u32, offset as u32)?;
                Ok(DiscoverySearchResult {
                    query,
                    engine: DiscoveryEngine::Everything,
                    total_count: result.total_count,
                    offset: result.offset,
                    hits: result.hits,
                })
            }
            DiscoveryEngine::Fallback => {
                let result = self.fallback.query(&query, limit as u32, offset as u32);
                Ok(DiscoverySearchResult {
                    query,
                    engine: DiscoveryEngine::Fallback,
                    total_count: result.total_count,
                    offset: result.offset,
                    hits: result.hits,
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
        ]
    }

    #[test]
    fn everything_is_active_when_available() {
        let backend = Arc::new(MockEverythingBackend::new(true, sample_hits()));
        let service = DiscoveryService::new(backend, true);
        let status = service.status();
        assert!(status.everything_available);
        assert_eq!(status.active_engine, DiscoveryEngine::Everything);
        assert!(!status.degraded);
    }

    #[test]
    fn fallback_only_when_everything_down_and_flag_on() {
        let backend = Arc::new(MockEverythingBackend::new(false, sample_hits()));
        let service = DiscoveryService::new(backend, true);
        assert_eq!(service.status().active_engine, DiscoveryEngine::Fallback);
        let result = service.search("readme".to_string(), None).unwrap();
        assert_eq!(result.engine, DiscoveryEngine::Fallback);
        assert_eq!(result.total_count, 0);
    }

    #[test]
    fn unavailable_when_everything_down_and_fallback_off() {
        let backend = Arc::new(MockEverythingBackend::new(false, sample_hits()));
        let service = DiscoveryService::new(backend, false);
        assert_eq!(service.status().active_engine, DiscoveryEngine::Unavailable);
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
}
