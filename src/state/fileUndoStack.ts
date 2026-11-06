// SPDX-License-Identifier: GPL-3.0-or-later
/** Pure undo-stack helpers (no Vue / IPC). */

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
    }
  | {
      kind: "create";
      path: string;
    }
  | {
      kind: "delete";
      originalPaths: string[];
    };

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
