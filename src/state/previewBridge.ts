// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AIA Preview Bridge (frontend): Space → selectedPaths → Peek CLI / QL pipe /
 * built-in preview pane. One AFM bridge + thin protocol backends — not
 * zero-adapter for every Quick Look–class tool.
 */

import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";

export type PreviewBridgeResult = {
  success: boolean;
  backend: "peek" | "quicklook" | "builtin" | "off" | "none" | string;
  error?: string | null;
  fallbackBuiltin: boolean;
};

/** Session force-show for experimental PreviewPane when bridge falls back. */
export const previewBridgeForcePane = ref(false);

export async function invokePreviewBridge(
  paths: string[],
): Promise<PreviewBridgeResult> {
  if (!paths.length) {
    return {
      success: false,
      backend: "none",
      error: "No selection",
      fallbackBuiltin: false,
    };
  }
  const result = await invoke<PreviewBridgeResult>("preview_bridge_invoke", {
    paths,
  });
  if (result.fallbackBuiltin) {
    previewBridgeForcePane.value = true;
  }
  return result;
}
