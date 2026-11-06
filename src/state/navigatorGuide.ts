// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator guide page path + places APIs (drives / user folders) — AIA-148. */

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { GUIDE_PATH, isGuidePath, normalizeGuideKey } from "./guidePath";

export { GUIDE_PATH, isGuidePath, normalizeGuideKey };

export type DriveInfo = {
  letter: string;
  path: string;
  label: string;
};

export type UserFolders = {
  home: string;
  desktop: string | null;
  documents: string | null;
  downloads: string | null;
  pictures: string | null;
  music: string | null;
  videos: string | null;
  onedrive?: string | null;
};

export type LibraryPlace = {
  id: string;
  path: string;
};

export type NetworkPlace = {
  name: string;
  path: string;
};

export type ShellPlaces = {
  onedrive: string | null;
  libraries: LibraryPlace[];
  network: NetworkPlace[];
};

export type UserFolderId =
  | "home"
  | "desktop"
  | "documents"
  | "downloads"
  | "pictures"
  | "music"
  | "videos";

export type UserFolderItem = {
  id: UserFolderId;
  path: string;
};

export const drivesList = ref<DriveInfo[]>([]);
export const userFolders = ref<UserFolders | null>(null);
export const shellPlaces = ref<ShellPlaces | null>(null);

export function guideTabName(translate: (key: string) => string): string {
  return translate("guide.tabName");
}

export async function loadDrives(): Promise<DriveInfo[]> {
  const list = await invoke<DriveInfo[]>("navigator_list_drives");
  drivesList.value = list;
  return list;
}

export async function loadUserFolders(): Promise<UserFolders> {
  const folders = await invoke<UserFolders>("navigator_user_folders");
  userFolders.value = folders;
  return folders;
}

export async function loadShellPlaces(): Promise<ShellPlaces> {
  const places = await invoke<ShellPlaces>("navigator_shell_places");
  shellPlaces.value = places;
  return places;
}

export async function loadNavigatorPlaces(): Promise<void> {
  await Promise.all([
    loadDrives().catch(() => {
      drivesList.value = [];
    }),
    loadUserFolders().catch(() => {
      userFolders.value = null;
    }),
    loadShellPlaces().catch(() => {
      shellPlaces.value = null;
    }),
  ]);
}

/** Ordered common Windows user dirs that currently exist. */
export function userFolderItems(folders: UserFolders | null): UserFolderItem[] {
  if (!folders) return [];
  const order: Array<[UserFolderId, string | null | undefined]> = [
    ["home", folders.home],
    ["desktop", folders.desktop],
    ["documents", folders.documents],
    ["downloads", folders.downloads],
    ["pictures", folders.pictures],
    ["music", folders.music],
    ["videos", folders.videos],
  ];
  const out: UserFolderItem[] = [];
  for (const [id, path] of order) {
    const trimmed = (path ?? "").trim();
    if (trimmed) out.push({ id, path: trimmed });
  }
  return out;
}

export async function resolveHomePath(): Promise<string> {
  try {
    const { hydrateSettings } = await import("./settingsHydrate");
    const doc = await hydrateSettings();
    const configured = String(doc.values?.["core.defaultLocation"] ?? "").trim();
    if (configured) return configured;
  } catch {
    /* fall through */
  }
  try {
    return await invoke<string>("navigator_default_path");
  } catch {
    return "C:\\";
  }
}
