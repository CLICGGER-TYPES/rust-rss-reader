// mock @tauri-apps/api/core
import { mockInvoke } from "./backend";

export const invoke = mockInvoke;

export function convertFileSrc(path: string): string {
  return path;
}

export async function isTauri(): Promise<boolean> {
  return true;
}

export const event = {
  listen: async () => () => {},
  emit: async () => {},
};

export const window = {
  getCurrent: () => ({ label: "main" }),
};
