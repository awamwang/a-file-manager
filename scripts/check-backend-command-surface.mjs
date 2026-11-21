// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Backend command-surface guard for the three Tauri domains whose behavior lives in Rust and
 * is reached only through invoke-handler commands: knowledge_service, managed_dependency,
 * extension_host.
 *
 * Why this gate exists (ADR 0025): these domains already carry offline cargo `#[cfg(test)]`
 * for their internals, but NOTHING pinned the IPC surface itself — so a rename / accidental
 * de-registration of a command silently breaks the frontend at runtime with no test going red.
 * This pins, deterministically and offline:
 *   1. every expected command is still registered in lib.rs's invoke_handler;
 *   2. frontend parity: every "<domain>_…" command string the frontend invokes resolves to a
 *      registered command (catches FE calling a renamed/removed command);
 *   3. the managed-dependency install chain keeps its effective offline tests + the network-stack
 *      roundtrip stays deliberately #[ignore]d (not silently deleted, not promoted to mandatory).
 *
 * It intentionally does NOT re-run cargo (test:rust executes these tests already) and adds no
 * network dependency.
 */
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

// --- 1. registration surface (lib.rs invoke_handler) -----------------------------------------
const DOMAINS = {
  "knowledge_service": { module: "knowledge_service::", cmds: [
    "knowledge_service_status", "knowledge_service_search", "knowledge_service_read_doc",
    "knowledge_service_vault_status", "knowledge_service_reveal", "knowledge_service_copy",
    "knowledge_service_unlock", "knowledge_service_lock",
  ] },
  "managed_dependency": { module: "managed_dependency::commands::", cmds: [
    "managed_dependency_list", "managed_dependency_ensure", "managed_dependency_stop",
    "managed_dependency_upgrade", "managed_dependency_check_update",
    "managed_dependency_check_all_updates", "managed_dependency_set_feed",
    "managed_dependency_get_feed",
  ] },
  "extension_host": { module: "extension_host::", cmds: [
    "extension_host_status", "extension_host_install_pack", "extension_host_list_installable",
    "extension_host_install_pack_from_feed", "extension_host_list_packs",
    "extension_host_set_pack_enabled", "extension_host_uninstall_pack",
    "extension_host_list_contributions",
  ] },
};

const lib = read("src-tauri/src/lib.rs");
const registered = new Set();
for (const [domain, { module, cmds }] of Object.entries(DOMAINS)) {
  for (const cmd of cmds) {
    assert.ok(
      lib.includes(`${module}${cmd}`),
      `${domain}: command ${cmd} is NOT registered in lib.rs invoke_handler (renamed/removed?)`,
    );
    registered.add(cmd);
  }
}

// --- 2. frontend parity: every "<domain>_…" literal in src must be a registered command -------
const SRC = join(root, "src");
const LITERAL = /["']((?:knowledge_service|managed_dependency|extension_host)_[a-z0-9_]+)["']/g;
const offenders = [];
let seenFe = 0;
function walk(dir) {
  for (const name of readdirSync(dir)) {
    const abs = join(dir, name);
    const st = statSync(abs);
    if (st.isDirectory()) { walk(abs); continue; }
    if (!/\.(ts|vue)$/.test(name)) continue;
    const text = readFileSync(abs, "utf8");
    for (const m of text.matchAll(LITERAL)) {
      seenFe += 1;
      if (!registered.has(m[1])) {
        offenders.push(`${relative(root, abs)} -> ${m[1]}`);
      }
    }
  }
}
walk(SRC);
assert.ok(seenFe > 0, "frontend parity: no backend command literals found in src (scan broken?)");
assert.deepEqual(
  offenders,
  [],
  "frontend invokes command names that are not registered in lib.rs:\n" + offenders.join("\n"),
);

// --- 3. managed install chain keeps its effective offline tests (+ network test stays ignored) --
const provision = read("src-tauri/src/managed_dependency/provision.rs");
for (const fn of [
  "fn force_provision_rejects_disallowed_host_before_fetch", // allow-list enforced BEFORE any fetch
  "fn ensure_artifact_downloads_to_sidecar_dir_when_missing", // download -> SHA verify -> atomic land
  "fn force_provision_overwrites_existing_and_verifies",      // manual upgrade overwrite path
]) {
  assert.ok(provision.includes(fn), `provision.rs lost offline install-chain test: ${fn}`);
}
assert.ok(
  /#\[ignore[^\]]*\]\s*\n\s*fn ensure_artifact_real_http_feed_download_roundtrip/.test(provision),
  "the real-HTTP feed download test must stay present AND #[ignore]d (offline CI), never deleted",
);

console.log(
  `backend-command-surface checks OK (24 commands registered; ${seenFe} FE command literals all resolve to registered commands; managed install-chain tests intact).`,
);
