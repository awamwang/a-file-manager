// SPDX-License-Identifier: GPL-3.0-or-later
/** Human-readable byte sizes for status / properties. */

export function formatBytes(size: number): string {
  if (!Number.isFinite(size) || size < 0) return "0 B";
  if (size < 1024) return `${Math.round(size)} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  if (size < 1024 * 1024 * 1024) return `${(size / (1024 * 1024)).toFixed(1)} MB`;
  return `${(size / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function selectionByteSize(
  selected: readonly string[],
  entries: ReadonlyArray<{ path: string; kind: string; size: number }>,
): number {
  if (!selected.length) return 0;
  const set = new Set(selected);
  let total = 0;
  for (const entry of entries) {
    if (!set.has(entry.path)) continue;
    if (entry.kind === "dir") continue;
    total += Math.max(0, entry.size || 0);
  }
  return total;
}
