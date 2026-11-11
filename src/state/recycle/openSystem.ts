// SPDX-License-Identifier: GPL-3.0-or-later
/** 用系统资源管理器打开回收站（快路径，避免 AFM 内整仓枚举）。 */

import { invoke } from "@tauri-apps/api/core";

export const RECYCLE_SHELL_TARGET = "shell:RecycleBinFolder";

export type OpenRecycleBinResult = {
  success: boolean;
  error: string | null;
};

/** 通过 explorer / shell:RecycleBinFolder 打开系统回收站。 */
export async function openRecycleBinInSystem(): Promise<OpenRecycleBinResult> {
  return invoke<OpenRecycleBinResult>("open_recycle_bin_system");
}
