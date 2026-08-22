import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  define: {
    __APP_BUILD_ID__: JSON.stringify(
      process.env.APP_BUILD_ID ?? new Date().toISOString().replace("T", " ").slice(0, 19)
    ),
  },
});
