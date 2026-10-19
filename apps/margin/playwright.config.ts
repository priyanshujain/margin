import { defineConfig } from "@playwright/test";

const PORT = 1420;
const BASE_URL = `http://localhost:${PORT}`;

export default defineConfig({
  testDir: "./tests",
  tsconfig: "./tsconfig.playwright.json",
  fullyParallel: true,
  workers: 2,
  retries: 0,
  reporter: [["list"]],
  outputDir: "node_modules/.cache/playwright",
  timeout: 30_000,
  expect: { timeout: 5_000 },

  use: {
    baseURL: BASE_URL,
    viewport: { width: 1280, height: 820 },
    deviceScaleFactor: 1,
    locale: "en-GB",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },

  projects: [{ name: "chromium", use: { browserName: "chromium" } }],

  webServer: {
    command: "node_modules/.bin/vite",
    url: BASE_URL,
    reuseExistingServer: false,
    timeout: 60_000,
    stdout: "ignore",
    stderr: "pipe",
  },
});
