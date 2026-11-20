import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],

  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },

  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
    restoreMocks: true,
    coverage: {
      provider: "v8",
      reporter: ["text", "lcov"],
      // Scope matches the roadmap gate (M11-05): logic-bearing code only.
      // Primitives are exercised through the gallery and component tests.
      include: ["src/lib/**", "src/stores/**", "src/components/status/**"],
      exclude: ["src/lib/bindings/**", "src/**/*.test.*"],
      thresholds: {
        lines: 80,
        functions: 80,
        branches: 75,
      },
    },
  },
});
