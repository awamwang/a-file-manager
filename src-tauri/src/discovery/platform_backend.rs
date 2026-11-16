// SPDX-License-Identifier: GPL-3.0-or-later
//! Thin bridge from the platform adapter's neutral search contract to the
//! Discovery service's `EverythingBackend` (7.21).
//!
//! No OS details live here: the macOS adapter owns `mdfind`, this file only
//! maps neutral types (`SearchHit` → `DiscoveryHit`) and declares the engine
//! label. Windows never constructs this backend — it keeps Everything IPC.

use super::backend::EverythingBackend;
use super::types::{DiscoveryEngine, DiscoveryHit, EverythingProbe, EverythingQueryResult};
use crate::platform;

pub struct PlatformSearchBackend;

impl EverythingBackend for PlatformSearchBackend {
    fn engine(&self) -> DiscoveryEngine {
        DiscoveryEngine::Spotlight
    }

    fn probe(&self) -> EverythingProbe {
        let probe = platform::probe_global_search();
        EverythingProbe {
            available: probe.available,
            detail: probe.detail,
        }
    }

    fn query(
        &self,
        search: &str,
        max_results: u32,
        offset: u32,
    ) -> Result<EverythingQueryResult, String> {
        let result = platform::global_search(search, max_results, offset)?;
        Ok(EverythingQueryResult {
            total_count: result.total_count,
            offset: result.offset,
            hits: result
                .hits
                .into_iter()
                .map(|hit| DiscoveryHit {
                    name: hit.name,
                    path: hit.path,
                    is_dir: hit.is_dir,
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_the_spotlight_engine() {
        assert_eq!(PlatformSearchBackend.engine(), DiscoveryEngine::Spotlight);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn probe_delegates_to_the_real_host_index() {
        assert!(PlatformSearchBackend.probe().available);
    }
}
