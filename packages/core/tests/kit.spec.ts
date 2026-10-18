import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test, type Page } from "@playwright/test";
import { THEMES } from "../src/hooks/theme";

const root = fileURLToPath(new URL("..", import.meta.url));
const shots = join(root, "kit", "screenshots");

const HEX = /#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b/;

const SURFACES = ["--paper", "--shell", "--raised", "--sidebar"];
const TEXT = ["--ink", "--ink-soft", "--ink-faint", "--accent-ink", "--danger-ink"];
const ON_FILL: [string, string][] = [
  ["--accent-contrast", "--accent"],
  ["--danger-contrast", "--danger"],
  ["--ink", "--note-surface"],
  ["--ink-soft", "--note-surface"],
  ...[1, 2, 3, 4, 5, 6, 7, 8].map((n): [string, string] => ["--accent-contrast", `--hue-${n}`]),
];
const AA = 4.5;
const NON_TEXT = 3;
const GLYPHS: [string, string][] = [["--check-tick", "--check"]];


async function settle(page: Page) {
  await page.evaluate(
    () =>
      new Promise<void>((done) => {
        window.scrollTo(0, 0);
        requestAnimationFrame(() => requestAnimationFrame(() => done()));
      }),
  );
}

async function openKit(page: Page, theme: string) {
  await page.addInitScript((id) => localStorage.setItem("kit-theme", id), theme);
  await page.goto("/");
  await expect(page.locator(".kit")).toBeVisible();
  await page.evaluate(() => document.fonts.ready);
  await settle(page);
}

function files(dir: string, ext: string): string[] {
  return readdirSync(join(root, dir))
    .filter((f) => f.endsWith(ext))
    .map((f) => join(dir, f));
}

for (const info of THEMES) {
  test(`the kit renders in ${info.id}`, async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(String(e)));
    await openKit(page, info.id);
    await expect(page.locator("html")).toHaveAttribute("data-theme", info.id);
    mkdirSync(shots, { recursive: true });
    await page.screenshot({ path: join(shots, `kit-${info.id}.png`), fullPage: true });
    expect(errors).toEqual([]);
  });
}

test("the primitives with a fixed geometry have it", async ({ page }) => {
  await openKit(page, "light");
  const row = page.locator(".row").first();
  expect((await row.boundingBox())?.height).toBe(46);
  const box = await row.locator(".avatar").boundingBox();
  expect(box?.width).toBe(30);
  expect(box?.height).toBe(30);
  const noted = page.locator(".row[data-noted]").first();
  expect((await noted.boundingBox())?.height).toBeGreaterThan(46);
  expect((await page.locator(".kit-stage .titlebar").boundingBox())?.height).toBe(46);
});

test("the palette opens on Cmd+K and on Cmd+Shift+P, and ? opens the sheet", async ({ page }) => {
  await openKit(page, "light");
  const live = page.locator('.kit > .overlay[data-align="top"]');
  await page.keyboard.press("Meta+k");
  await expect(live).toBeVisible();
  await expect(live.locator(".palette-row").first()).toContainText("Quick open");
  await page.keyboard.press("Escape");
  await expect(live).toHaveCount(0);
  await page.keyboard.press("Meta+Shift+p");
  await expect(live).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(live).toHaveCount(0);
  await page.keyboard.press("Shift+?");
  await expect(page.locator('.kit > .overlay .sheet[aria-label="Keyboard shortcuts"]')).toBeVisible();
});

test("every palette meets WCAG AA for its text token pairs", async ({ page }) => {
  await openKit(page, "light");
  const rows: { pair: string; ratio: number }[] = [];
  for (const info of THEMES) {
    const pairs: [string, string][] = [
      ...TEXT.flatMap((text) => SURFACES.map((surface): [string, string] => [text, surface])),
      ...ON_FILL,
      ...GLYPHS,
    ];
    const ratios = await page.evaluate(
      ({ id, pairs }) => {
        document.documentElement.setAttribute("data-theme", id);
        const probe = document.createElement("canvas");
        probe.width = 1;
        probe.height = 1;
        const ctx = probe.getContext("2d", { willReadFrequently: true })!;
        const style = getComputedStyle(document.documentElement);
        const rgb = (token: string): [number, number, number] => {
          ctx.clearRect(0, 0, 1, 1);
          ctx.fillStyle = "#ffffff";
          ctx.fillRect(0, 0, 1, 1);
          ctx.fillStyle = style.getPropertyValue(token).trim();
          ctx.fillRect(0, 0, 1, 1);
          const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
          return [r, g, b];
        };
        const channel = (c: number) => {
          const s = c / 255;
          return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
        };
        const luminance = ([r, g, b]: [number, number, number]) =>
          0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
        return pairs.map(([text, surface]) => {
          const a = luminance(rgb(text));
          const b = luminance(rgb(surface));
          return Math.round(((Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)) * 100) / 100;
        });
      },
      { id: info.id, pairs },
    );
    pairs.forEach(([text, surface], i) => rows.push({ pair: `${info.id}: ${text} on ${surface}`, ratio: ratios[i] }));
  }

  mkdirSync(shots, { recursive: true });
  writeFileSync(join(shots, "contrast.json"), JSON.stringify(rows, null, 2));
  const glyph = (pair: string) => GLYPHS.some(([a, b]) => pair.endsWith(`${a} on ${b}`));
  const failing = rows.filter((r) => r.ratio < (glyph(r.pair) ? NON_TEXT : AA));
  console.log(failing.map((r) => `${r.pair} ${r.ratio}`).join("\n"));
  expect(failing.map((r) => `${r.pair} ${r.ratio}`)).toEqual([]);
});

test("no stylesheet in the design system writes a colour", () => {
  const sheets = [...files("src/ui", ".css"), ...files("src/keys", ".css"), ...files("kit", ".css")];
  expect(sheets.length).toBeGreaterThan(10);
  const offenders: string[] = [];
  for (const file of sheets) {
    readFileSync(join(root, file), "utf8")
      .split("\n")
      .forEach((line, i) => {
        if (HEX.test(line)) offenders.push(`${file}:${i + 1}  ${line.trim()}`);
      });
  }
  expect(offenders, "a colour belongs in src/tokens.css as a token").toEqual([]);
});

test("every text field tells the webview not to fill it in", () => {
  const sources = [...files("src/ui", ".tsx"), ...files("src/keys", ".tsx"), ...files("kit", ".tsx")];
  const offenders: string[] = [];
  for (const file of sources) {
    const text = readFileSync(join(root, file), "utf8");
    for (const tag of text.match(/<input\b[\s\S]*?\n\s*\/>/g) ?? []) {
      if (/type="(?:file|checkbox|radio|range|color)"/.test(tag)) continue;
      if (/NO_AUTOFILL|autoComplete=/.test(tag)) continue;
      offenders.push(`${file}  ${tag.split("\n")[0].trim()}`);
    }
  }
  expect(offenders, "spread NO_AUTOFILL from src/ui onto it").toEqual([]);
});
