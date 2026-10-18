#!/usr/bin/env node
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { chromium } from "@playwright/test";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = join(root, "appstore", "screenshots");
const browser = await chromium.launch();

try {
  const page = await browser.newPage({ viewport: { width: 2560, height: 1600 }, deviceScaleFactor: 1 });
  for (const name of ["inbox-light", "screener", "feed", "inbox-dark"]) {
    await page.goto(pathToFileURL(join(output, "src", `${name}.html`)).href);
    await page.evaluate(async () => {
      await document.fonts.ready;
      if (!document.fonts.check('32px "Hanken Grotesk"') || !document.fonts.check('92px "Literata"')) {
        throw new Error("Screenshot fonts failed to load");
      }
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    });
    await page.screenshot({ path: join(output, `${name}.png`) });
    console.log(`rendered ${name}.png (2560x1600)`);
  }
} finally {
  await browser.close();
}
