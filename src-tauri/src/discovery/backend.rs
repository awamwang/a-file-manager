// SPDX-License-Identifier: GPL-3.0-or-later

use super::types::{DiscoveryEngine, EverythingProbe, EverythingQueryResult};

/// Product path: Everything IPC/SDK (ADR 0002). Must not shell `es.exe`.
/// 7.21: the trait is now the neutral host-index contract — macOS plugs in
/// Spotlight through `PlatformSearchBackend` without changing the service.
pub trait EverythingBackend: Send + Sync {
    /// Which engine this backend represents, so `DiscoveryStatus` can report
    /// the real host index instead of always naming Everything.
    fn engine(&self) -> DiscoveryEngine {
        DiscoveryEngine::Everything
    }

    fn probe(&self) -> EverythingProbe;

    fn query(
        &self,
        search: &str,
        max_results: u32,
        offset: u32,
    ) -> Result<EverythingQueryResult, String>;
}
