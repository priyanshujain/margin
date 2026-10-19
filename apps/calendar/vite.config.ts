import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import type { Plugin } from "vite";
// A relative path rather than the package name, so Vite bundles this TypeScript into the config
// instead of handing it to Node, which will not strip types under node_modules.
import { themeBootScript } from "./node_modules/@margin/core/src/hooks/theme";
import { THEME_PALETTES } from "./src/store/themePalettes";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// The theme, phone and touch attributes are margin-core's boot script, inlined so the first paint is
// already the right palette and shape.
function themeBoot(): Plugin {
  return {
    name: "margin-theme-boot",
    transformIndexHtml: (html) =>
      html.replace(
        "<!--theme-boot-->",
        `<script>${themeBootScript({ prefix: "margincal", palettes: THEME_PALETTES })}</script>`,
      ),
  };
}

// Ports are offset from margin's 1420/1421 so both apps can run side by side.
export default defineConfig({
  plugins: [react(), themeBoot()],

  // Core is TypeScript source, so it is served module by module like the app's own rather than
  // pre-bundled.
  optimizeDeps: { exclude: ["@margin/core"] },

  clearScreen: false,
  server: {
    port: 1430,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1431,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
  },
});
