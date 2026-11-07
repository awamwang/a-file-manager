// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AIA Preview Bridge (frontend): Space → selectedPaths → QuickLook pipe /
 * Peek CLI / built-in preview pane (`auto` prefers QL; AIA-254). One AFM
 * bridge + thin protocol backends — not zero-adapter for every QL-class tool.
 *
 * AIA-174: while a Space-opened session is active, selection changes call
 * `preview_bridge_switch` (QL Switch / Peek PID replace / builtin no-op).
 *
 * AIA-244: pane × (dismiss) clears the session sticky — selection alone must
 * not re-open until Space / preview command opens again.
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

/**
 * Bumped on dismiss so in-flight invoke/switch results cannot resurrect the
 * session after the user closed the pane with ×.
 */
let sessionEpoch = 0;

function applySession(result: PreviewBridgeResult, epoch: number): void {
  if (epoch !== sessionEpoch) return;
  const active = result.sessionActive === true;
  previewBridgeSessionActive.value = active;
  if (result.fallbackBuiltin && active) {
    previewBridgeForcePane.value = true;
  } else if (!active && result.backend === "builtin") {
    // Space / dismiss ended a bridge-forced built-in session.
    previewBridgeForcePane.value = false;
  } else if (!active) {
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
  const epoch = sessionEpoch;
  const result = await invoke<PreviewBridgeResult>("preview_bridge_invoke", {
    paths,
  });
  applySession(result, epoch);
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
  const epoch = sessionEpoch;
  const result = await invoke<PreviewBridgeResult>("preview_bridge_switch", {
    paths,
  });
  applySession(result, epoch);
  return result;
}

/**
 * AIA-244: user closed preview with ×. Clear local sticky flags immediately,
 * invalidate in-flight switch/invoke, then ask Rust to drop the Space session.
 */
export async function dismissPreviewBridge(): Promise<PreviewBridgeResult> {
  sessionEpoch += 1;
  const epoch = sessionEpoch;
  previewBridgeSessionActive.value = false;
  previewBridgeForcePane.value = false;
  try {
    const result = await invoke<PreviewBridgeResult>("preview_bridge_dismiss");
    applySession(
      {
        ...result,
        sessionActive: false,
        fallbackBuiltin: false,
      },
      epoch,
    );
    return {
      ...result,
      sessionActive: false,
      fallbackBuiltin: false,
    };
  } catch {
    return {
      success: true,
      backend: "none",
      fallbackBuiltin: false,
      sessionActive: false,
    };
  }
}
