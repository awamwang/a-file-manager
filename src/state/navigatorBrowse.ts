// SPDX-License-Identifier: GPL-3.0-or-later

import { ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type NavigatorEntry = {
  name: string;
  path: string;
  kind: "dir" | "file" | string;
  size: number;
};

export type NavigatorListing = {
  path: string;
  entries: NavigatorEntry[];
  dirCount: number;
  fileCount: number;
};

export const currentPath = ref("");
export const listing = shallowRef<NavigatorListing | null>(null);
export const listError = ref<string | null>(null);
export const listLoading = ref(false);

/** Lightweight Navigator tab paths (Open all Project links → tabs only). */
export const navigatorTabs = ref<string[]>([]);

export async function navigateTo(path: string): Promise<void> {
  const trimmed = path.trim();
  listLoading.value = true;
  listError.value = null;
  try {
    const result = await invoke<NavigatorListing>("navigator_list", { path: trimmed });
    listing.value = result;
    currentPath.value = result.path;
  } catch (err) {
    listing.value = null;
    listError.value = err instanceof Error ? err.message : String(err);
    currentPath.value = trimmed;
  } finally {
    listLoading.value = false;
  }
}

export async function openPathsAsNavigatorTabs(paths: string[]): Promise<void> {
  const cleaned = paths.map((p) => p.trim()).filter(Boolean);
  navigatorTabs.value = cleaned;
  if (cleaned.length > 0) {
    await navigateTo(cleaned[0]);
  }
}

export async function loadDefaultPath(): Promise<void> {
  listLoading.value = true;
  listError.value = null;
  try {
    const path = await invoke<string>("navigator_default_path");
    await navigateTo(path);
  } catch (err) {
    listLoading.value = false;
    listError.value = err instanceof Error ? err.message : String(err);
  }
}

export async function goUp(): Promise<void> {
  if (!currentPath.value) return;
  const parent = await invoke<string | null>("navigator_parent", {
    path: currentPath.value,
  });
  if (parent) {
    await navigateTo(parent);
  }
}

export async function openEntry(entry: NavigatorEntry): Promise<void> {
  if (entry.kind === "dir") {
    await navigateTo(entry.path);
  }
}
