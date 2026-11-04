// SPDX-License-Identifier: GPL-3.0-or-later

use super::types::{EverythingProbe, EverythingQueryResult};

/// Product path: Everything IPC/SDK (ADR 0002). Must not shell `es.exe`.
pub trait EverythingBackend: Send + Sync {
    fn probe(&self) -> EverythingProbe;

    fn query(
        &self,
        search: &str,
        max_results: u32,
        offset: u32,
    ) -> Result<EverythingQueryResult, String>;
}
