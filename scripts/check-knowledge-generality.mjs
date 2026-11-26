// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * knowledge 扩展「通用性」门控（以 knowledge 管理为例，校验扩展功能是否真的支持
 * 所有以 Markdown 为格式的知识目录，而非被硬编码 / 窄化到某一固定来源）。
 *
 * 两道防线：
 *  1) 源码静态不变量：检索/索引内核按扩展名（.md/.markdown，大小写不敏感）+ 递归全量
 *     遍历任意根目录；pack.json 的知识根由用户设置占位符驱动（无硬编码绝对路径）。
 *  2) 行为测试：upstream/awam-knowledge 的两份通用性自检（pipeline/build_index 侧 +
 *     运行时检索内核侧）必须通过。
 *
 * 缺一环 = 静默回归：换成别的 Markdown 目录、恰含 pages/、用 .markdown/大写扩展名、
 * 或把索引根写死到某台机器的路径时，knowledge 不再「通用」。
 */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const km = join(root, "upstream", "awam-knowledge");
const readFrom = (dir, rel) => readFileSync(join(dir, rel), "utf8");

// --- 1. 检索内核（AFM sidecar 真正对外的 /search 走 bm25）：扩展名 + 递归全量 ---
const bmPy = readFrom(km, "src/awam_knowledge/bm25.py");
assert.ok(
  bmPy.includes('_RE_MD = re.compile(r"\\.(md|markdown)$", re.IGNORECASE)'),
  "bm25 检索内核必须按 .md/.markdown 大小写不敏感识别 Markdown（扩展名通用）",
);
assert.ok(
  bmPy.includes("def walk(dir_path: Path)") && bmPy.includes("walk(root)"),
  "bm25 检索内核必须从根目录递归遍历（默认全量，不窄化到固定子目录）",
);
// 不允许出现「存在 pages/ 或 journals/ 就只扫这两个目录」一类窄化分支
assert.ok(
  !/if.*(pages|journals).*(return|only)/.test(bmPy),
  "bm25 检索内核不得按 pages/journals 窄化扫描范围（混合布局回归）",
);

// --- 2. build_index 索引器：NOTE_EXTS 集 + 默认全量 + 白/黑名单 ---
const buildPy = readFrom(km, "src/awam_knowledge/km_build_index.py");
assert.ok(
  /NOTE_EXTS\s*=\s*\{[^}]*\.md[^}]*\.markdown[^}]*\}/.test(buildPy),
  "build_index NOTE_EXTS 必须同时含 .md 与 .markdown",
);
assert.ok(
  buildPy.includes('p.suffix.lower() not in NOTE_EXTS'),
  "build_index 必须大小写不敏感匹配扩展名（大写 .MD/.Markdown 通用）",
);
assert.ok(
  buildPy.includes("root.rglob(") && buildPy.includes("默认全量"),
  "iter_note_files 必须默认全量递归扫描（恰含 pages/ 不再窄化）",
);
assert.ok(
  buildPy.includes("include_dirs") && buildPy.includes("skip_dirs"),
  "iter_note_files 必须支持白名单 include_dirs 与黑名单 skip_dirs（通用配置）",
);

// --- 3. Node 参照实现：扩展名口径与 Python 一致（parity 防漂移）---
const bmJs = readFrom(km, "code/bm25-index.js");
assert.ok(
  bmJs.includes("(md|markdown)$/i"),
  "Node 检索内核必须用 /\\.(md|markdown)$/i（与 Python 扩展名口径一致）",
);

// --- 3b. 黑名单大小写不敏感（2026-11-26 修复：Windows/macOS 大小写不敏感 FS 下
//     .GIT/Node_Modules/.Recycle/VAULT 等大写目录须与 build 侧 p.lower() 一致被排除，
//     否则内部/工具目录在对外 /search 泄露；Python 与 Node 双向锁定以保持 parity）---
assert.ok(
  bmPy.includes("norm_path(rel_path).lower()") && bmPy.includes("norm_path(e).lower()"),
  "Python bm25.is_excluded 必须对路径与黑名单项小写（黑名单大小写不敏感，与 iter_note_files 口径一致）",
);
assert.ok(
  bmJs.includes("normPath(relPath).toLowerCase()") && bmJs.includes("normPath(e).toLowerCase()"),
  "Node bm25-index.isExcluded 必须对路径与黑名单项小写（与 Python is_excluded 保持 parity）",
);

// --- 3c. 链接环路防护（link-safe）：符号链接 + NTFS junction / reparse point 一律不跟随。
//     Node 靠 libuv Dirent 把重解析点判为 symlink、isDirectory()=false 自然跳过；
//     Python 的 is_symlink() 不识别 junction，须用 _is_link_like（is_symlink ∪ REPARSE_POINT）
//     对齐，否则自环 junction 会让 walk 撞 MAX_PATH 生成大量重复伪文档。---
assert.ok(
  bmPy.includes("FILE_ATTRIBUTE_REPARSE_POINT") &&
    bmPy.split("if _is_link_like(e):").length - 1 >= 2,
  "Python bm25 两处 walk 必须用 _is_link_like 跳过符号链接/junction/reparse point（link-safe 环路防护）",
);
assert.ok(
  bmJs.split("link-safe").length - 1 >= 2,
  "Node bm25-index 两处 walk 必须标注 link-safe（符号链接/junction 不递归，与 Python _is_link_like 对齐）",
);

// --- 4. pack.json：知识根由用户设置驱动，不硬编码绝对路径 ---
const packPath = join(root, "packs", "knowledge", "pack.json");
const packJson = JSON.parse(readFileSync(packPath, "utf8"));
const sidecar = packJson.contributions.sidecars.find((s) => s.id === "awam-knowledge-http");
assert.ok(sidecar, "knowledge sidecar 贡献缺失");
assert.equal(
  sidecar.env.KM_DATA_ROOT,
  "${afm:settings:power.awamKnowledgeDataRoot}",
  "知识根必须由用户设置占位符驱动（换任意 Markdown 目录 = 改设置，不改代码）",
);
assert.equal(
  sidecar.env.KM_SOURCE_ID,
  "${afm:settings:power.awamKnowledgeSourceId}",
  "知识源 id 必须由用户设置占位符驱动",
);
assert.ok(
  !/G:\\|[A-Za-z]:\\\\|[A-Za-z]:\//.test(JSON.stringify(sidecar.env)),
  "sidecar env 不得嵌入任何盘符绝对路径（安装无关的通用性）",
);

// --- 5. Rust 客户端不硬编码某台机器的知识路径 ---
const rs = readFrom(root, "src-tauri/src/knowledge_service.rs");
assert.ok(
  !rs.includes(String.raw`G:\Knowledge\data`),
  "knowledge_service.rs 不得硬编码 G:\\Knowledge\\data",
);

// --- 6. 行为测试文件必须存在（删除即门控失败）---
const generalityPipeline = join(km, "tests", "generality_check.py");
const generalitySearch = join(km, "tests", "generality_search_check.py");
assert.ok(existsSync(generalityPipeline), "缺少 tests/generality_check.py（pipeline/索引侧通用性自检）");
assert.ok(existsSync(generalitySearch), "缺少 tests/generality_search_check.py（检索内核侧通用性自检）");

// --- 7. 运行两份通用性自检，要求 JSON pass:true / status 0 ---
function findPython() {
  for (const cmd of ["python", "python3"]) {
    const probe = spawnSync(cmd, ["--version"], { encoding: "utf8" });
    if (probe.status === 0) return cmd;
  }
  return null;
}

const py = findPython();
assert.ok(
  py,
  "knowledge 通用性门控需要 Python（awam-knowledge sidecar 即 Python onefile 构建依赖）",
);
for (const script of ["generality_check.py", "generality_search_check.py"]) {
  const r = spawnSync(py, [join("tests", script)], { cwd: km, encoding: "utf8" });
  if (r.stdout) process.stdout.write(r.stdout);
  if (r.stderr) process.stderr.write(r.stderr);
  assert.equal(r.status, 0, `${script} 通用性自检未通过（status=${r.status}）`);
  let out = null;
  try {
    out = JSON.parse(r.stdout.slice(r.stdout.lastIndexOf("{")));
  } catch {
    /* 解析失败时以 status 断言为准 */
  }
  if (out) assert.equal(out.pass, true, `${script} 报告 pass=false: ${JSON.stringify(out.failed)}`);
}

console.log("knowledge generality OK (any-Markdown-dir: ext+case+full-scan+blacklist-case-insensitive+config-driven root).");
