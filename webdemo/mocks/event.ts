// Mock of @tauri-apps/api/event
export type UnlistenFn = () => void;

export async function listen<T = unknown>(_event: string, _handler: (e: { payload: T }) => void): Promise<UnlistenFn> {
  return () => {};
}
export async function once<T = unknown>(_event: string, _handler: (e: { payload: T }) => void): Promise<UnlistenFn> {
  return () => {};
}
export async function emit(_event: string, _payload?: unknown): Promise<void> {}
export function emitTo<T = unknown>(_label: string, _event: string, _payload?: unknown): Promise<void> { return Promise.resolve(); }
