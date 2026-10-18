import { createReadStream, existsSync } from "node:fs";
import { basename, join } from "node:path";
import { fileURLToPath } from "node:url";
import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";
import { themeBootScript } from "../src/hooks/theme";

const fonts = fileURLToPath(new URL("../fonts", import.meta.url));

function kit(): Plugin {
  return {
    name: "margin-kit",
    transformIndexHtml: (html) =>
      html.replace("<!--boot-->", `<script>${themeBootScript({ prefix: "kit" })}</script>`),
    configureServer(server) {
      server.middlewares.use("/fonts", (req, res, next) => {
        const file = join(fonts, basename(req.url ?? ""));
        if (!existsSync(file)) return next();
        res.setHeader("Content-Type", "font/ttf");
        createReadStream(file).pipe(res);
      });
    },
  };
}

export default defineConfig({
  root: fileURLToPath(new URL(".", import.meta.url)),
  plugins: [react(), kit()],
  clearScreen: false,
  server: { port: 1460, strictPort: true },
});
