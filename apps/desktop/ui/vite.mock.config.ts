import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const src = path.resolve(__dirname, "src");

// 浏览器内 mock 开发配置：把 Tauri 相关模块替换为本地 mock，
// 方便在普通 Chromium 里交互测试同一套 App.tsx。
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@tauri-apps/api/core": path.join(src, "mock/tauri-core.ts"),
      "@tauri-apps/api/window": path.join(src, "mock/window.ts"),
      "@tauri-apps/plugin-opener": path.join(src, "mock/opener.ts"),
      "@tauri-apps/plugin-dialog": path.join(src, "mock/dialog.ts"),
    },
  },
  server: {
    port: 5174,
    strictPort: true,
  },
  build: {
    outDir: "dist-mock",
    emptyOutDir: true,
  },
});
