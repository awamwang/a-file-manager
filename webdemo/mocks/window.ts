// Mock of @tauri-apps/api/window
function fakeWindow() {
  const noop = async (..._a: unknown[]) => {};
  return {
    label: "main",
    isMaximized: async () => true,
    isFocused: async () => true,
    isVisible: async () => true,
    maximize: noop, unmaximize: noop, toggleMaximize: noop,
    minimize: noop, restore: noop, show: noop, hide: noop, close: noop,
    setFocus: noop, setAlwaysOnTop: noop, setSize: noop, setMinSize: noop,
    setPosition: noop, setTitle: noop, setDecorations: noop, setResizable: noop,
    setFullscreen: noop, setIgnoreCursorEvents: noop, startDragging: noop,
    internalToggleMaximize: noop,
    innerSize: async () => ({ width: 1400, height: 900 }),
    outerSize: async () => ({ width: 1400, height: 900 }),
    innerPosition: async () => ({ x: 0, y: 0 }),
    outerPosition: async () => ({ x: 0, y: 0 }),
    scaleFactor: async () => 1,
    onCloseRequested: async (_cb?: unknown) => (() => {}),
    onResized: async (_cb?: unknown) => (() => {}),
    onMoved: async (_cb?: unknown) => (() => {}),
    onFocusChanged: async (_cb?: unknown) => (() => {}),
    onScaleFactorChanged: async (_cb?: unknown) => (() => {}),
    currentMonitor: async () => null,
    primaryMonitor: async () => null,
  };
}
export function getCurrentWindow() { return fakeWindow(); }
export function getAllWindows() { return Promise.resolve([fakeWindow()]); }
export function getCurrentWebviewWindow() { return fakeWindow(); }
export class Window { constructor(public override readonly label: string = "main") {} }
