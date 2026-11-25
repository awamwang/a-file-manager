// Mock of @tauri-apps/plugin-dialog
type OpenOptions = Record<string, unknown>;
export async function open(_opts?: OpenOptions | string): Promise<string | string[] | null> { return null; }
export async function save(_opts?: Record<string, unknown> | string): Promise<string | null> { return null; }
export async function message(_msg: string, _opts?: Record<string, unknown> | string): Promise<void> {}
export async function ask(_msg: string, _opts?: Record<string, unknown> | string): Promise<boolean> { return true; }
export async function confirm(_msg: string, _opts?: Record<string, unknown> | string): Promise<boolean> { return true; }
export const tauri = { open, save, message, ask, confirm };
