// SPDX-License-Identifier: GPL-3.0-or-later
/** Expand Windows-style %VAR% segments in address-bar paths. */

function readProcessEnv(): Record<string, string | undefined> {
  const g = globalThis as {
    process?: { env?: Record<string, string | undefined> };
  };
  return g.process?.env ?? {};
}

export function expandEnvPath(
  input: string,
  env: Record<string, string | undefined> = readProcessEnv(),
): string {
  return input.replace(/%([^%]+)%/g, (match, name: string) => {
    const key = name.trim();
    if (!key) return match;
    const direct = env[key];
    if (direct != null && direct !== "") return direct;
    const upper = env[key.toUpperCase()];
    if (upper != null && upper !== "") return upper;
    // Browser / Tauri webview: process.env may be empty; keep token for backend.
    return match;
  });
}
