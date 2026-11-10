// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator back / forward stacks + breadcrumb helpers (AIA-63). */

import { computed, ref } from "vue";

/** Keep in sync with `guide/path.ts` (avoid ESM extension issues in Node checks). */
const GUIDE_PATH = "afm://guide";
/** Keep in sync with `provider/path.ts`. */
const PROVIDER_ROOT = "awam://root";

function isGuidePath(path: string): boolean {
  return (
    path.trim().replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase() ===
    GUIDE_PATH
  );
}

function isProviderPath(path: string): boolean {
  return path.trim().toLowerCase().startsWith("awam://");
}

function toProviderUri(path: string): string {
  const trimmed = path.trim();
  if (!trimmed) return "";
  const key = trimmed.replace(/\/+$/, "").toLowerCase();
  if (!key || key === "awam:" || key === "awam://" || key === "awam://root") {
    return PROVIDER_ROOT;
  }
  return trimmed.replace(/\/+$/, "") || PROVIDER_ROOT;
}

function providerParentPath(path: string): string | null {
  const uri = toProviderUri(path);
  if (!uri || !isProviderPath(uri)) return null;
  if (uri.toLowerCase() === PROVIDER_ROOT) return null;
  const rest = uri.slice("awam://".length);
  const slash = rest.lastIndexOf("/");
  if (slash < 0) return PROVIDER_ROOT;
  const parentRest = rest.slice(0, slash);
  if (!parentRest) return PROVIDER_ROOT;
  return `awam://${parentRest}`;
}

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
  const trimmed = path.trim();
  // URI 方案（afm:// / awam:// / http://…）保留 `/`，避免破坏权威路径。
  if (/^[a-z][a-z0-9+.-]*:/i.test(trimmed)) {
    return trimmed.replace(/\/+$/, "").toLowerCase();
  }
  return trimmed.replace(/\//g, "\\").replace(/\\+$/, "").toLowerCase();
}

export type BreadcrumbCrumb = {
  label: string;
  path: string;
  /** Windows-style root / “This PC” segment — menu lists Home, drives, Recent, Favorites. */
  kind?: "root" | "path";
};

/** Pure path → breadcrumb crumbs for the address bar (no root segment). */
export function buildBreadcrumbs(path: string): BreadcrumbCrumb[] {
  const trimmed = path.trim();
  if (!trimmed) return [];
  if (isGuidePath(trimmed) || normalizeGuideLike(trimmed)) {
    return [];
  }
  if (isProviderPath(trimmed)) {
    return buildProviderBreadcrumbs(trimmed);
  }

  const raw = trimmed.replace(/\//g, "\\");

  if (raw.startsWith("\\\\")) {
    const body = raw.slice(2);
    const parts = body.split("\\").filter(Boolean);
    if (!parts.length) {
      return [{ label: raw, path: raw, kind: "path" }];
    }
    const crumbs: BreadcrumbCrumb[] = [];
    let acc = "\\\\" + parts[0];
    crumbs.push({ label: "\\\\" + parts[0], path: acc, kind: "path" });
    for (let i = 1; i < parts.length; i++) {
      acc += "\\" + parts[i];
      crumbs.push({ label: parts[i], path: acc, kind: "path" });
    }
    return crumbs;
  }

  const driveMatch = raw.match(/^([A-Za-z]:)(.*)$/);
  if (driveMatch) {
    const driveRoot = driveMatch[1] + "\\";
    const rest = driveMatch[2].replace(/^\\+/, "");
    const crumbs: BreadcrumbCrumb[] = [
      { label: driveMatch[1], path: driveRoot, kind: "path" },
    ];
    if (!rest) return crumbs;
    let acc = driveMatch[1];
    for (const part of rest.split("\\").filter(Boolean)) {
      acc += "\\" + part;
      crumbs.push({ label: part, path: acc, kind: "path" });
    }
    return crumbs;
  }

  const parts = raw.split("\\").filter(Boolean);
  const crumbs: BreadcrumbCrumb[] = [];
  let acc = "";
  for (const part of parts) {
    acc = acc ? `${acc}\\${part}` : part;
    crumbs.push({ label: part, path: acc, kind: "path" });
  }
  return crumbs;
}

function normalizeGuideLike(path: string): boolean {
  return path.trim().toLowerCase().startsWith("afm://");
}

/** `awam://rclone/a/b` → Providers 根 + 各级 backend/path 段。 */
export function buildProviderBreadcrumbs(
  path: string,
  providersLabel = "Providers",
): BreadcrumbCrumb[] {
  const uri = toProviderUri(path);
  if (!uri) return [];
  const crumbs: BreadcrumbCrumb[] = [
    { label: providersLabel, path: PROVIDER_ROOT, kind: "path" },
  ];
  if (uri.toLowerCase() === PROVIDER_ROOT) {
    return crumbs;
  }
  const rest = uri.slice("awam://".length);
  const parts = rest.split("/").filter(Boolean);
  let acc = "awam://";
  for (let i = 0; i < parts.length; i++) {
    const part = parts[i]!;
    acc = i === 0 ? `awam://${part}` : `${acc}/${part}`;
    crumbs.push({ label: part, path: acc, kind: "path" });
  }
  return crumbs;
}

/**
 * Windows-style breadcrumbs with a leading root segment.
 * Root path is the guide landing (`afm://guide`); its chevron opens Home/drives/Recent/Favorites.
 */
export function buildNavigatorBreadcrumbs(
  path: string,
  rootLabel: string,
  providersLabel = "Providers",
): BreadcrumbCrumb[] {
  const root: BreadcrumbCrumb = {
    label: rootLabel,
    path: GUIDE_PATH,
    kind: "root",
  };
  if (!path.trim() || isGuidePath(path)) {
    return [root];
  }
  if (isProviderPath(path)) {
    return [root, ...buildProviderBreadcrumbs(path, providersLabel)];
  }
  if (normalizeGuideLike(path)) {
    const leaf = path.trim().replace(/^afm:\/\//i, "");
    return [
      root,
      {
        label: leaf || path,
        path: path.trim(),
        kind: "path",
      },
    ];
  }
  return [root, ...buildBreadcrumbs(path)];
}

/**
 * Up / parent target aligned with breadcrumb hierarchy.
 * Drive roots (`D:\`) and UNC share roots (`\\server\share`) → guide (`afm://guide`).
 * Provider backend 根 → `awam://root`；Providers 根 → guide。
 * Guide / empty → `null` (nowhere further).
 */
export function navigableParentPath(path: string): string | null {
  const trimmed = path.trim();
  if (!trimmed || isGuidePath(trimmed)) {
    return null;
  }
  if (isProviderPath(trimmed)) {
    return providerParentPath(trimmed) ?? GUIDE_PATH;
  }
  if (normalizeGuideLike(trimmed)) {
    return null;
  }

  const raw = trimmed.replace(/\//g, "\\");

  // Windows drive root: `D:`, `D:\`, `D:/`
  if (/^[A-Za-z]:\\?$/.test(raw)) {
    return GUIDE_PATH;
  }

  // UNC host or share root: `\\server` / `\\server\share` (Kernel parent is None).
  if (raw.startsWith("\\\\")) {
    const parts = raw.slice(2).split("\\").filter(Boolean);
    if (parts.length <= 2) {
      return GUIDE_PATH;
    }
  }

  const crumbs = buildBreadcrumbs(trimmed);
  if (crumbs.length <= 1) {
    // Single-segment roots still climb to the guide landing.
    return GUIDE_PATH;
  }
  return crumbs[crumbs.length - 2]!.path;
}
