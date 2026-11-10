// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Opt-in operation spans (`[AFM_perf]`).
 * Enable with `localStorage.setItem("afm.perfProbe", "1")` then reload;
 * always no-ops when disabled so release builds stay quiet.
 */

export type PerfDetail = Record<string, string | number | boolean | null | undefined>;

function enabled(): boolean {
  try {
    return globalThis.localStorage?.getItem("afm.perfProbe") === "1";
  } catch {
    return false;
  }
}

/** Format optional detail bag as `k=v` pairs for the console line. */
function formatDetail(detail?: PerfDetail): string {
  if (!detail) return "";
  const parts: string[] = [];
  for (const [key, value] of Object.entries(detail)) {
    if (value === undefined || value === null || value === "") continue;
    parts.push(`${key}=${String(value)}`);
  }
  return parts.length ? ` ${parts.join(" ")}` : "";
}

function resolveDetail(
  detail: PerfDetail | (() => PerfDetail) | undefined,
): PerfDetail | undefined {
  if (typeof detail === "function") return detail();
  return detail;
}

/** Instant mark (no duration). */
export function perfMark(name: string, detail?: PerfDetail): void {
  if (!enabled()) return;
  console.info(`[AFM_perf] ${name}${formatDetail(detail)}`);
}

/**
 * Time an async or sync operation and log `[AFM_perf] <name> <ms>ms …`.
 * When disabled, runs `fn` with no timing overhead beyond one flag check.
 * `detail` may be a thunk evaluated after `fn` so callers can include `n=…`.
 */
export async function perfSpan<T>(
  name: string,
  fn: () => Promise<T> | T,
  detail?: PerfDetail | (() => PerfDetail),
): Promise<T> {
  if (!enabled()) {
    return await fn();
  }
  const t0 = performance.now();
  try {
    return await fn();
  } finally {
    const ms = (performance.now() - t0).toFixed(1);
    console.info(`[AFM_perf] ${name} ${ms}ms${formatDetail(resolveDetail(detail))}`);
  }
}
