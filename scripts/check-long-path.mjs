// SPDX-License-Identifier: GPL-3.0-or-later
// LC-009 (已拍板 2026-11-22): 长路径（>260 字符）支持。
//   1. 出厂 exe 的 Windows manifest 必须声明 longPathAware=true（配合系统
//      LongPathsEnabled，Win32 API 才接受 >MAX_PATH 的普通路径）；
//   2. 传输/重命名代码层必须能往返 \\?\ 扩展形态的深路径（cargo
//      `long_path_ops_roundtrip` 真执行）。
// 本 gate 钉第 1 层 manifest 声明与第 2 层测试存在；行为由 cargo 测试保证。
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const build = readFileSync(join(root, "src-tauri/build.rs"), "utf8");
const ops = readFileSync(join(root, "src-tauri/src/file_ops/mod.rs"), "utf8");

// 第 1 层：manifest 声明。app_manifest 是整体替换——丢了这段 Windows 就退回
// UAC/长路径启发式，所以必须显式钉住。
assert.ok(
  build.includes(
    '<longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>',
  ),
  "LC-009: the app manifest must declare longPathAware=true",
);
// 替换式 manifest 必须继续保留 asInvoker 与 Common-Controls（build.rs 注释契约）。
assert.ok(build.includes('requestedExecutionLevel level="asInvoker"'));
assert.ok(build.includes("Microsoft.Windows.Common-Controls"));

// 第 2 层：cargo 行为测试存在且只在 Windows 跑。
assert.ok(
  /#\[cfg\(windows\)\]\s*#\[test\]\s*fn long_path_ops_roundtrip\(\)/.test(ops),
  "LC-009: the \\?\ extended long-path roundtrip test must exist",
);

console.log("Long-path support checks OK (LC-009).");
