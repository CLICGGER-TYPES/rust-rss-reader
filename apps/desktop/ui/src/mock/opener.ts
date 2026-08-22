// mock @tauri-apps/plugin-opener
export async function openUrl(url: string): Promise<void> {
  console.log("[mock] openUrl:", url);
}
export async function openPath(path: string): Promise<void> {
  console.log("[mock] openPath:", path);
}
export async function revealItemInDir(path: string): Promise<void> {
  console.log("[mock] revealItemInDir:", path);
}
