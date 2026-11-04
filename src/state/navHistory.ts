// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator back / forward stacks + breadcrumb helpers (AIA-63). */

import { computed, ref } from "vue";

const backStack = ref<string[]>([]);
const forwardStack = ref<string[]>([]);

/** Suppress recording while a history jump is in flight. */
let historyNav = false;

export const canGoBack = computed(() => backStack.value.length > 0);
export const canGoForward = computed(() => forwardStack.value.length > 0);

export function beginHistoryNavigation(): void {
  historyNav = true;
}

export function endHistoryNavigation(): void {
  historyNav = false;
}

export function resetNavHistory(): void {
  backStack.value = [];
  forwardStack.value = [];
}

/** Record a user-driven navigation from `from` → `to` (clears forward stack). */
export function recordNavHistory(from: string, to: string): void {
  if (historyNav) return;
  const a = from.trim();
  const b = to.trim();
  if (!a || !b || normalizeKey(a) === normalizeKey(b)) return;
  backStack.value = [...backStack.value, a];
  forwardStack.value = [];
}

/** Pop back target and push `current` onto forward. */
export function popBack(current: string): string | null {
  if (!backStack.value.length) return null;
  const target = backStack.value[backStack.value.length - 1];
  backStack.value = backStack.value.slice(0, -1);
  const here = current.trim();
  if (here) {
    forwardStack.value = [...forwardStack.value, here];
  }
  return target;
}

/** Pop forward target and push `current` onto back. */
export function popForward(current: string): string | null {
  if (!forwardStack.value.length) return null;
  const target = forwardStack.value[forwardStack.value.length - 1];
  forwardStack.value = forwardStack.value.slice(0, -1);
  const here = current.trim();
  if (here) {
    backStack.value = [...backStack.value, here];
  }
  return target;
}

function normalizeKey(path: string): string {
  return path.replace(/\//g, "\\").replace(/\\+$/, "").toLowerCase();
}

export type BreadcrumbCrumb = {
  label: string;
  path: string;
};

/** Pure path → breadcrumb crumbs for the address bar. */
export function buildBreadcrumbs(path: string): BreadcrumbCrumb[] {
  const raw = path.trim().replace(/\//g, "\\");
  if (!raw) return [];

  if (raw.startsWith("\\\\")) {
    const body = raw.slice(2);
    const parts = body.split("\\").filter(Boolean);
    if (!parts.length) {
      return [{ label: raw, path: raw }];
    }
    const crumbs: BreadcrumbCrumb[] = [];
    let acc = "\\\\" + parts[0];
    crumbs.push({ label: "\\\\" + parts[0], path: acc });
    for (let i = 1; i < parts.length; i++) {
      acc += "\\" + parts[i];
      crumbs.push({ label: parts[i], path: acc });
    }
    return crumbs;
  }

  const driveMatch = raw.match(/^([A-Za-z]:)(.*)$/);
  if (driveMatch) {
    const driveRoot = driveMatch[1] + "\\";
    const rest = driveMatch[2].replace(/^\\+/, "");
    const crumbs: BreadcrumbCrumb[] = [{ label: driveMatch[1], path: driveRoot }];
    if (!rest) return crumbs;
    let acc = driveMatch[1];
    for (const part of rest.split("\\").filter(Boolean)) {
      acc += "\\" + part;
      crumbs.push({ label: part, path: acc });
    }
    return crumbs;
  }

  const parts = raw.split("\\").filter(Boolean);
  const crumbs: BreadcrumbCrumb[] = [];
  let acc = "";
  for (const part of parts) {
    acc = acc ? `${acc}\\${part}` : part;
    crumbs.push({ label: part, path: acc });
  }
  return crumbs;
}
