import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Tauri injects TAURI_DEV_HOST when running against a physical device.
const host = process.env["TAURI_DEV_HOST"];

export default defineConfig({
  plugins: [react(), tailwindcss()],

  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },

  // Tauri owns the terminal output; keep Vite quiet so Rust logs stay readable.
  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
    host: host ?? false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      // The Rust side is watched by `tauri dev`, not Vite.
      ignored: ["**/src-tauri/**"],
    },
  },

  envPrefix: ["VITE_", "TAURI_ENV_*"],

  build: {
    // Tauri ships a fixed WebView per platform, so target that engine directly
    // instead of down-levelling for the open web.
    target: process.env["TAURI_ENV_PLATFORM"] === "windows" ? "chrome105" : "safari15",
    sourcemap: Boolean(process.env["TAURI_ENV_DEBUG"]),
  },
});
