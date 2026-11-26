// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 平台降级：隐藏宿主上恒不可用、且点了必失败的 provider 后端卡片。
 *
 * awam-file 网关的后端注册表是跨平台的，Windows 专属后端在没有依赖的
 * macOS 上仍会出现在 `/api/backends` 里（healthy=false + detail 标注
 * "Windows only" / "*.exe not found"）。Providers 页此前把它们渲染成
 * 可点击卡片（`awam://<id>`），点进去只会得到错误。这里按平台能力
 * 统一过滤，使卡片与 Guide 根目录入口都不再暴露无效入口。
 *
 * 判定依据：Mac 上 `driveLetters=false`（Windows 恒 true）；未解析能力时
 * 保持 Windows-first 行为（不过滤），与 UI 其余平台门控一致。
 * 静态契约：scripts/check-provider-platform-filter.mjs。
 */

import { getCachedPlatformCapabilities, type PlatformCapabilities } from "../../config/platform.ts";
import type { ProviderBackendInfo } from "./listing.ts";

/**
 * 仅在 Windows 有实现的后端。每个 id 都对应网关源码中 `#[cfg(windows)]`
 * 或依赖 Windows-only 外部程序（wsl.exe / File History）的适配器。
 */
export const WINDOWS_ONLY_BACKEND_IDS: ReadonlySet<string> = new Set([
  "smb", // SMB 映射盘 / UNC 枚举依赖 Windows 网络层
  "volumes", // 盘符卷列表（driveLetters）
  "places", // Windows 快速访问 / 已知文件夹
  "wsl", // wsl.exe 集成
  "filehistory", // Windows File History 版本快照
]);

/** Mac 上是否应当隐藏 Windows-only 后端。能力未知（neutral os=""）时保持 Windows-first。 */
export function shouldHideWindowsOnlyBackends(caps?: PlatformCapabilities): boolean {
  const resolved = caps ?? getCachedPlatformCapabilities();
  return resolved.os === "macos" && resolved.driveLetters === false;
}

/**
 * 按当前平台过滤后端列表。非 Mac / 能力未解析时原样返回同一引用，
 * 避免无谓地让 Vue ref 触发重渲染。`caps` 可注入以便离线单测。
 */
export function filterUnsupportedBackends<T extends Pick<ProviderBackendInfo, "id">>(
  backends: T[],
  caps?: PlatformCapabilities,
): T[] {
  if (!shouldHideWindowsOnlyBackends(caps)) return backends;
  return backends.filter((b) => !WINDOWS_ONLY_BACKEND_IDS.has(b.id));
}
