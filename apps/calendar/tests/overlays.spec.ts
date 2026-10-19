// The overlays and the settings screen: summoned, then dismissed. The thing to check is that each
// one arrives, takes the keyboard while it is up, and goes away in the right order.

import { expect, test, type Page } from "@playwright/test";
import { openApp, settle } from "./app";

async function openCalendars(page: Page) {
  await page.locator('button[title="Calendars"]').click();
}

async function openSettings(page: Page) {
  await page.keyboard.press("ControlOrMeta+,");
}

const settings = (page: Page) => page.getByRole("region", { name: "Settings" });

test("Jump to opens and closes on Escape, and takes the keyboard", async ({ page }) => {
  await openApp(page);
  await page.keyboard.press("m");
  const dialog = page.getByRole("dialog", { name: "Jump to" });
  await expect(dialog).toContainText(/Today/);

  // `d` is day view out there. In here it belongs to the panel, which does nothing with it.
  await page.keyboard.press("d");
  await settle(page);
  expect(await page.evaluate(() => document.documentElement.getAttribute("data-view"))).toBe("week");

  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(page.locator(".grid-event").first()).toBeVisible();
});

test("clicking the backdrop dismisses a panel", async ({ page }) => {
  await openApp(page);
  await page.keyboard.press("m");
  const dialog = page.getByRole("dialog", { name: "Jump to" });
  await expect(dialog).toBeVisible();

  await page.mouse.click(30, 500);
  await expect(dialog).toHaveCount(0);
});

test("Cmd+, gives Settings the stage, and Escape gives it back", async ({ page }) => {
  await openApp(page);
  await openSettings(page);
  await expect(settings(page)).toBeVisible();
  await expect(page.locator(".grid")).toHaveCount(0);

  await page.keyboard.press("d");
  await settle(page);
  expect(await page.evaluate(() => document.documentElement.getAttribute("data-view"))).toBe("week");

  await page.keyboard.press("Escape");
  await expect(settings(page)).toHaveCount(0);
  await expect(page.locator(".grid-event").first()).toBeVisible();
});

test("the rail moves between sections", async ({ page }) => {
  await openApp(page);
  await openSettings(page);
  await expect(page.locator(".settings-title")).toHaveText("Accounts");
  await expect(settings(page)).toContainText("Google, you@example.com");

  await page.locator(".settings-tab", { hasText: "Calendars" }).click();
  await expect(page.locator(".settings-title")).toHaveText("Calendars");
  await page.locator(".settings-tab", { hasText: "General" }).click();
  await expect(settings(page)).toContainText(/Week view/);
});

test("adding an account asks for the provider, then for what that provider needs", async ({ page }) => {
  await openApp(page);
  await openSettings(page);
  await page.getByRole("button", { name: "Add account" }).click();

  const sheet = page.getByRole("dialog", { name: "Add account" });
  await expect(sheet.getByRole("combobox")).toHaveValue("google");
  await sheet.getByRole("combobox").selectOption("fastmail");
  await sheet.getByRole("button", { name: "Continue" }).click();

  const login = page.getByRole("dialog", { name: "Fastmail" });
  await expect(login).toContainText("Fastmail address");
  await expect(login.getByText("Server", { exact: true })).toHaveCount(0);
  await login.locator('input[type="text"]').fill("sam@fastmail.test");
  await login.locator('input[type="password"]').fill("app-password");
  await login.getByRole("button", { name: "Sign in" }).click();

  await expect(login).toHaveCount(0);
  await expect(settings(page)).toContainText("Fastmail, sam@fastmail.test");
});

test("a custom server asks for its address", async ({ page }) => {
  await openApp(page);
  await openSettings(page);
  await page.getByRole("button", { name: "Add account" }).click();
  const sheet = page.getByRole("dialog", { name: "Add account" });
  await sheet.getByRole("combobox").selectOption("caldav");
  await sheet.getByRole("button", { name: "Continue" }).click();
  await expect(page.getByRole("dialog", { name: "CalDAV server" }).getByText("Server", { exact: true })).toBeVisible();
});

test("Escape unwinds one layer at a time", async ({ page }) => {
  await openApp(page);
  await openSettings(page);
  await page.getByRole("button", { name: "Remove an account" }).click();

  const sheet = page.getByRole("dialog", { name: "Remove an account" });
  await expect(sheet).toBeVisible();
  await sheet.getByRole("button", { name: "Remove" }).first().click();
  const confirm = page.locator(".confirm");
  await expect(confirm).toContainText("Remove Google, you@example.com?");

  await page.keyboard.press("Escape");
  await expect(confirm).toHaveCount(0);
  await expect(sheet).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(sheet).toHaveCount(0);
  await expect(settings(page)).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(settings(page)).toHaveCount(0);
});

test("the mini month jumps the view to the day you pick", async ({ page }) => {
  await openApp(page);
  const range = await page.locator(".header-range").textContent();

  await page.keyboard.press("m");
  await expect(page.getByRole("dialog", { name: "Jump to" })).toBeVisible();
  // A day in the same month that is not the one already showing.
  await page.locator(".mini-day:not([data-outside]):not([data-span])").last().click();

  await expect(page.getByRole("dialog", { name: "Jump to" })).toHaveCount(0);
  expect(await page.locator(".header-range").textContent()).not.toBe(range);
  await expect(page.locator(".grid-event").first()).toBeVisible();
});

test("turning a calendar off takes its events off the grid", async ({ page }) => {
  await openApp(page);
  const meetup = page.locator(".grid-event", { hasText: "Antithesis" });
  expect(await meetup.count()).toBeGreaterThan(0);

  await openCalendars(page);
  await page
    .locator(".cal-pop .cal-row", { hasText: "Antithesis x Bengaluru Systems Meetup" })
    .locator("input[type=checkbox]")
    .uncheck();
  await page.keyboard.press("Escape");

  await expect(meetup).toHaveCount(0);
});

test("the theme switch actually repaints", async ({ page }) => {
  await openApp(page);
  const paper = () =>
    page.evaluate(() => getComputedStyle(document.querySelector(".grid")!).backgroundColor);
  const light = await paper();

  await openSettings(page);
  await page.locator(".settings-tab", { hasText: "General" }).click();
  await page.getByRole("tab", { name: "Dark", exact: true }).click();
  await page.keyboard.press("Escape");
  await settle(page);

  expect(await page.evaluate(() => document.documentElement.getAttribute("data-theme"))).toBe(
    "dark",
  );
  expect(await paper()).not.toBe(light);
});

test("a url in a description is a link that asks before it opens", async ({ page }) => {
  await openApp(page, { view: "day" });
  await page.locator(".grid-event", { hasText: "Antithesis" }).first().click();
  await expect(page.locator(".details-card[data-placed]")).toBeVisible();

  const link = page.locator(".details-desc a", { hasText: "https://luma.com/join/meetup" });
  await expect(link).toHaveText("https://luma.com/join/meetup");
  await expect(link).toHaveCSS("text-decoration-line", "underline");

  await page.evaluate(() => {
    const opened: string[] = [];
    (window as unknown as { opened: string[] }).opened = opened;
    window.open = (url) => {
      opened.push(String(url));
      return null;
    };
  });
  const opened = () => page.evaluate(() => (window as unknown as { opened: string[] }).opened);

  await link.click();
  const prompt = page.getByRole("alertdialog");
  await expect(prompt).toContainText("Do you want to open this page?");
  await expect(page.locator(".details-card")).toBeVisible();

  await prompt.getByRole("button", { name: "Cancel" }).click();
  await expect(prompt).toHaveCount(0);
  await expect(page.locator(".details-card")).toBeVisible();
  expect(await opened()).toEqual([]);

  await link.click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Open" }).click();
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  expect(await opened()).toEqual(["https://luma.com/join/meetup"]);
});
