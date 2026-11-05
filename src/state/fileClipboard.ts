// SPDX-License-Identifier: GPL-3.0-or-later
/** App-local file clipboard for Kernel copy / cut / paste (AIA-62). */

import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type ClipboardMode = "copy" | "cut";

export type FileOpsResult = {
  ok: boolean;
  paths: string[];
  error: string | null;
};

export const clipboardPaths = ref<string[]>([]);
export const clipboardMode = ref<ClipboardMode | null>(null);

export const hasClipboard = computed(
  () => clipboardPaths.value.length > 0 && clipboardMode.value !== null,
);

export function setClipboard(paths: string[], mode: ClipboardMode): void {
  clipboardPaths.value = [...paths];
  clipboardMode.value = mode;
}

export function clearClipboard(): void {
  clipboardPaths.value = [];
  clipboardMode.value = null;
}

export async function pasteClipboard(destDir: string): Promise<FileOpsResult> {
  const paths = [...clipboardPaths.value];
  const mode = clipboardMode.value;
  if (!paths.length || !mode) {
    return { ok: false, paths: [], error: "Clipboard is empty" };
  }
  const command = mode === "copy" ? "file_ops_copy" : "file_ops_move";
  const result = await invoke<FileOpsResult>(command, {
    sources: paths,
    destDir,
  });
  if (result.ok && mode === "cut") {
    clearClipboard();
  }
  return result;
}

export async function renameEntry(
  path: string,
  newName: string,
): Promise<FileOpsResult> {
  return invoke<FileOpsResult>("file_ops_rename", { path, newName });
}

export async function createFolder(
  parent: string,
  name: string,
): Promise<FileOpsResult> {
  return invoke<FileOpsResult>("file_ops_create_dir", { parent, name });
}

export async function createFile(
  parent: string,
  name: string,
): Promise<FileOpsResult> {
  return invoke<FileOpsResult>("file_ops_create_file", { parent, name });
}

export async function deleteEntries(paths: string[]): Promise<FileOpsResult> {
  return invoke<FileOpsResult>("file_ops_delete", { paths });
}

export async function pathsWithoutRecycle(paths: string[]): Promise<string[]> {
  return invoke<string[]>("file_ops_can_recycle", { paths });
}
