// SPDX-License-Identifier: GPL-3.0-or-later
/** App-local favorites state (AIA-57). */

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type FavoriteItem = {
  path: string;
  addedAt: number;
  isDir: boolean;
};

export type FavoritesDocument = {
  favorites: FavoriteItem[];
};

export type FavoritesToggleResult = {
  document: FavoritesDocument;
  added: boolean;
  path: string;
};

export type FavoritesOsHookStatus = {
  registered: boolean;
  note: string;
};

export const favoritesDoc = ref<FavoritesDocument>({ favorites: [] });
export const favoritesOsHook = ref<FavoritesOsHookStatus | null>(null);

function normalize(path: string): string {
  return path.trim().replace(/\//g, "\\").toLowerCase();
}

export function isFavoritePath(path: string | null | undefined): boolean {
  if (!path) return false;
  const needle = normalize(path);
  return favoritesDoc.value.favorites.some(
    (item) => normalize(item.path) === needle,
  );
}

export async function loadFavorites(): Promise<FavoritesDocument> {
  const doc = await invoke<FavoritesDocument>("favorites_get");
  favoritesDoc.value = doc;
  return doc;
}

export async function toggleFavorite(
  path: string,
  isDir: boolean,
): Promise<FavoritesToggleResult> {
  const result = await invoke<FavoritesToggleResult>("favorites_toggle", {
    path,
    isDir,
  });
  favoritesDoc.value = result.document;
  return result;
}

export async function loadFavoritesOsHookStatus(): Promise<FavoritesOsHookStatus> {
  const status = await invoke<FavoritesOsHookStatus>("favorites_os_hook_status");
  favoritesOsHook.value = status;
  return status;
}
