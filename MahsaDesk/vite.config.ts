import { defineConfig } from "vite";

// Tauri expects a fixed port; fail if occupied.
export default defineConfig({
  clearScreen: false,
  server: {
    port: 15230,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: "chrome110",
    minify: "esbuild",
    sourcemap: false,
  },
});
