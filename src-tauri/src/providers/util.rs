// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs::Metadata;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn normalize_path(path: &str) -> String {
    path.replace('/', "\\")
}

pub fn metadata_modified_time_unix_ms(metadata: &Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[allow(dead_code)]
pub fn system_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
