// SPDX-License-Identifier: GPL-3.0-or-later

import { ref } from "vue";
import { BUILTIN_SHORTCUTS } from "../shortcuts/registry.ts";
import { eventMatchesChord } from "../shortcuts/chord.ts";

/** Shared open state for the Command Panel overlay. */
export const commandPanelOpen = ref(false);

export function openCommandPanel(): void {
  commandPanelOpen.value = true;
}

export function closeCommandPanel(): void {
  commandPanelOpen.value = false;
}

export function toggleCommandPanel(): void {
  commandPanelOpen.value = !commandPanelOpen.value;
}

const panelChord = BUILTIN_SHORTCUTS.find((s) => s.id === "commandPanel.toggle")!.chord;

/** Match Ctrl+Shift+P via central shortcut registry (Windows-first). */
export function isToggleCommandPanelShortcut(event: KeyboardEvent): boolean {
  return eventMatchesChord(event, panelChord);
}
