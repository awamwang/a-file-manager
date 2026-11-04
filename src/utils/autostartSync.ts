// SPDX-License-Identifier: GPL-3.0-or-later
/** Sync Settings `core.launchAtStartup` with HKCU Run (AIA-60). */

import { invoke } from "@tauri-apps/api/core";

export type AutostartStatus = {
  enabled: boolean;
  command: string | null;
  error: string | null;
};

export const AFM_AUTOSTART_CLI_FLAG = "--afm-autostart";

export async function applyLaunchAtStartupPreference(
  wantsLaunchAtStartup: boolean,
): Promise<AutostartStatus> {
  return invoke<AutostartStatus>("autostart_set_enabled", {
    enabled: wantsLaunchAtStartup,
  });
}

export async function readAutostartStatus(): Promise<AutostartStatus> {
  return invoke<AutostartStatus>("autostart_status");
}
