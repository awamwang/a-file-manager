// SPDX-License-Identifier: GPL-3.0-or-later

use super::backend::EverythingBackend;
use super::types::{DiscoveryHit, EverythingProbe, EverythingQueryResult};
use std::sync::Mutex;

/// In-memory Everything stand-in for unit tests (no IPC / no install).
#[derive(Debug, Default)]
pub struct MockEverythingBackend {
    available: Mutex<bool>,
    hits: Mutex<Vec<DiscoveryHit>>,
}

impl MockEverythingBackend {
    pub fn new(available: bool, hits: Vec<DiscoveryHit>) -> Self {
        Self {
            available: Mutex::new(available),
            hits: Mutex::new(hits),
        }
    }
}

impl EverythingBackend for MockEverythingBackend {
    fn probe(&self) -> EverythingProbe {
        let available = *self.available.lock().expect("mock available lock");
        EverythingProbe {
            available,
            detail: if available {
                "mock Everything IPC available".to_string()
            } else {
                "mock Everything IPC unavailable".to_string()
            },
        }
    }

    fn query(
        &self,
        search: &str,
        max_results: u32,
        offset: u32,
    ) -> Result<EverythingQueryResult, String> {
        if !self.probe().available {
            return Err("Everything IPC unavailable (mock)".to_string());
        }

        let needle = search.to_ascii_lowercase();
        let all: Vec<DiscoveryHit> = self
            .hits
            .lock()
            .expect("mock hits lock")
            .iter()
            .filter(|hit| {
                hit.name.to_ascii_lowercase().contains(&needle)
                    || hit.path.to_ascii_lowercase().contains(&needle)
            })
            .cloned()
            .collect();

        let total_count = all.len() as u64;
        let start = (offset as usize).min(all.len());
        let end = (start + max_results as usize).min(all.len());

        Ok(EverythingQueryResult {
            total_count,
            offset: offset as u64,
            hits: all[start..end].to_vec(),
        })
    }
}
