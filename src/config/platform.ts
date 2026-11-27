// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Frontend bridge to the Rust `platform/` capability layer (macOS port, step 1).
 *
 * Mirrors the cross-platform browser pattern: the UI asks a neutral contract
 * what the host can do, instead of sprinkling `if (isMac)` over components.
 * The authoritative answer comes from the `platform_capabilities` IPC command;
 * a synchronous user-agent check is only a cold-start / vite-only fallback for
 * shortcut matching, which cannot await IPC inside a keydown handler.
 *
 * This module intentionally has no static import of `@tauri-apps/api` or Vue:
 * `scripts/shortcut-checks.ts` imports the chord helpers directly in Node.
 */

/** Keep in sync with `PlatformCapabilities` in `src-tauri/src/platform/mod.rs`
 * (serde `rename_all = "camelCase"`). */
export type PlatformCapabilities = {
  os: string;
  arch: string;
  /** "ctrl" on Windows/Linux, "cmd" on macOS. */
  primaryModifier: string;
  pathSeparator: string;
  caseInsensitivePaths: boolean;
  shellIcons: boolean;
  systemThumbnails: boolean;
  trashDelete: boolean;
  trashListRestore: boolean;
  /** File clipboard provider registered (CF_HDROP on Windows). 7.24. */
  shellClipboard: boolean;
  nativeOpenWithDialog: boolean;
  nativeContextMenu: boolean;
  osShellVerb: boolean;
  registryAutostart: boolean;
  nativeAutostart: boolean;
  /** Store (MSIX) build: autostart is owned by windows.startupTask, not HKCU Run. */
  storeBuild: boolean;
  singleInstance: boolean;
  globalSearch: boolean;
  /** Host-wide index: "everything" (Windows) | "spotlight" (macOS) | "none". */
  globalSearchEngine: "everything" | "spotlight" | "none";
  quickLookPreview: boolean;
  /** PowerToys Peek CLI (Windows only); hidden from macOS preview options. */
  peekPreview: boolean;
  oleDragOut: boolean;
  /** AppKit / OS-native drag-out. Off on macOS (7.13 command abort). */
  nativeDragOut: boolean;
  /** WebView-native HTML5 file drag-out (`text/uri-list`). */
  webviewDragOut: boolean;
  hiddenFileAttributes: boolean;
  driveLetters: boolean;
  /** Mounted volumes enumerable, independent of drive-letter shape (7.16). */
  volumeList: boolean;
  systemTray: boolean;
  webview2Required: boolean;
  /** Synchronous OS microphone permission pre-check (Windows ConsentStore). */
  microphonePermissionPrecheck: boolean;
  /** Managed feed ships native sidecar artifacts for this host (per-OS builds). */
  sidecarNativeArtifacts: boolean;
};

export const PLATFORM_COMMAND = "platform_capabilities";

/**
 * Used before the IPC answers (vite-only dev, command missing). `os: ""` is a
 * deliberate "unknown" sentinel: capability-gated UI stays unchanged until
 * we know the real host, so Windows never loses a setting by accident.
 */
export const NEUTRAL_PLATFORM: PlatformCapabilities = {
  os: "",
  arch: "",
  primaryModifier: "ctrl",
  pathSeparator: "\\",
  caseInsensitivePaths: false,
  shellIcons: false,
  systemThumbnails: false,
  trashDelete: false,
  trashListRestore: false,
  shellClipboard: false,
  nativeOpenWithDialog: false,
  nativeContextMenu: false,
  osShellVerb: false,
  registryAutostart: false,
  nativeAutostart: false,
  storeBuild: false,
  singleInstance: false,
  globalSearch: false,
  globalSearchEngine: "none",
  quickLookPreview: false,
  peekPreview: false,
  oleDragOut: false,
  nativeDragOut: false,
  webviewDragOut: false,
  hiddenFileAttributes: false,
  driveLetters: false,
  volumeList: false,
  systemTray: false,
  webview2Required: false,
  microphonePermissionPrecheck: false,
  sidecarNativeArtifacts: false,
};

let cached: PlatformCapabilities | null = null;
let hydrateOnce: Promise<PlatformCapabilities> | null = null;

/** Last resolved capabilities; neutral while unknown. */
export function getCachedPlatformCapabilities(): PlatformCapabilities {
  return cached ?? NEUTRAL_PLATFORM;
}

/** Single-flight IPC. Never rejects: a missing command degrades to neutral. */
export function hydratePlatform(): Promise<PlatformCapabilities> {
  if (hydrateOnce) return hydrateOnce;
  hydrateOnce = (async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const caps = await invoke<PlatformCapabilities>(PLATFORM_COMMAND);
      cached = caps;
      return caps;
    } catch {
      // IPC unavailable (vite-only) or command not registered yet.
      // Keep `cached` null so `isMacPlatform()` stays on the user-agent
      // fallback, and allow a later call to retry.
      hydrateOnce = null;
      cached = null;
      return NEUTRAL_PLATFORM;
    }
  })();
  return hydrateOnce;
}

/** Cold-start fallback for synchronous keydown matching. */
function looksLikeMacUserAgent(): boolean {
  if (typeof navigator === "undefined") return false;
  return /Macintosh|Mac OS X|MacIntel/.test(navigator.userAgent ?? "");
}

/** True when the primary modifier is ⌘ (Cmd), i.e. macOS. */
export function isMacPlatform(): boolean {
  if (cached) {
    return cached.primaryModifier === "cmd" || cached.os === "macos";
  }
  return looksLikeMacUserAgent();
}

/** Replace Windows modifier words in human-readable shortcut text (⌘/⌥/⇧). */
export function platformShortcutLabel(label: string): string {
  if (!isMacPlatform()) return label;
  return label
    .replace(/\bCtrl\b/g, "⌘")
    .replace(/\bAlt\b/g, "⌥")
    .replace(/\bShift\b/g, "⇧")
    .replace(/\bMeta\b/g, "⌘");
}

/** `aria-keyshortcuts` uses W3C key names, not glyphs: Control → Meta on macOS. */
export function platformAriaKeys(label: string): string {
  if (!isMacPlatform()) return label;
  return label.replace(/\bControl\b/g, "Meta");
}
