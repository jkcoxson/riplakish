import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte()],
  build: {
    rollupOptions: {
      output: {
        entryFileNames: "scripts.js",
        assetFileNames: "styles.css",
        chunkFileNames: "chunk.js",
        manualChunks: undefined,
      },
    },
  },
});
