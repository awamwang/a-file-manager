// Mock of @tauri-apps/api/core — intercepts every invoke() and routes to the
// stateful virtual backend (vfs.ts).
import { handle } from "./vfs.ts";

export async function invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  // tiny async boundary to mimic IPC round-trip
  const res = handle(cmd, args || {});
  return res as T;
}

export function convertFileSrc(path: string, _proto?: string): string {
  // no asset protocol in browser demo; return a harmless placeholder
  return "data:image/svg+xml," + encodeURIComponent(
    `<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'></svg>`
  );
}

export function __clearCache() {}
export function hasWindowLoaded() { return true; }
