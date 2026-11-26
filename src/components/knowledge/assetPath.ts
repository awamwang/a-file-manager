// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 知识笔记里的相对图片路径解析。
 *
 * 知识弹窗是应用内页面：笔记里的 `![](../assets/x.jpg)` 会按应用 origin 解析，
 * 取不到知识库里的文件 → 裂图。这里把相对路径解析回「知识根下的绝对路径」，
 * 解析结果必须仍在知识根内（越界即拒），供 preview_load 取回 data: URL 内联。
 */

/** 带协议的引用（http:/data:/file:/tauri: …）不是知识根内相对路径，一律不放行。 */
const SCHEME_RX = /^[a-z][a-z0-9+.-]*:/i;

/**
 * 把笔记里的相对图片路径解析成知识根下的绝对路径。
 *
 * @param root 知识根绝对路径（服务端 /doc 返回的 root）
 * @param docRelPath 笔记相对路径（相对知识根，正斜杠）
 * @param src 笔记里的图片 src（相对路径）
 * @returns 知识根内的绝对路径；越界 / 非相对路径返回 null
 */
export function resolveKnowledgeAssetPath(
  root: string,
  docRelPath: string,
  src: string,
): string | null {
  if (!root || !src) return null;
  if (SCHEME_RX.test(src)) return null;
  if (src.startsWith("/") || src.startsWith("\\")) return null;

  const sep = root.includes("\\") ? "\\" : "/";
  const rootParts = root.replace(/\\/g, "/").split("/").filter(Boolean);
  const dirParts = docRelPath.replace(/\\/g, "/").split("/").slice(0, -1);
  const parts = [...rootParts, ...dirParts];
  for (const seg of src.replace(/\\/g, "/").split("/")) {
    if (seg === "" || seg === ".") continue;
    if (seg === "..") {
      // 弹出知识根 = 越界，直接拒（不解析出根外的路径）
      if (parts.length <= rootParts.length) return null;
      parts.pop();
      continue;
    }
    parts.push(seg);
  }
  // 落到知识根本身（不是文件）→ 无效
  if (parts.length <= rootParts.length) return null;
  return root + sep + parts.slice(rootParts.length).join(sep);
}
