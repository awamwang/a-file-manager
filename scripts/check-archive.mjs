// SPDX-License-Identifier: GPL-3.0-only
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const rust = readFileSync(join(root, "src-tauri/src/archive/mod.rs"), "utf8");
const lib = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const cargo = readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8");
const model = readFileSync(
  join(root, "src/components/navigator/contextMenuModel.ts"),
  "utf8",
);
const menu = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

// Dependencies: ZIP-only (deflate), directory recursion via walkdir.
assert.ok(
  cargo.includes('zip = { version = "2"') && cargo.includes('"deflate"'),
  "zip crate present with the deflate feature (RAR/7z are out of scope)",
);
assert.ok(cargo.includes('walkdir = "2"'), "walkdir dependency present");

// Module + command registration; frontend/backend command names must match.
assert.ok(lib.includes("mod archive;"), "archive module declared");
assert.ok(lib.includes("archive::archive_compress_zip"), "compress command registered");
assert.ok(lib.includes("archive::archive_extract_zip"), "extract command registered");
assert.match(
  rust,
  /#\[tauri::command\][\s\S]*?pub fn archive_compress_zip/,
  "archive_compress_zip is a tauri command",
);
assert.match(
  rust,
  /#\[tauri::command\][\s\S]*?pub fn archive_extract_zip/,
  "archive_extract_zip is a tauri command",
);

// Backend behavior: deflate, reader/writer wired, camelCase result field.
assert.ok(rust.includes("CompressionMethod::Deflated"), "compression must be deflate");
assert.ok(
  rust.includes("ZipWriter::new") && rust.includes("ZipArchive::new"),
  "zip writer + reader are wired",
);
assert.ok(rust.includes("WalkDir::new"), "directory recursion via walkdir");
assert.ok(
  rust.includes('rename_all = "camelCase"') && rust.includes("output_path"),
  "ArchiveResult serializes output_path as camelCase",
);

// Security contract: extraction must reject zip-slip (path traversal).
assert.ok(rust.includes("fn sanitize_zip_entry_path"), "zip-slip sanitizer present");
assert.ok(
  rust.includes(
    "Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None",
  ),
  "zip-slip sanitizer rejects escaping components",
);
assert.ok(rust.includes("Unsafe zip entry path"), "unsafe entries abort extraction");

// Cargo-side regression tests must exist (behavioral guard).
assert.ok(rust.includes("fn rejects_zip_slip"), "zip-slip regression test present");
assert.ok(
  rust.includes("fn compress_and_extract_roundtrip"),
  "roundtrip regression test present",
);

// Frontend menu items + case-insensitive isZipEntry gate.
assert.ok(model.includes('id: "compressZip"'), "compressZip menu item");
assert.ok(model.includes('id: "extractZip"'), "extractZip menu item");
assert.ok(model.includes("isZipEntry(entry)"), "extractZip gated by isZipEntry");
assert.ok(
  model.includes("entry.name.toLowerCase()") && model.includes('.endsWith(".zip")'),
  "isZipEntry lowercases the name and matches .zip",
);

// Frontend invokes the exact backend command names with camelCase args.
assert.ok(
  menu.includes('invoke<ArchiveResult>("archive_compress_zip"'),
  "vue invokes archive_compress_zip",
);
assert.ok(
  menu.includes('invoke<ArchiveResult>("archive_extract_zip"'),
  "vue invokes archive_extract_zip",
);
assert.ok(
  menu.includes("destZip") && menu.includes("zipPath") && menu.includes("destDir"),
  "camelCase invoke arguments (destZip/zipPath/destDir)",
);

// Source-of-truth strings (en).
assert.equal(en.contextMenu.compressZip, "Compress to ZIP");
assert.equal(en.contextMenu.extractZip, "Extract ZIP");

// Behavioral unit: menu gating incl. case-insensitive extract.
const unit = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/archive-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (unit.stdout) process.stdout.write(unit.stdout);
if (unit.stderr) process.stderr.write(unit.stderr);
assert.equal(unit.status, 0, "archive behavior checks failed");

console.log(
  "Archive checks OK (ZIP compress/extract v1: deflate, zip-slip guard, FE/BE command-name parity).",
);
