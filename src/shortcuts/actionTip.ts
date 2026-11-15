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
  el.addEventListener("mouseenter", enter);
  el.addEventListener("mouseleave", leave);
  el.addEventListener("focus", enter);
  el.addEventListener("blur", leave);
  cleanups.set(el, () => {
    el.removeEventListener("mouseenter", enter);
    el.removeEventListener("mouseleave", leave);
    el.removeEventListener("focus", enter);
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
