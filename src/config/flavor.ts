// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Build flavor (AIA-222).
 *
 * - Default / AI package: `VITE_AFM_AI_ADDON` unset or not `"0"` → AI Add-on UI on.
 * - Core-only package: `VITE_AFM_AI_ADDON=0` (+ Cargo `--no-default-features`).
 */
export const AI_ADDON_ENABLED: boolean = import.meta.env?.VITE_AFM_AI_ADDON !== "0";
