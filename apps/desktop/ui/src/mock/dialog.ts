// mock @tauri-apps/plugin-dialog
export async function open(options?: Record<string, unknown>): Promise<string | string[] | null> {
  console.log("[mock] open dialog:", options);
  return "/tmp/mock-import.opml";
}

export async function save(options?: Record<string, unknown>): Promise<string | null> {
  console.log("[mock] save dialog:", options);
  return "/tmp/mock-export.opml";
}

export async function message(message: string, options?: Record<string, unknown>): Promise<void> {
  console.log("[mock] message:", message, options);
}

export async function ask(message: string, options?: Record<string, unknown>): Promise<boolean> {
  console.log("[mock] ask:", message, options);
  return true;
}

export async function confirm(message: string, options?: Record<string, unknown>): Promise<boolean> {
  console.log("[mock] confirm:", message, options);
  return true;
}
