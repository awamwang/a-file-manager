// SPDX-License-Identifier: GPL-3.0-or-later
/** Windows shell 右键菜单适配器：原生 TrackPopupMenu（FIL-53 / AIA-229）。 */

import { invoke } from "@tauri-apps/api/core";

export type ShellMenuResult = {
  success: boolean;
  cancelled: boolean;
  error: string | null;
};

/** 在屏幕坐标弹出系统原生右键菜单；失败返回错误文案，取消返回 null。 */
export async function showNativeShellContextMenu(
  path: string,
  x: number,
  y: number,
): Promise<string | null> {
  try {
    const result = await invoke<ShellMenuResult>("shell_show_context_menu", {
      path,
      x,
      y,
    });
    if (!result.success && result.error) return result.error;
    return null;
  } catch (err) {
    return err instanceof Error ? err.message : String(err);
  }
}
