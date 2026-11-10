// SPDX-License-Identifier: GPL-3.0-or-later
/** Explorer drop rules: same volume moves, another volume copies; Alt = link. */

export type DropMode = "copy" | "move" | "link";

/** Drive letter, or `unc:server` for a UNC path. Empty when unknown. */
export function volumeKey(path: string): string {
  const trimmed = path.trim().replace(/\//g, "\\");
  const drive = /^([a-zA-Z]):/.exec(trimmed);
  if (drive) return drive[1].toLowerCase();
  if (trimmed.startsWith("\\\\")) {
    const server = trimmed.slice(2).split("\\")[0]?.toLowerCase() ?? "";
    return server ? `unc:${server}` : "";
  }
  return "";
}

export function dropTransferMode(
  sources: string[],
  dest: string,
  modifiers: { ctrl?: boolean; shift?: boolean; alt?: boolean } = {},
): DropMode {
  if (modifiers.alt) return "link";
  if (modifiers.ctrl) return "copy";
  if (modifiers.shift) return "move";
  const destVolume = volumeKey(dest);
  if (!destVolume) return "copy";
  const sameVolume = sources.every((source) => volumeKey(source) === destVolume);
  return sameVolume ? "move" : "copy";
}
