import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Standard Vite config. The Tauri dev server points at port 5173
// (see apps/launcher/src-tauri/tauri.conf.json).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    target: "es2022",
    outDir: "dist",
    sourcemap: true,
  },
});
