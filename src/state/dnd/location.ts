// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 同位置投放判定（Explorer 规则）：源已经位于目标目录时，拖动是无效操作——
 * 不显示投放提示、不高亮目标、不切换标签、不发起任何文件操作。
 * 纯函数，供 dragover / drop 与检查脚本共用。
 */

/** 规范化 Windows 路径：统一分隔符、折叠 . / ..、去尾部分隔符、忽略大小写。 */
export function normalizeDropPath(path: string): string {
  const raw = path.trim().replace(/\//g, "\\");
  const unc = raw.startsWith("\\\\");
  const body = unc ? raw.slice(2) : raw;
  // UNC 保留 server + share；盘符路径保留 "C:" 作为根，.. 不允许越过它。
  const rootDepth = unc ? 2 : 1;
  const parts: string[] = [];
  for (const part of body.split("\\")) {
    if (!part || part === ".") continue;
    if (part === "..") {
      if (parts.length > rootDepth) parts.pop();
      continue;
    }
    parts.push(part);
  }
  return `${unc ? "\\\\" : ""}${parts.join("\\")}`.toLowerCase();
}

/** 源的父目录；盘符根下的文件返回 "c:"。 */
export function parentDropDir(path: string): string {
  const normalized = normalizeDropPath(path);
  const idx = normalized.lastIndexOf("\\");
  return idx >= 0 ? normalized.slice(0, idx) : "";
}

/** 源是否已经在目标目录里（或源本身就是目标目录）。 */
export function isSourceInDest(source: string, dest: string): boolean {
  const s = normalizeDropPath(source);
  const d = normalizeDropPath(dest);
  if (!s || !d) return false;
  return s === d || parentDropDir(s) === d;
}

/** 过滤掉已在目标目录内的源，剩下才是真正需要搬动的路径。 */
export function effectiveDropSources(sources: string[], dest: string): string[] {
  return sources.filter((source) => !isSourceInDest(source, dest));
}
