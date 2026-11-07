// SPDX-License-Identifier: GPL-3.0-or-later
/** AIA-231: persisted collapse state for navigator folder-tree sections. */

import { ref } from "vue";

export type FolderTreeSectionId =
  | "folders"
  | "drives"
  | "recycle"
  | "onedrive"
  | "libraries"
  | "network";

export const FOLDER_TREE_SECTION_IDS: FolderTreeSectionId[] = [
  "folders",
  "drives",
  "recycle",
  "onedrive",
  "libraries",
  "network",
];

const STORAGE_KEY = "afm.folderTree.collapsedSections";

/** Section ids that are collapsed. Missing id ⇒ expanded (default). */
export const collapsedFolderTreeSections = ref<Set<FolderTreeSectionId>>(new Set());

function isSectionId(value: string): value is FolderTreeSectionId {
  return (FOLDER_TREE_SECTION_IDS as string[]).includes(value);
}

export function readCollapsedFolderTreeSections(): Set<FolderTreeSectionId> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return new Set();
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return new Set();
    return new Set(parsed.filter((id): id is FolderTreeSectionId => typeof id === "string" && isSectionId(id)));
  } catch {
    return new Set();
  }
}

export function writeCollapsedFolderTreeSections(ids: Iterable<FolderTreeSectionId>): void {
  try {
    const list = [...ids].filter(isSectionId);
    if (list.length === 0) localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, JSON.stringify(list));
  } catch {
    /* private mode / quota */
  }
}

export function loadFolderTreeSectionCollapse(): void {
  collapsedFolderTreeSections.value = readCollapsedFolderTreeSections();
}

export function isFolderTreeSectionExpanded(id: FolderTreeSectionId): boolean {
  return !collapsedFolderTreeSections.value.has(id);
}

export function setFolderTreeSectionExpanded(id: FolderTreeSectionId, expanded: boolean): void {
  const next = new Set(collapsedFolderTreeSections.value);
  if (expanded) next.delete(id);
  else next.add(id);
  collapsedFolderTreeSections.value = next;
  writeCollapsedFolderTreeSections(next);
}

export function toggleFolderTreeSection(id: FolderTreeSectionId): void {
  setFolderTreeSectionExpanded(id, !isFolderTreeSectionExpanded(id));
}
