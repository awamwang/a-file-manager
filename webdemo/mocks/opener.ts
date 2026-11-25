// Mock of @tauri-apps/plugin-opener
export async function openUrl(_url: string) {}
export async function openPath(_path: string, _with?: string) { return ""; }
export async function revealItemInDir(_path: string) {}
export async function revealItemsInDir(_paths: string[]) {}
export async function openWithDefault(_path: string) {}
export async function openWith(_path: string, _app: string) {}
const defaultApi = { openUrl, openPath, revealItemInDir, revealItemsInDir };
export default defaultApi;
