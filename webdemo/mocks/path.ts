// Mock of @tauri-apps/api/path
export const sep = "\\";
export async function resolve(...parts: string[]) { return parts.join("\\"); }
export async function normalize(p: string) { return p.replace(/\//g, "\\").replace(/\\+$/, ""); }
export async function join(...parts: string[]) { return parts.map((p, i) => (i && p.startsWith("\\")) ? p.slice(1) : p).join("\\"); }
export async function dirname(p: string) { const n = p.replace(/\//g, "\\").replace(/\\+$/, ""); const i = n.lastIndexOf("\\"); return i > 2 ? n.slice(0, i) : n.slice(0, 3); }
export async function basename(p: string, ext?: string) { const n = p.replace(/\//g, "\\"); const b = n.slice(n.lastIndexOf("\\") + 1); return ext && b.endsWith(ext) ? b.slice(0, -ext.length) : b; }
export async function extname(p: string) { const b = await basename(p); const i = b.lastIndexOf("."); return i > 0 ? b.slice(i) : ""; }
export async function isAbsolute(p: string) { return /^[A-Za-z]:\\/.test(p) || p.startsWith("\\\\"); }
export function resolveResource(r: string) { return Promise.resolve("G:\\demo-resources\\" + r); }
export function appConfigDir() { return Promise.resolve("C:\\Users\\demo\\AppData\\Roaming\\a-file-manager"); }
export function appDataDir() { return Promise.resolve("C:\\Users\\demo\\AppData\\Roaming\\a-file-manager"); }
export function homeDir() { return Promise.resolve("C:\\Users\\demo"); }
export function tempDir() { return Promise.resolve("C:\\Users\\demo\\AppData\\Local\\Temp"); }
export function desktopDir() { return Promise.resolve("C:\\Users\\demo\\Desktop"); }
export function documentDir() { return Promise.resolve("C:\\Users\\demo\\Documents"); }
export function downloadDir() { return Promise.resolve("C:\\Users\\demo\\Downloads"); }
export function pictureDir() { return Promise.resolve("C:\\Users\\demo\\Pictures"); }
export function videoDir() { return Promise.resolve("C:\\Users\\demo\\Videos"); }
export function musicDir() { return Promise.resolve("C:\\Users\\demo\\Music"); }
