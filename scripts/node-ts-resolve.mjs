// SPDX-License-Identifier: GPL-3.0-or-later
/** Resolve bare relative `.ts` imports + stub Tauri for Node session checks. */
import { pathToFileURL } from "node:url";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

export async function resolve(specifier, context, nextResolve) {
  if (specifier === "@tauri-apps/api/core") {
    return {
      shortCircuit: true,
      url: pathToFileURL(join(here, "mocks", "tauri-core.mjs")).href,
    };
  }
  if (specifier === "@tauri-apps/plugin-opener") {
    return {
      shortCircuit: true,
      url: pathToFileURL(join(here, "mocks", "tauri-opener.mjs")).href,
    };
  }

  if (
    (specifier.startsWith("./") || specifier.startsWith("../")) &&
    !/\.[cm]?[jt]sx?$/.test(specifier)
  ) {
    try {
      return await nextResolve(`${specifier}.ts`, context);
    } catch {
      // fall through
    }
  }

  return nextResolve(specifier, context);
}
