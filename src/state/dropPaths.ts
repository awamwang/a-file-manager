// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Resolve filesystem paths from a DataTransfer (AIA-205).
 * Supports in-app AFM payload, WebView2 `File.path`, and `text/uri-list`.
 */

const AFM_MIME = "application/x-afm-paths";

type FileWithPath = File & { path?: string };

/** `file:///C:/foo` / `file:/C:/foo` → `C:\foo`; UNC `file://server/share` → `\\server\share`. */
export function fileUrlToPath(url: string): string | null {
  const raw = url.trim();
  if (!raw || raw.startsWith("#")) return null;
  let decoded: string;
  try {
    decoded = decodeURIComponent(raw.replace(/\+/g, "%20"));
  } catch {
    decoded = raw;
  }
  if (!/^file:/i.test(decoded)) {
    // Plain Windows path pasted as text
    if (/^[a-zA-Z]:[\\/]/.test(decoded) || decoded.startsWith("\\\\")) {
      return decoded.replace(/\//g, "\\");
    }
    return null;
  }
  // file:///C:/… or file://localhost/C:/…
  const m = /^file:\/\/(?:localhost)?\/([a-zA-Z]):[\\/](.*)$/i.exec(decoded);
  if (m) {
    return `${m[1].toUpperCase()}:\\${m[2].replace(/\//g, "\\")}`;
  }
  // file://server/share/…
  const unc = /^file:\/\/([^/]+)\/(.*)$/i.exec(decoded);
  if (unc) {
    return `\\\\${unc[1]}\\${unc[2].replace(/\//g, "\\")}`;
  }
  // file:/C:/…
  const short = /^file:\/([a-zA-Z]):[\\/](.*)$/i.exec(decoded);
  if (short) {
    return `${short[1].toUpperCase()}:\\${short[2].replace(/\//g, "\\")}`;
  }
  return null;
}

export function pathToFileUrl(path: string): string {
  const normalized = path.trim().replace(/\\/g, "/");
  if (normalized.startsWith("//")) {
    return `file:${normalized}`;
  }
  return `file:///${normalized}`;
}

function uniquePaths(paths: string[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const raw of paths) {
    const p = raw.trim().replace(/\//g, "\\");
    if (!p) continue;
    const key = p.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(p);
  }
  return out;
}

/** True when the drag payload may carry OS files (Explorer / other apps). */
export function dataTransferLooksExternal(dt: DataTransfer | null | undefined): boolean {
  if (!dt) return false;
  const types = Array.from(dt.types ?? []);
  if (types.includes(AFM_MIME)) return false;
  return types.includes("Files") || types.includes("text/uri-list");
}

export function pathsFromDataTransfer(dt: DataTransfer | null | undefined): string[] {
  if (!dt) return [];

  try {
    const raw = dt.getData(AFM_MIME);
    if (raw) {
      const parsed = JSON.parse(raw) as unknown;
      if (Array.isArray(parsed)) {
        const afm = uniquePaths(parsed.map(String));
        if (afm.length) return afm;
      }
    }
  } catch {
    /* fall through */
  }

  const fromFiles: string[] = [];
  if (dt.files && dt.files.length > 0) {
    for (let i = 0; i < dt.files.length; i++) {
      const file = dt.files.item(i) as FileWithPath | null;
      const path = file?.path?.trim();
      if (path) fromFiles.push(path);
    }
  }
  if (fromFiles.length) return uniquePaths(fromFiles);

  const uriBlob =
    dt.getData("text/uri-list") ||
    dt.getData("text/plain") ||
    "";
  if (uriBlob) {
    const fromUri: string[] = [];
    for (const line of uriBlob.split(/\r?\n/)) {
      const path = fileUrlToPath(line);
      if (path) fromUri.push(path);
    }
    if (fromUri.length) return uniquePaths(fromUri);
  }

  return [];
}

/** Build uri-list + plain text companions for outbound HTML5 drag (OLE still primary). */
export function setOutboundPathPayload(dt: DataTransfer, paths: string[]): void {
  const cleaned = uniquePaths(paths);
  dt.setData(AFM_MIME, JSON.stringify(cleaned));
  dt.setData("text/uri-list", cleaned.map(pathToFileUrl).join("\r\n"));
  dt.setData("text/plain", cleaned.join("\r\n"));
}
