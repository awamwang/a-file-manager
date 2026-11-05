// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AIA Preview Bridge (frontend): Space → selectedPaths → Peek CLI / QL pipe /
 * built-in preview pane. One AFM bridge + thin protocol backends — not
 * zero-adapter for every Quick Look–class tool.
 *
 * AIA-174: while a Space-opened session is active, selection changes call
 * `preview_bridge_switch` (QL Switch / Peek PID replace / builtin no-op).
 */

import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";

export type PreviewBridgeResult = {
  success: boolean;
  backend: "peek" | "quicklook" | "builtin" | "off" | "none" | string;
  error?: string | null;
  fallbackBuiltin: boolean;
  sessionActive?: boolean;
};

/** Session force-show for experimental PreviewPane when bridge falls back. */
export const previewBridgeForcePane = ref(false);

/** True after Space opened Peek/QL/builtin until session ends. */
export const previewBridgeSessionActive = ref(false);

function applySession(result: PreviewBridgeResult): void {
  const active = result.sessionActive === true;
  previewBridgeSessionActive.value = active;
  if (result.fallbackBuiltin && active) {
    previewBridgeForcePane.value = true;
  } else if (!active && result.backend === "builtin") {
    // Space dismissed a bridge-forced built-in session.
    previewBridgeForcePane.value = false;
  }
}

export async function invokePreviewBridge(
  paths: string[],
): Promise<PreviewBridgeResult> {
  if (!paths.length) {
    return {
      success: false,
      backend: "none",
      error: "No selection",
      fallbackBuiltin: false,
      sessionActive: false,
    };
  }
  const result = await invoke<PreviewBridgeResult>("preview_bridge_invoke", {
    paths,
  });
  applySession(result);
  return result;
}

/** Push a new path to an already-open Space preview session. */
export async function switchPreviewBridge(
  paths: string[],
): Promise<PreviewBridgeResult> {
  if (!previewBridgeSessionActive.value) {
    return {
      success: false,
      backend: "none",
      error: "No active session",
      fallbackBuiltin: false,
      sessionActive: false,
    };
  }
  if (!paths.length) {
    return {
      success: false,
      backend: "none",
      error: "No selection",
      fallbackBuiltin: false,
      sessionActive: previewBridgeSessionActive.value,
    };
  }
  const result = await invoke<PreviewBridgeResult>("preview_bridge_switch", {
    paths,
  });
  applySession(result);
  return result;
}
