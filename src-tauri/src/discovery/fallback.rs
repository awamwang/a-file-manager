// SPDX-License-Identifier: GPL-3.0-or-later

use super::types::{DiscoveryHit, EverythingQueryResult};

/// Self-built index stub — only when Everything is down and fallback flag is on.
/// Not a dual-default engine (ADR 0002).
#[derive(Debug, Default)]
pub struct FallbackIndexStub;

impl FallbackIndexStub {
    pub fn query(
        &self,
        _search: &str,
        _max_results: u32,
        offset: u32,
    ) -> EverythingQueryResult {
        EverythingQueryResult {
            total_count: 0,
            offset: offset as u64,
            hits: Vec::<DiscoveryHit>::new(),
        }
    }
}
