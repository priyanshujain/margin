import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import type { Plugin } from "vite";
import { themeBootScript } from "./node_modules/@margin/core/src/hooks/theme";
import { THEME_PALETTES } from "./src/store/themePalettes";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

function themeBoot(): Plugin {
  return {
    name: "margin-theme-boot",
    transformIndexHtml: (html) =>
      html.replace("<!--theme-boot-->", `<script>${themeBootScript({ prefix: "margin", palettes: THEME_PALETTES })}</script>`),
  };
}

export default defineConfig({
  plugins: [react(), themeBoot()],

  resolve: {
    dedupe: ["@tiptap/core", "@tiptap/pm", "react", "react-dom", "zustand"],
  },

  optimizeDeps: {
    exclude: ["@margin/core"],
    include: [
      "@tiptap/core",
      "@tiptap/extension-list",
      "@tiptap/extension-placeholder",
      "@tiptap/pm/model",
      "@tiptap/pm/state",
      "@tiptap/pm/view",
      "@tiptap/react",
      "@tiptap/starter-kit",
    ],
  },

  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**", "**/website/**"],
    },
  },

  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
  },
});
