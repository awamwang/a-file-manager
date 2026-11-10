// SPDX-License-Identifier: GPL-3.0-or-later
/** FIL-30: Skill 导入不跟随 junction；打开工作区 / 导入 IPC 异步化，避免互相卡死 UI。 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const skills = read("src-tauri/src/ai/skills.rs");
const deploy = read("src-tauri/src/ai/deploy.rs");
const aiMod = read("src-tauri/src/ai/mod.rs");
const openWith = read("src-tauri/src/open_with/mod.rs");
const projects = read("src/views/ProjectsView.vue");

// 导入拷贝跳过 symlink/junction，避免回链 Skill 库无限递归
assert.match(
  skills,
  /fn copy_dir_recursive[\s\S]*?if ty\.is_symlink\(\)\s*\{\s*continue;/,
  "skills copy_dir_recursive must skip symlinks/junctions",
);
assert.match(
  deploy,
  /fn copy_dir_recursive[\s\S]*?if ty\.is_symlink\(\)\s*\{\s*continue;/,
  "deploy copy_dir_recursive must skip symlinks/junctions",
);
assert.ok(
  skills.includes("import_skips_junction_back_into_library"),
  "regression test for junction-back-into-library import",
);

// 导入 IPC：async + spawn_blocking（FIL-27 同款）
assert.match(
  aiMod,
  /#\[tauri::command\][\s\S]*?pub async fn ai_skills_import[\s\S]*?spawn_blocking/,
  "ai_skills_import must be async + spawn_blocking",
);

// 打开路径：async + spawn_blocking；目录走 explorer
assert.match(
  openWith,
  /#\[tauri::command\][\s\S]*?pub async fn open_path_default[\s\S]*?spawn_blocking/,
  "open_path_default must be async + spawn_blocking",
);
assert.ok(
  openWith.includes("open_folder_in_explorer"),
  "folder open helper must exist",
);
assert.match(
  openWith,
  /fn open_path_default_inner[\s\S]*?if p\.is_dir\(\)[\s\S]*?open_folder_in_explorer/,
  "open_path_default must open directories via explorer spawn",
);

// 项目配置区仍走 open_path_default
assert.ok(projects.includes("async function openWorkspaceFolder"));
assert.ok(projects.includes('"open_path_default"'));

console.log("check-fil30-skill-open-freeze: ok");
