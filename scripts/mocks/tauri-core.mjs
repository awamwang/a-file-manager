// SPDX-License-Identifier: GPL-3.0-or-later
/** Minimal invoke stub for Node navigator-session checks. */

const handlers = new Map();

export function __setInvokeHandler(cmd, fn) {
  handlers.set(cmd, fn);
}

export function __clearInvokeHandlers() {
  handlers.clear();
}

export async function invoke(cmd, args = {}) {
  const handler = handlers.get(cmd);
  if (handler) {
    return handler(args);
  }
  throw new Error(`tauri invoke stub: unhandled command ${cmd}`);
}
