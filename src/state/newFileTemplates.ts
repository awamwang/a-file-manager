// SPDX-License-Identifier: GPL-3.0-or-later
/** ShellNew-backed new-file templates. */

import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";

export type NewFileTemplate = {
  extension: string;
  label: string;
  defaultName: string;
};

export const newFileTemplates = ref<NewFileTemplate[]>([]);

export async function loadNewFileTemplates(): Promise<NewFileTemplate[]> {
  try {
    const list = await invoke<NewFileTemplate[]>("file_ops_new_templates");
    newFileTemplates.value = list;
    return list;
  } catch {
    newFileTemplates.value = [
      {
        extension: "txt",
        label: "Text Document",
        defaultName: "New Text Document.txt",
      },
    ];
    return newFileTemplates.value;
  }
}
