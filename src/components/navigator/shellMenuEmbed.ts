// SPDX-License-Identifier: GPL-3.0-or-later
/** Windows shell verbs adapter：扁平化 + 加载/调用（AIA-239）。 */

import { invoke } from "@tauri-apps/api/core";
import type { ShellEmbedMenuChild } from "./contextMenuModel";

export type ShellContextMenuItem = {
  id: number;
  name: string;
  verb: string | null;
  children: ShellContextMenuItem[] | null;
};

export type FlattenedShellItem = {
  commandId: number;
  verb: string | null;
  label: string;
};

type GetShellContextMenuResult = {
  success: boolean;
  items: ShellContextMenuItem[];
  error: string | null;
};

type InvokeShellMenuResult = {
  success: boolean;
  error: string | null;
};

export type ShellEmbedLabels = {
  loading: string;
  error: string;
  empty: string;
};

/** Recursively flatten nested shell parents into "Parent › Child" leaf rows. */
export function flattenShellItems(
  items: ShellContextMenuItem[],
  prefix = "",
): FlattenedShellItem[] {
  const out: FlattenedShellItem[] = [];
  for (const item of items) {
    const label = prefix ? `${prefix} › ${item.name}` : item.name;
    if (item.children?.length) {
      out.push(...flattenShellItems(item.children, label));
      continue;
    }
    if (!item.id && !item.verb) continue;
    out.push({
      commandId: item.id,
      verb: item.verb,
      label,
    });
  }
  return out;
}

function placeholderChild(
  id: string,
  labelKey: string,
  label: string,
): ShellEmbedMenuChild {
  return {
    id,
    labelKey,
    rawLabel: label,
    label,
    enabled: false,
    shellCommandId: 0,
    shellVerb: null,
  };
}

/** 加载 shell 子项（含 loading/empty/error 占位）。 */
export async function loadShellEmbedChildren(
  path: string,
  labels: ShellEmbedLabels,
): Promise<{ children: ShellEmbedMenuChild[]; error: string | null }> {
  try {
    const result = await invoke<GetShellContextMenuResult>("shell_get_context_menu", {
      path,
    });
    if (!result.success) {
      const message = result.error || labels.error;
      return {
        children: [
          placeholderChild("shell:error", "contextMenu.systemMenuError", message),
        ],
        error: message,
      };
    }
    const flat = flattenShellItems(result.items ?? []);
    if (!flat.length) {
      return {
        children: [
          placeholderChild("shell:empty", "contextMenu.systemMenuEmpty", labels.empty),
        ],
        error: null,
      };
    }
    return {
      children: flat.map((row, index) => ({
        id: `shell:${row.commandId}:${index}`,
        labelKey: "contextMenu.systemMenu",
        rawLabel: row.label,
        label: row.label,
        enabled: true,
        shellCommandId: row.commandId,
        shellVerb: row.verb,
      })),
      error: null,
    };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return {
      children: [
        placeholderChild("shell:error", "contextMenu.systemMenuError", message),
      ],
      error: message,
    };
  }
}

export function shellEmbedLoadingPlaceholder(label: string): ShellEmbedMenuChild[] {
  return [placeholderChild("shell:loading", "contextMenu.systemMenuLoading", label)];
}

/** 调用单个 shell verb；失败返回错误文案。 */
export async function invokeShellEmbedChild(
  path: string,
  child: Pick<ShellEmbedMenuChild, "shellCommandId" | "shellVerb">,
): Promise<string | null> {
  try {
    const result = await invoke<InvokeShellMenuResult>("shell_invoke_context_menu_item", {
      path,
      commandId: child.shellCommandId,
      commandVerb: child.shellVerb,
    });
    if (!result.success && result.error) return result.error;
    return null;
  } catch (err) {
    return err instanceof Error ? err.message : String(err);
  }
}
