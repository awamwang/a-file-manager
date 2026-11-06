// SPDX-License-Identifier: GPL-3.0-or-later
/** Shell file clipboard (CF_HDROP) so copy/cut/paste works with File Explorer. */

import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { resolveNameConflictChoice } from "./nameConflict";

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
  void invoke("file_ops_clipboard_set", { paths, mode }).catch(() => undefined);
}

export function clearClipboard(): void {
  clipboardPaths.value = [];
  clipboardMode.value = null;
  void invoke("file_ops_clipboard_clear").catch(() => undefined);
}

/** Read CF_HDROP so paste works for copies made in File Explorer. */
export async function refreshClipboardFromSystem(): Promise<void> {
  try {
    const result = await invoke<{ paths: string[]; mode: ClipboardMode | null }>(
      "file_ops_clipboard_get",
    );
    clipboardPaths.value = result.paths ?? [];
    clipboardMode.value =
      result.mode === "copy" || result.mode === "cut" ? result.mode : null;
    if (!clipboardPaths.value.length) clipboardMode.value = null;
  } catch {
    /* keep the last local snapshot if the shell clipboard is busy */
  }
}

function samePath(left: string, right: string): boolean {
  const norm = (value: string) =>
    value.trim().replace(/\//g, "\\").replace(/\\+$/, "").toLowerCase();
  return norm(left) === norm(right);
}

function baseName(path: string): string {
  const norm = path.replace(/\//g, "\\").replace(/\\+$/, "");
  const idx = norm.lastIndexOf("\\");
  return idx >= 0 ? norm.slice(idx + 1) : norm;
}

export async function pasteClipboard(destDir: string): Promise<FileOpsResult> {
  await refreshClipboardFromSystem();
  const paths = [...clipboardPaths.value];
  const mode = clipboardMode.value;
  if (!paths.length || !mode) {
    return { ok: false, paths: [], error: "Clipboard is empty" };
  }
  const result = await transferEntries(paths, destDir, mode === "copy" ? "copy" : "move");
  if (result.ok && mode === "cut") clearClipboard();
  return result;
}

export async function transferEntries(
  sources: string[],
  destDir: string,
  mode: "copy" | "move",
): Promise<FileOpsResult> {
  const command = mode === "copy" ? "file_ops_copy" : "file_ops_move";
  const transferred: string[] = [];
  for (const source of sources) {
    const name = baseName(source);
    const destFile = `${destDir.replace(/[\\/]+$/, "")}\\${name}`;
    let conflict = "keep";
    if (!samePath(source, destFile)) {
      const exists = await invoke<boolean>("file_ops_name_exists", { destDir, name });
      if (exists) {
        const choice = await resolveNameConflictChoice(name);
        if (choice === "cancel") {
          return { ok: false, paths: transferred, error: "Cancelled" };
        }
        conflict = choice;
      }
    }
    const result = await invoke<FileOpsResult>(command, {
      sources: [source],
      destDir,
      conflict,
    });
    if (!result.ok) return result;
    transferred.push(...result.paths);
  }
  return { ok: true, paths: transferred, error: null };
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

export async function deleteEntries(
  paths: string[],
  permanent = false,
): Promise<FileOpsResult> {
  const command = permanent ? "file_ops_delete_permanent" : "file_ops_delete";
  return invoke<FileOpsResult>(command, { paths });
}

export async function pathsWithoutRecycle(paths: string[]): Promise<string[]> {
  return invoke<string[]>("file_ops_can_recycle", { paths });
}
