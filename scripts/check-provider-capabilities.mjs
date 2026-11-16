// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 7.24 / 6.5 §4 guard: capability bits must be DERIVED from the providers the
 * active adapter registers, and business entry points must go through the
 * neutral platform layer. Comments are stripped before matching so a doc line
 * can never satisfy an assertion (same trap as 7.21 / 7.22).
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

/** Rust/TS source with line + block comments removed (strings are preserved). */
function code(source) {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .map((line) => line.replace(/\/\/.*$/, ""))
    .join("\n");
}

const platform = code(read("src-tauri/src/platform/mod.rs"));
const windows = code(read("src-tauri/src/platform/windows.rs"));
const macos = code(read("src-tauri/src/platform/macos.rs"));
const fileOps = code(read("src-tauri/src/file_ops/mod.rs"));
const clip = code(read("src-tauri/src/file_ops/clipboard.rs"));
const recycle = code(read("src-tauri/src/file_ops/recycle.rs"));
const platformTs = code(read("src/config/platform.ts"));

// 1. Neutral contract owns both provider traits + their registration slots.
for (const needle of [
  "pub trait TrashProvider",
  "pub trait ClipboardProvider",
  "fn supports_list_restore(&self) -> bool",
  "fn trash_provider(&self) -> Option<&dyn TrashProvider>",
  "fn clipboard_provider(&self) -> Option<&dyn ClipboardProvider>",
  "pub fn apply_provider_capabilities(&mut self, platform: &dyn Platform)",
  "pub fn delete_to_trash(",
  "pub fn trash_list(",
  "pub fn trash_restore(",
  "pub fn clipboard_set(",
  "pub fn clipboard_get(",
  "pub fn clipboard_clear(",
  "pub(crate) fn assert_provider_capability_contract(",
]) {
  assert.ok(platform.includes(needle), `platform/mod.rs missing ${needle}`);
}

// 2. Every provider-backed flag is derived inside one shared helper, never
//    hand-assigned in an adapter.
for (const needle of [
  "self.trash_delete = platform.trash_provider().is_some()",
  "self.shell_clipboard = platform.clipboard_provider().is_some()",
  "platform.global_search_engine() != GlobalSearchEngine::None",
]) {
  assert.ok(platform.includes(needle), `capability derivation missing: ${needle}`);
}
for (const [name, source] of [
  ["windows.rs", windows],
  ["macos.rs", macos],
]) {
  assert.ok(
    source.includes("caps.apply_provider_capabilities(self)"),
    `${name} must derive provider-backed capabilities`,
  );
  assert.ok(
    !/caps\.(trash_delete|trash_list_restore|shell_clipboard|global_search)\s*=/.test(source),
    `${name} must not hand-declare provider-backed capability bits`,
  );
  assert.ok(source.includes("fn trash_provider(&self)"), `${name} must register a trash provider`);
  assert.ok(
    source.includes("fn clipboard_provider(&self)"),
    `${name} must declare its clipboard provider`,
  );
  assert.ok(
    source.includes("assert_provider_capability_contract(&"),
    `${name} must run the shared provider contract test`,
  );
}
const macClipBody = macos
  .split("fn clipboard_provider(&self) -> Option<&dyn ClipboardProvider> {")[1]
  ?.split("\n    }")[0];
assert.ok(
  windows.includes("Some(&super::NATIVE_CLIPBOARD)") && /^\s*None\s*$/m.test(macClipBody ?? ""),
  "CF_HDROP clipboard stays Windows-only; macOS registers no provider",
);
assert.ok(
  windows.includes("Some(&super::NATIVE_TRASH)") && macos.includes("Some(&super::NATIVE_TRASH)"),
  "both adapters register the trash provider",
);

// 3. Business entry points delegate; OS backends are not called from commands.
assert.ok(fileOps.includes("crate::platform::delete_to_trash(&pb, path)"));
assert.ok(
  fileOps.includes("crate::platform::trash_list()") &&
    fileOps.includes("crate::platform::trash_restore(&paths)"),
  "recycle list / restore commands must go through the provider",
);
assert.ok(
  !/recycle::list_recycle_bin\(\)/.test(fileOps) &&
    !/recycle::restore_original_paths\(/.test(fileOps),
  "file_ops commands must not call the Windows backend directly",
);
assert.ok(
  clip.includes("crate::platform::clipboard_set(&paths, &mode)") &&
    clip.includes("crate::platform::clipboard_get()") &&
    clip.includes("crate::platform::clipboard_clear()"),
  "clipboard commands must go through the provider",
);
assert.ok(
  !/file_ops_clipboard_set[\s\S]{0,160}set_shell_clipboard\(/.test(clip) &&
    !/file_ops_clipboard_get[\s\S]{0,160}get_shell_clipboard\(/.test(clip),
  "clipboard commands must not call the Windows backend directly",
);

// 4. Neutral DTOs are owned by the platform; `file_ops` only re-exports them.
assert.ok(clip.includes("pub use crate::platform::ShellClipboard;"));
assert.ok(recycle.includes("pub use crate::platform::RecycleBinItem;"));
assert.ok(platform.includes("pub struct ShellClipboard {"));
assert.ok(platform.includes("pub struct RecycleBinItem {"));

// 5. Frontend shape mirrors the new derived field (exact field match is also
//    checked by shortcut-checks.ts against the Rust struct).
assert.ok(platform.includes("pub shell_clipboard: bool"));
assert.ok(platformTs.includes("shellClipboard: boolean;"));
assert.ok(platformTs.includes("shellClipboard: false,"));

console.log("Provider capability checks OK (6.5 §4 / 7.24).");
