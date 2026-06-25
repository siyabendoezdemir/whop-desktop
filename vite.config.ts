import { defineConfig } from "vite";

// Minimal Vite config. This local frontend exists only to satisfy Tauri's
// `frontendDist` bundling requirement. The actual app window loads
// https://whop.com directly as a top-level external URL (see src-tauri/src/lib.rs),
// so this bundle is effectively a fallback/placeholder that the user never sees.
export default defineConfig({
  clearScreen: false,
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2021",
  },
  server: {
    port: 1420,
    strictPort: true,
  },
});
