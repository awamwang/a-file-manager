// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Floating shortcut hint (v-action-tip).
 *
 * The directive only stores *what* to show; the host resolves chords at render
 * time from the central registry, so a hovered button always shows the FINAL
 * (post-override) shortcuts — never a hardcoded builtin label.
 */

import { ref, type Directive } from "vue";

export type ActionTipBinding = {
  label: string;
  actionId: string;
} | null;

export type ActionTipState = {
  label: string;
  actionId: string;
  left: number;
  top: number;
  below: boolean;
  owner: HTMLElement;
};

export const activeActionTip = ref<ActionTipState | null>(null);

/**
 * On macOS the reveal path (`afm_reveal_main_window` → `set_focus()`) can hand
 * DOM focus to the first focusable control — the Command Panel button — with
 * no user input at all. Because `v-action-tip` treats focus as a show trigger,
 * that popped the floating hint on the very first frame. Keep the policy as a
 * pure factory so the cold-start case can be baseline-tested headlessly.
 */
export type ActionTipTrigger = "hover" | "focus";

export function createActionTipInteractionGate() {
  let interacted = false;
  return {
    markInteraction(): void {
      interacted = true;
    },
    allows(trigger: ActionTipTrigger): boolean {
      // Hover is already a direct pointer signal; only focus needs the guard.
      return trigger === "hover" || interacted;
    },
  };
}

const interactionGate = createActionTipInteractionGate();

/** Called by ActionTipHost on real user input (pointerdown / keydown). */
export function markActionTipInteraction(): void {
  interactionGate.markInteraction();
}

const bindings = new WeakMap<HTMLElement, ActionTipBinding>();
const cleanups = new WeakMap<HTMLElement, () => void>();

export function showActionTip(el: HTMLElement): void {
  const value = bindings.get(el);
  if (!value || !value.actionId) {
    hideActionTip(el);
    return;
  }
  const rect = el.getBoundingClientRect();
  const viewportWidth = typeof window === "undefined" ? 1280 : window.innerWidth;
  const half = 120;
  const below = rect.top < 64;
  activeActionTip.value = {
    label: value.label,
    actionId: value.actionId,
    left: Math.min(
      Math.max(rect.left + rect.width / 2, half),
      Math.max(half, viewportWidth - half),
    ),
    top: below ? rect.bottom + 8 : rect.top - 8,
    below,
    owner: el,
  };
}

/** With no argument (or a matching owner) the floating card is dismissed. */
export function hideActionTip(el?: HTMLElement | null): void {
  if (!el || activeActionTip.value?.owner === el) {
    activeActionTip.value = null;
  }
}

function attach(el: HTMLElement): void {
  if (cleanups.has(el)) return;
  const enter = () => showActionTip(el);
  const leave = () => hideActionTip(el);
  const focus = () => {
    if (interactionGate.allows("focus")) showActionTip(el);
  };
  el.addEventListener("mouseenter", enter);
  el.addEventListener("mouseleave", leave);
  el.addEventListener("focus", focus);
  el.addEventListener("blur", leave);
  cleanups.set(el, () => {
    el.removeEventListener("mouseenter", enter);
    el.removeEventListener("mouseleave", leave);
    el.removeEventListener("focus", focus);
    el.removeEventListener("blur", leave);
  });
}

export const vActionTip: Directive<HTMLElement, ActionTipBinding> = {
  mounted(el, binding) {
    bindings.set(el, binding.value);
    attach(el);
  },
  updated(el, binding) {
    bindings.set(el, binding.value);
    // Locale changes / label updates must refresh an open card immediately.
    if (activeActionTip.value?.owner === el) showActionTip(el);
  },
  unmounted(el) {
    cleanups.get(el)?.();
    cleanups.delete(el);
    hideActionTip(el);
    bindings.delete(el);
  },
};
