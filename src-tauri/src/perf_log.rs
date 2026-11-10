// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in AFM_PERF line logger (stderr + optional file).
//! Enable with `AFM_PERF=1`; set `AFM_PERF_LOG=<path>` to also append lines to a file
//! (needed on Windows GUI subsystem where stderr is often discarded).

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::OnceLock;

fn enabled() -> bool {
    std::env::var_os("AFM_PERF").is_some()
}

fn log_path() -> Option<&'static str> {
    static PATH: OnceLock<Option<String>> = OnceLock::new();
    PATH.get_or_init(|| std::env::var("AFM_PERF_LOG").ok())
        .as_deref()
}

/// Emit one AFM_PERF line when the env flag is set.
pub fn afm_perf_line(line: impl AsRef<str>) {
    if !enabled() {
        return;
    }
    let line = line.as_ref();
    eprintln!("{line}");
    if let Some(path) = log_path() {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }
}
