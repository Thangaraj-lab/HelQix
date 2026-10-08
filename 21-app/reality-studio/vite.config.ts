/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// HELQIX Reality Studio build configuration.
// The dev proxy targets the Layer 22 API server when it is running locally.
// If VITE_API_ORIGIN is not set, the app operates against the typed
// integration boundary and reports endpoints as UNAVAILABLE (never faked).
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: process.env.VITE_API_ORIGIN
      ? {
          "/api": {
            target: process.env.VITE_API_ORIGIN,
            changeOrigin: true,
          },
        }
      : undefined,
  },
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: "./src/test/setup.ts",
    css: false,
  },
});
