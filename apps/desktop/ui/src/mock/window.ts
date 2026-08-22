// mock @tauri-apps/api/window
export function getCurrentWindow() {
  return {
    setDecorations: async () => {},
    minimize: async () => {},
    toggleMaximize: async () => {},
    maximize: async () => {},
    close: async () => {},
    setSize: async () => {},
  };
}
