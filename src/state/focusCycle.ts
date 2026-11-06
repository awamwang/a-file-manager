// SPDX-License-Identifier: GPL-3.0-or-later
/** F6 focus cycle across Explorer-like chrome regions. */

export const FOCUS_CYCLE_REGIONS = [
  "tabstrip",
  "sidenav",
  "content",
  "statusbar",
] as const;

export type FocusCycleRegion = (typeof FOCUS_CYCLE_REGIONS)[number];

export const FOCUS_CYCLE_ATTR = "data-focus-region";

export function nextFocusRegion(
  current: FocusCycleRegion | null,
  reverse = false,
): FocusCycleRegion {
  const list = FOCUS_CYCLE_REGIONS;
  if (!current) return reverse ? list[list.length - 1]! : list[0]!;
  const idx = list.indexOf(current);
  if (idx < 0) return reverse ? list[list.length - 1]! : list[0]!;
  if (reverse) {
    return list[(idx - 1 + list.length) % list.length]!;
  }
  return list[(idx + 1) % list.length]!;
}

export function regionFromElement(el: Element | null): FocusCycleRegion | null {
  if (!el) return null;
  const host = el.closest(`[${FOCUS_CYCLE_ATTR}]`);
  if (!(host instanceof HTMLElement)) return null;
  const value = host.getAttribute(FOCUS_CYCLE_ATTR);
  if (!value) return null;
  return (FOCUS_CYCLE_REGIONS as readonly string[]).includes(value)
    ? (value as FocusCycleRegion)
    : null;
}

/** Prefer a focusable child; otherwise focus the region host itself. */
export function focusRegionElement(host: HTMLElement): void {
  const preferred = host.querySelector<HTMLElement>(
    "[data-focus-default], button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])",
  );
  const target = preferred ?? host;
  if (!host.hasAttribute("tabindex") && target === host) {
    host.tabIndex = -1;
  }
  target.focus({ preventScroll: true });
}
