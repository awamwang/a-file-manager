// SPDX-License-Identifier: GPL-3.0-or-later
//! `--afm-perf-baseline` / `AFM_PERF_BASELINE=1` 自驱性能基线诊断模式（S1/S5）。
//!
//! 前端 `src/state/perfBaseline.ts` 采集后经 `perf_baseline_report` 逐行打回
//! 启动终端（`[AFM_BASELINE] …`）。debug 构建继承启动它的终端（`npm run tauri
//! dev` / `cargo run`）；release GUI exe 无控制台，基线一律用 dev 构建跑。
//! ADR 0025：诊断启动参数，不进门禁链；基线运行允许与已驻留实例并存
//! （绕过单实例锁，不建托盘），报告完由前端调 `app_quit` 退出。

use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Detect the baseline launch flag. Called once from `run()` before the
/// single-instance check so a diagnostic run may proceed beside a live instance.
pub fn detect_flag(args: &[String]) -> bool {
    let env_on = std::env::var("AFM_PERF_BASELINE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let on = env_on || args.iter().any(|a| a == "--afm-perf-baseline");
    ENABLED.store(on, Ordering::SeqCst);
    if on {
        println!("[AFM_BASELINE] perf-baseline launch mode enabled");
    }
    on
}

#[tauri::command]
pub fn perf_baseline_enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

/// One frontend-collected baseline line → the launching terminal.
#[tauri::command]
pub fn perf_baseline_report(line: String) {
    println!("[AFM_BASELINE] {line}");
}
