// SPDX-License-Identifier: GPL-3.0-or-later
/** Pure undo-stack helpers (no Vue / IPC). */

/** UN-008 change-detection token (`file_ops_fingerprints`, serde camelCase). */
export type Fingerprint = { size: number; modifiedMs: number };

export type UndoRecord =
  | {
      kind: "rename";
      currentPath: string;
      previousName: string;
    }
  | {
      kind: "move";
      items: Array<{ currentPath: string; originalDir: string }>;
    }
  | {
      kind: "copy";
      createdPaths: string[];
      /** UN-008: record-time fingerprints of `createdPaths` (one per path,
       * null where unreadable). Absent until the async capture resolves —
       * undo must proceed when it is missing, never block. */
      fingerprints?: Array<Fingerprint | null>;
    }
  | {
      kind: "create";
      path: string;
      /** UN-008: record-time fingerprint of `path`. */
      fingerprints?: Array<Fingerprint | null>;
    }
  | {
      kind: "delete";
      originalPaths: string[];
    };

/** UN-008 (已拍板 2026-11-22): did the on-disk state drift from the record-time
 * baseline? Only a real change on a still-present path counts (same index,
 * different size/mtime): a vanished path (current null) is left to the delete
 * path's own handling, and a missing baseline (stored null/undefined) must
 * never block the undo. */
export function isContaminated(
  stored: ReadonlyArray<Fingerprint | null> | undefined,
  current: ReadonlyArray<Fingerprint | null>,
): boolean {
  if (!stored || stored.length === 0) return false;
  for (let i = 0; i < stored.length; i++) {
    const before = stored[i];
    const now = current[i];
    if (!before || !now) continue;
    if (before.size !== now.size || before.modifiedMs !== now.modifiedMs) {
      return true;
    }
  }
  return false;
}

export function parentDir(path: string): string {
  const norm = path.replace(/\//g, "\\").replace(/\\+$/, "");
  const idx = norm.lastIndexOf("\\");
  return idx > 0 ? norm.slice(0, idx) : norm;
}

export function baseName(path: string): string {
  const norm = path.replace(/\//g, "\\").replace(/\\+$/, "");
  const idx = norm.lastIndexOf("\\");
  return idx >= 0 ? norm.slice(idx + 1) : norm;
}

export function pushRecord(stack: UndoRecord[], record: UndoRecord): UndoRecord[] {
  return [...stack, record];
}

export function popRecord(stack: UndoRecord[]): {
  stack: UndoRecord[];
  record: UndoRecord | null;
} {
  if (!stack.length) return { stack, record: null };
  return {
    stack: stack.slice(0, -1),
    record: stack[stack.length - 1]!,
  };
}

export function renameUndoRecord(fromPath: string, toPath: string): UndoRecord {
  return {
    kind: "rename",
    currentPath: toPath,
    previousName: baseName(fromPath),
  };
}

export function moveUndoRecord(
  sources: string[],
  destinations: string[],
): UndoRecord {
  return {
    kind: "move",
    items: destinations.map((currentPath, index) => ({
      currentPath,
      originalDir: parentDir(sources[index] ?? currentPath),
    })),
  };
}

/** 撤销成功后需作废的目录 listing（源/目标父目录等）。 */
export function dirsAffectedByUndo(record: UndoRecord): string[] {
  const dirs: string[] = [];
  const push = (dir: string) => {
    const trimmed = dir.trim();
    if (!trimmed) return;
    if (!dirs.includes(trimmed)) dirs.push(trimmed);
  };
  switch (record.kind) {
    case "rename":
      push(parentDir(record.currentPath));
      break;
    case "move":
      for (const item of record.items) {
        push(parentDir(item.currentPath));
        push(item.originalDir);
      }
      break;
    case "copy":
      for (const path of record.createdPaths) push(parentDir(path));
      break;
    case "create":
      push(parentDir(record.path));
      break;
    case "delete":
      for (const path of record.originalPaths) push(parentDir(path));
      break;
  }
  return dirs;
}
