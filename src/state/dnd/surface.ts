// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Shared folder-drop apply path for list / tree / tabs / places (AIA-240 + AIA-242 + AIA-250).
 * Keeps volume + modifier rules and undo/refresh side effects in one place.
 */

import {
  dataTransferLooksExternal,
  pathsFromDataTransfer,
} from "./paths.ts";
import { dropTransferMode, type DropMode } from "./mode.ts";
import { effectiveDropSources } from "./location.ts";
import {
  activeDragSources,
  endDragSession,
  hideDropHint,
  showDropHint,
} from "./session.ts";
import { transferEntries } from "../fileClipboard.ts";
import { recordCopyUndo, recordMoveUndo } from "../fileUndo/index.ts";
import { clearSelection, selectedPaths } from "../navigatorSelection.ts";
import { afterFileMutation } from "../fileMutation.ts";

export type FolderDropCommit = {
  ok: boolean;
  mode?: DropMode;
  count?: number;
  error?: string;
  /** 静默 no-op：源已在目标目录，调用方不应弹 toast / 切标签 / 刷新。 */
  noop?: "same-location";
};

export type FolderDropResolution = {
  /** 真正需要搬动的路径（已剔除同位置源）。 */
  sources: string[];
  mode: DropMode;
  /** 源全部已在目标目录且当前为 move：Explorer 视为无效操作。 */
  blocked: boolean;
};

/**
 * 一次投放的统一判定。dragover 传拖动会话 / 选中项，drop 传 dataTransfer
 * 解析出的路径，保证「光标提示」和「实际动作」用同一套规则。
 */
export function resolveFolderDrop(
  event: DragEvent,
  destPath: string,
  candidates: string[],
): FolderDropResolution {
  const mode = previewDropMode(event, destPath, candidates);
  if (!candidates.length) return { sources: [], mode, blocked: false };
  // 显式 Ctrl（复制副本）/ Alt（创建快捷方式）在原目录仍有意义，保持可用。
  if (mode !== "move") return { sources: candidates, mode, blocked: false };
  const sources = effectiveDropSources(candidates, destPath);
  return { sources, mode, blocked: sources.length === 0 };
}

/** Resolve copy|move|link for dragover / drop (empty external sources → modifiers only). */
export function previewDropMode(
  event: DragEvent,
  destPath: string,
  sourcePaths: string[],
): DropMode {
  const external = dataTransferLooksExternal(event.dataTransfer);
  if (external && sourcePaths.length === 0) {
    if (event.altKey) return "link";
    if (event.shiftKey) return "move";
    return "copy";
  }
  return dropTransferMode(sourcePaths, destPath, {
    ctrl: event.ctrlKey,
    shift: event.shiftKey,
    alt: event.altKey,
  });
}

/** Paths used for dragover cursor preview (drag session wins: tab switch clears selection). */
export function previewSourcePaths(event: DragEvent): string[] {
  const session = activeDragSources();
  if (session.length) return session;
  if (dataTransferLooksExternal(event.dataTransfer)) return [];
  return [...selectedPaths.value];
}

/**
 * Accept a folder drop target: preventDefault, set dropEffect, show the release hint.
 * Returns null when the destination must not accept (selected row, or sources that
 * already live in this folder — an Explorer-style no-op).
 */
export function acceptFolderDragOver(
  event: DragEvent,
  destPath: string,
  options: {
    rejectIfSelected?: boolean;
    hintTarget?: string;
    hintKey?: string;
  } = {},
): DropMode | null {
  const hintKey = options.hintKey ?? destPath;
  const resolution = resolveFolderDrop(event, destPath, previewSourcePaths(event));
  if (
    (options.rejectIfSelected && selectedPaths.value.includes(destPath)) ||
    resolution.blocked
  ) {
    hideDropHint(hintKey);
    // 不 preventDefault：浏览器不会派发 drop，也不会出现投放光标。
    // stopPropagation 防止父级目标抢答后重新显示提示 / 高亮。
    event.stopPropagation();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "none";
    return null;
  }
  event.preventDefault();
  event.stopPropagation();
  if (event.dataTransfer) event.dataTransfer.dropEffect = resolution.mode;
  showDropHint(event, resolution.mode, options.hintTarget ?? destPath, hintKey);
  return resolution.mode;
}

/** Apply inbound / in-app drop into `destPath`; refresh listing + undo bookkeeping. */
export async function commitFolderDrop(
  event: DragEvent,
  destPath: string,
): Promise<FolderDropCommit> {
  event.preventDefault();
  event.stopPropagation();
  const transferred = pathsFromDataTransfer(event.dataTransfer);
  // WebView2 原生 OLE 回灌时 dataTransfer 常只有 Files（File.path 可能为空、
  // 也没有 uri-list）；dragover 拿不到源路径，但按下时登记的会话就是同一次
  // 拖动的源。会话由 dragend / pointerdown / 本次 drop 清理且有 TTL，
  // 外部拖入时它为空，因此不会借用上一次的路径。
  const sessionPaths = activeDragSources();
  const resolved =
    transferred.length || !sessionPaths.length
      ? transferred
      : sessionPaths.map((p) => p.replace(/\//g, "\\"));
  const resolution = resolveFolderDrop(event, destPath, resolved);
  if (resolution.blocked) {
    // 拖回原目录：不产生任何文件操作、提示、toast 或标签切换。
    endDragSession();
    return { ok: false, noop: "same-location" };
  }
  const paths = resolution.sources;
  if (!paths.length) {
    endDragSession();
    return { ok: false, error: "empty" };
  }

  try {
    const mode = resolution.mode;
    const result = await transferEntries(paths, destPath, mode);
    if (!result.ok) {
      return { ok: false, mode, error: result.error ?? "failed" };
    }
    if (mode === "copy") recordCopyUndo(result.paths);
    else if (mode === "move") {
      recordMoveUndo(paths, result.paths);
    }
    clearSelection();
    // 目标可能不是当前路径；移动时还需作废源父目录。
    await afterFileMutation({
      invalidateEntryPaths: mode === "move" ? paths : undefined,
      invalidateDirs: [destPath],
    });
    return { ok: true, mode, count: result.paths.length };
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : String(err),
    };
  } finally {
    endDragSession();
  }
}

export function noticeKeyForDropMode(mode: DropMode): string {
  if (mode === "copy") return "navigator.copiedInto";
  if (mode === "move") return "navigator.movedInto";
  return "navigator.linkedInto";
}
