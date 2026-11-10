// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 路径键局部性：Windows 原生 `\` 写入/比较；URI 方案（afm://…）不改分隔符。
 * Open State / browse / folder tree 等共用此模块，避免多处浅层 normalize。
 */

/** 规范为 OS 原生路径（Windows=`\`）；虚拟 URI 保持方案分隔符。 */
export function toNativeFsPath(path: string): string {
  const trimmed = path.trim();
  if (!trimmed) return "";
  if (
    /^[a-zA-Z]:[\\/]/.test(trimmed) ||
    trimmed.startsWith("\\\\") ||
    trimmed.startsWith("//")
  ) {
    let native = trimmed.replace(/\//g, "\\");
    if (/^[a-zA-Z]:\\$/i.test(native)) return native;
    return native.replace(/\\+$/, "");
  }
  // 含 `://` 的方案（afm://guide、http://…）不改分隔符
  if (/^[a-z][a-z0-9+.-]*:/i.test(trimmed)) {
    return trimmed.replace(/\/+$/, "");
  }
  return trimmed.replace(/\//g, "\\").replace(/\\+$/, "");
}

/**
 * 相等比较键：本地路径小写；`awam://` 仅折叠 scheme/backend，**保留 path 大小写**
 *（browser `Profile 1` 等对大小写敏感）。
 */
export function normalizePathKey(path: string): string {
  const native = toNativeFsPath(path);
  const lower = native.toLowerCase();
  if (!lower.startsWith("awam://")) {
    return lower;
  }
  const body = native.slice("awam://".length).replace(/\/+$/, "");
  if (!body) return "awam://root";
  const slash = body.indexOf("/");
  if (slash < 0) {
    const backend = body.toLowerCase();
    return !backend || backend === "root" ? "awam://root" : `awam://${backend}`;
  }
  const backend = body.slice(0, slash).toLowerCase() || "root";
  const rest = body.slice(slash + 1).replace(/\/+$/, "");
  if (!rest) {
    return !backend || backend === "root" ? "awam://root" : `awam://${backend}`;
  }
  return `awam://${backend}/${rest}`;
}

/** browse / tree 等会话面共用的路径相等判断。 */
export function pathsEqual(a: string, b: string): boolean {
  const left = normalizePathKey(a);
  const right = normalizePathKey(b);
  if (!left || !right) return false;
  return left === right;
}

/**
 * 文件夹树比较键：在 normalizePathKey 之上去掉盘符根的尾 `\`，
 * 使 `C:\` 与 `C:` 同键，且 `C:\Temp`.startsWith(`C:` + `\`) 仍成立。
 */
export function normalizeTreePathKey(path: string): string {
  const key = normalizePathKey(path);
  if (/^[a-z]:\\$/i.test(key)) return key.slice(0, 2);
  return key;
}
