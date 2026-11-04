// SPDX-License-Identifier: GPL-3.0-or-later

import { ref } from "vue";

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

/** Match Ctrl+Shift+P (Windows-first). Ignore when composing IME text. */
export function isToggleCommandPanelShortcut(event: KeyboardEvent): boolean {
  if (event.isComposing) return false;
  if (!event.ctrlKey || !event.shiftKey || event.altKey || event.metaKey) return false;
  return event.key === "P" || event.key === "p";
}
