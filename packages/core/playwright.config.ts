import { defineConfig } from "@playwright/test";

const PORT = 1460;
const BASE_URL = `http://localhost:${PORT}`;

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  retries: 0,
  reporter: [["list"]],
  outputDir: "test-results",
  timeout: 60_000,
  expect: { timeout: 5_000 },

  use: {
    baseURL: BASE_URL,
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1,
    locale: "en-GB",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },

  projects: [{ name: "webkit", use: { browserName: "webkit" } }],

  webServer: {
    command: "npx vite kit",
    url: BASE_URL,
    reuseExistingServer: true,
    timeout: 60_000,
    stdout: "ignore",
    stderr: "pipe",
  },
});
