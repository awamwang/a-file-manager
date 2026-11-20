// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Build flavor (AIA-222).
 *
 * - Mainline (ai flavor): `VITE_AFM_AI_ADDON` unset or not `"0"` → AI Add-on UI on.
 * - Lite (core flavor): `VITE_AFM_AI_ADDON=0` (+ Cargo `--no-default-features`). See ADR 0024.
 */
export const AI_ADDON_ENABLED: boolean = import.meta.env?.VITE_AFM_AI_ADDON !== "0";
