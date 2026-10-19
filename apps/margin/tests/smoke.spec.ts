// The app in a browser against the dev IPC mock in src/dev/mockIpc.ts: no Tauri, no disk, no network.

import { expect, test, type Page } from "@playwright/test";

const MOD = process.platform === "darwin" ? "Meta" : "Control";

async function launch(page: Page, storage: Record<string, string> = {}): Promise<void> {
  await page.addInitScript((seed) => {
    if (sessionStorage.getItem("seeded")) return;
    localStorage.clear();
    for (const [key, value] of Object.entries(seed)) localStorage.setItem(key, value);
    sessionStorage.setItem("seeded", "1");
  }, storage);
  await page.goto("/");
}

async function openFieldNotes(page: Page): Promise<void> {
  await launch(page);
  await page.locator(".card-book", { hasText: "Field Notes" }).click();
  await expect(page.locator(".prose")).toContainText("The river was quiet");
}

test("the library lists the books and opens one", async ({ page }) => {
  await launch(page);
  await expect(page.locator(".card-book")).toHaveCount(1);
  await expect(page.locator(".card-book")).toContainText("Ada Byron");
  await page.locator(".card-book").click();
  await expect(page.locator(".doc-title")).toContainText("Field Notes");
  await expect(page.locator(".prose strong")).toHaveText("bold");
});

test("typing marks the book dirty and the autosave writes it", async ({ page }) => {
  await openFieldNotes(page);
  await page.locator(".prose p").first().click();
  await page.keyboard.press("End");
  await page.keyboard.type(" Then the heron.");
  await expect
    .poll(() => page.evaluate(() => window.__marginMock?.saves.length ?? 0), { timeout: 5_000 })
    .toBeGreaterThan(0);
  const saved = await page.evaluate(() => JSON.parse(window.__marginMock!.files.get("field-notes")!));
  expect(JSON.stringify(saved.chapters[0].content)).toContain("Then the heron.");
  await expect(page.locator(".dirty-dot")).toHaveCount(0);
});

test("the palette opens with both chords and runs a command", async ({ page }) => {
  await launch(page, { "margin-theme": "light" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.keyboard.press(`${MOD}+k`);
  await expect(page.locator(".palette input")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(page.locator(".palette")).toHaveCount(0);
  await page.keyboard.press(`${MOD}+Shift+p`);
  await page.keyboard.type("dark mode");
  await page.keyboard.press("Enter");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  expect(await page.evaluate(() => localStorage.getItem("margin-theme"))).toBe("dark");
});

test("the palette offers the book's commands once a book is open", async ({ page }) => {
  await openFieldNotes(page);
  await page.keyboard.press(`${MOD}+k`);
  await expect(page.locator(".palette")).toContainText("Export EPUB");
  await page.keyboard.type("next chapter");
  await page.keyboard.press("Enter");
  await expect(page.locator(".chapter-title-input")).toHaveValue("Evening");
});

test("an old stored theme still applies before paint", async ({ page }) => {
  await launch(page, { "margin-theme": "dark" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
});

test("settings is a full screen with a way back", async ({ page }) => {
  await openFieldNotes(page);
  await page.keyboard.press(`${MOD}+,`);
  await expect(page.locator(".settings")).toBeVisible();
  await expect(page.locator(".app")).toBeHidden();
  await page.locator(".settings-tab", { hasText: "Appearance" }).click();
  await page.getByRole("tab", { name: "Dark" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.locator(".settings-back", { hasText: "Back to Margin" }).click();
  await expect(page.locator(".settings")).toHaveCount(0);
  await expect(page.locator(".prose")).toBeVisible();
});

test("adding Google Drive waits for the browser and closing cancels the sign-in", async ({ page }) => {
  await launch(page);
  await page.keyboard.press(`${MOD}+,`);
  await page.locator(".settings-tab", { hasText: "Backup" }).click();
  await page.getByRole("button", { name: "Add account" }).click();
  await expect(page.locator("select")).toHaveValue("gdrive");
  await page.getByRole("button", { name: "Continue" }).click();
  await expect(page.getByText("Waiting for Google in your browser")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByText("Waiting for Google in your browser")).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => window.__marginMock!.cancelled())).toBe(1);
  await expect(page.locator(".settings")).toBeVisible();

  await page.getByRole("button", { name: "Add account" }).click();
  await page.getByRole("button", { name: "Continue" }).click();
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect.poll(() => page.evaluate(() => window.__marginMock!.cancelled())).toBe(2);
});

test("a misspelling is underlined and can be corrected", async ({ page }) => {
  await openFieldNotes(page);
  const mark = page.locator(".prose .proof-mark", { hasText: "teh" });
  await expect(mark).toBeVisible();
  await mark.click();
  await page.locator(".proof-suggestion", { hasText: /^the$/ }).click();
  await expect(page.locator(".prose")).toContainText("and the birds");
});

test("the shortcuts sheet lists Margin's own commands", async ({ page }) => {
  await openFieldNotes(page);
  await page.locator(".prose p").first().click();
  await page.keyboard.press(`${MOD}+/`);
  await expect(page.locator(".shortcuts")).toContainText("Export EPUB");
  await expect(page.locator(".shortcuts")).toContainText("Next chapter");
});
