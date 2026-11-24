// SPDX-License-Identifier: GPL-3.0-or-later
// --afm-perf-baseline 自驱性能基线诊断模式（S1/S5）：
// 启动参数/环境变量 → 前端 runner 自动采集 → perf_baseline_report 打回终端 → 自动退出。
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const libRs = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const modRs = readFileSync(join(root, "src-tauri/src/perf_baseline.rs"), "utf8");
const mainTs = readFileSync(join(root, "src/main.ts"), "utf8");
const startup = readFileSync(join(root, "src/state/startupTiming.ts"), "utf8");
const probe = readFileSync(join(root, "src/state/perfProbe.ts"), "utf8");
const runner = readFileSync(join(root, "src/state/perfBaseline.ts"), "utf8");
const pkg = readFileSync(join(root, "package.json"), "utf8");

// FE↔BE parity (ADR 0025): invoke names == command fn names, both registered.
assert.ok(modRs.includes("pub fn perf_baseline_enabled"), "BE command perf_baseline_enabled");
assert.ok(modRs.includes("pub fn perf_baseline_report"), "BE command perf_baseline_report");
assert.ok(
  libRs.includes("perf_baseline::perf_baseline_enabled") &&
    libRs.includes("perf_baseline::perf_baseline_report"),
  "commands registered in generate_handler",
);
assert.ok(
  runner.includes('invoke<boolean>("perf_baseline_enabled")'),
  "FE queries perf_baseline_enabled",
);
assert.ok(
  runner.includes('invoke("perf_baseline_report"'),
  "FE reports via perf_baseline_report",
);

// Launch flag: CLI arg + env var; diagnostic run may bypass single-instance.
assert.ok(modRs.includes("--afm-perf-baseline"), "CLI flag recognized");
assert.ok(modRs.includes("AFM_PERF_BASELINE"), "env var recognized");
assert.ok(
  libRs.includes("perf_baseline::detect_flag(&raw_args)"),
  "detect_flag wired in run()",
);
assert.ok(
  /!primary_instance && add_favorite_path\.is_none\(\) && !perf_baseline/.test(libRs),
  "baseline run bypasses the single-instance early return (no tray)",
);

// S1: marks always recorded in-memory so the runner can dump the chain without
// racing boot (console printing stays behind the localStorage switch).
assert.ok(startup.includes("export function startupMarks"), "startupMarks exported");
assert.ok(startup.includes("marks.push({ label, ms })"), "marks always recorded");
assert.ok(runner.includes("startupMarks("), "runner dumps the S1 chain");
assert.ok(
  runner.includes('startupMark("first_listing_ready")'),
  "runner completes the chain with first_listing_ready",
);

// S5: span recording + two-tab round-trip sampling + auto exit.
assert.ok(probe.includes("export function setSpanRecording"), "setSpanRecording exported");
assert.ok(probe.includes("export function getRecordedSpans"), "getRecordedSpans exported");
assert.ok(
  probe.includes("MAX_RECORDED_SPANS") && probe.includes("recordedSpans.length < MAX_RECORDED_SPANS"),
  "span recording is capped",
);
assert.ok(runner.includes("setSpanRecording(true)"), "runner enables span recording");
assert.ok(runner.includes("newNavigatorTab"), "runner opens the second tab");
assert.ok(runner.includes("switchNavigatorTab"), "runner samples tab switches");
assert.ok(runner.includes("closeNavigatorTab"), "runner closes its temp tab (session restored)");
assert.ok(runner.includes('invoke("app_quit")'), "runner exits the app after reporting");
assert.ok(
  runner.includes("getRecordedSpans") && runner.includes("report(`spans "),
  "runner dumps recorded spans",
);

// Boot wiring + gate wired into the aggregated chain.
assert.ok(mainTs.includes("runPerfBaselineIfEnabled"), "runner wired at boot");
assert.ok(pkg.includes('"test:perf-baseline"'), "test:perf-baseline defined");
assert.match(
  pkg,
  /"test": "[^"]*test:perf-baseline/,
  "test:perf-baseline in the aggregated test chain",
);

console.log(
  "perf-baseline launch mode OK (--afm-perf-baseline / AFM_PERF_BASELINE=1; S1/S5 self-drive, CLI report).",
);
