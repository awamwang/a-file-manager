// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  filterUnsupportedBackends,
  shouldHideWindowsOnlyBackends,
  WINDOWS_ONLY_BACKEND_IDS,
} from "../src/state/provider/platformFilter.ts";
import { NEUTRAL_PLATFORM } from "../src/config/platform.ts";

const backends = [
  { id: "local" },
  { id: "rclone" },
  { id: "smb" },
  { id: "volumes" },
  { id: "places" },
  { id: "wsl" },
  { id: "filehistory" },
  { id: "knowledge" },
];

const mac = { ...NEUTRAL_PLATFORM, driveLetters: false, os: "macos" as const };
const win = { ...NEUTRAL_PLATFORM, driveLetters: true, os: "windows" as const };

for (const id of ["smb", "volumes", "places", "wsl", "filehistory"]) {
  assert.ok(WINDOWS_ONLY_BACKEND_IDS.has(id), `${id} must be in the Windows-only set`);
}
assert.equal(shouldHideWindowsOnlyBackends(mac), true);
assert.equal(shouldHideWindowsOnlyBackends(win), false);
// Neutral (capabilities not hydrated yet) stays Windows-first: no hiding.
assert.equal(shouldHideWindowsOnlyBackends(NEUTRAL_PLATFORM), false);

const onMac = filterUnsupportedBackends(backends, mac).map((b) => b.id);
assert.deepEqual(onMac, ["local", "rclone", "knowledge"]);
const onWindows = filterUnsupportedBackends(backends, win);
assert.equal(onWindows, backends, "Windows keeps the same array reference");
const neutral = filterUnsupportedBackends(backends, NEUTRAL_PLATFORM);
assert.equal(neutral, backends, "unknown host keeps the same array reference");

console.log("Provider platform filter checks OK (macOS hides Windows-only backends).");
