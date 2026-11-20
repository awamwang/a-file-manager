// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Single source of truth for frontend **local filesystem path** semantics.
 * Mirrors the Rust `platform` adapter's separator rules (see
 * `src-tauri/src/platform/{windows,macos}.rs`):
 *
 * - POSIX absolute paths (a single leading `/`, not `//`) keep `/` and their
 *   leading root. A backslash is a legal filename character there.
 * - Windows drive / UNC paths normalize to `\`.
 * - URI schemes (`afm://guide`, `http://…`) keep their own separators.
 *
 * 7.14 / 7.16: four modules used to hand-roll this decision independently
 * (`navHistory`, `pathKey`, `navigatorBrowse`, `settingsEffectiveDefaults`),
 * which produced real paths like `Users\demo` on macOS. New code MUST
 * import from here instead of writing `replace(/\//g, "\\")` on its own.
 * Provider URIs (`awam://…`) are an application-level concept and stay in
 * their own modules.
 */

/** True for `/Users/...`, false for `//server/share` (UNC) and `C:\...`. */
export function isPosixAbsolute(path: string): boolean {
  const trimmed = path.trim();
  return trimmed.startsWith("/") && !trimmed.startsWith("//");
}

/** Host-native separator for an already-normalized local path. */
export function fsPathSeparator(path: string): string {
  return isPosixAbsolute(path) ? "/" : "\\";
}

/**
 * Normalize a display / config path to a real host filesystem path.
 * This is the frontend counterpart of `crate::platform::to_native_fs_path`.
 */
export function toNativeFsPath(path: string): string {
  const trimmed = path.trim();
  if (!trimmed) return "";
  if (isPosixAbsolute(trimmed)) {
    return trimmed.replace(/\/+$/, "") || "/";
  }
  if (
    /^[a-zA-Z]:[\\/]/.test(trimmed) ||
    trimmed.startsWith("\\\\") ||
    trimmed.startsWith("//")
  ) {
    const native = trimmed.replace(/\//g, "\\");
    if (/^[a-zA-Z]:\\$/i.test(native)) return native;
    return native.replace(/\\+$/, "");
  }
  // `afm://guide`, `http://…` — never rewrite the scheme separators.
  if (/^[a-z][a-z0-9+.-]*:/i.test(trimmed)) {
    return trimmed.replace(/\/+$/, "");
  }
  return trimmed.replace(/\//g, "\\").replace(/\\+$/, "");
}

/** Join a filename onto a local parent path with the host-native separator. */
export function joinFsPath(parent: string, name: string): string {
  const native = toNativeFsPath(parent);
  if (!native) return name;
  if (isPosixAbsolute(native)) {
    // `"/"` trims to `""`, so `${root}/${name}` yields `/Users` rather than `//Users`.
    const root = native.replace(/\/+$/, "");
    return `${root}/${name}`;
  }
  if (/^[a-zA-Z]:\\?$/i.test(native)) {
    return `${native.slice(0, 2)}\\${name}`;
  }
  return `${native.replace(/\\+$/, "")}\\${name}`;
}

/** Split a local path into root + segments for breadcrumb-style rendering. */
export function splitFsPath(path: string): { root: string; segments: string[] } {
  const trimmed = path.trim();
  if (isPosixAbsolute(trimmed)) {
    return { root: "/", segments: trimmed.split("/").filter(Boolean) };
  }
  const normalized = toNativeFsPath(trimmed);
  if (normalized.startsWith("\\\\")) {
    const parts = normalized.slice(2).split("\\").filter(Boolean);
    if (!parts.length) return { root: normalized, segments: [] };
    return { root: `\\\\${parts[0]}`, segments: parts.slice(1) };
  }
  const drive = normalized.match(/^([A-Za-z]:\\?)(.*)$/);
  if (drive) {
    return {
      root: `${drive[1].slice(0, 2)}\\`,
      segments: drive[2].split("\\").filter(Boolean),
    };
  }
  return { root: "", segments: normalized.split("\\").filter(Boolean) };
}
