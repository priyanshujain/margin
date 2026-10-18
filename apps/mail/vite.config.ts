import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import type { Plugin } from "vite";
// A relative path rather than the package name, so Vite bundles this TypeScript into the config
// instead of handing it to Node, which will not strip types under node_modules.
import { themeBootScript } from "./node_modules/@margin/core/src/hooks/theme";
import { THEME_PALETTES } from "./src/store/themePalettes";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// Ports are offset from margin's 1420/1421, the calendar's 1430/1431 and the editor's 1440/1441, so
// the four apps can run side by side. The Playwright config reuses whatever answers on the port, so
// a port shared with a sibling is a suite that silently drives the wrong app.
// The theme, phone and touch attributes are margin-core's boot script, inlined so the first paint is
// already the right palette and shape.
function themeBoot(): Plugin {
  return {
    name: "margin-theme-boot",
    transformIndexHtml: (html) =>
      html.replace(
        "<!--theme-boot-->",
        `<script>${themeBootScript({ prefix: "marginmail", palettes: THEME_PALETTES })}</script>`,
      ),
  };
}

export default defineConfig({
  plugins: [react(), themeBoot()],

  // Core is TypeScript source, so it is served module by module like the app's own rather than
  // pre-bundled.
  optimizeDeps: { exclude: ["@margin/core"] },

  clearScreen: false,
  server: {
    port: 1450,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1451,
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
