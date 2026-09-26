import { defineConfig, devices } from "@playwright/test";

// Runs the built UI in Chromium against the mock backend (no Tauri needed).
export default defineConfig({
  testDir: "e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: process.env.CI ? [["github"], ["list"]] : "list",
  use: {
    baseURL: "http://localhost:4173",
    trace: "retain-on-failure",
    viewport: { width: 1200, height: 800 },
    launchOptions: process.env.PW_CHROMIUM ? { executablePath: process.env.PW_CHROMIUM } : {},
  },
  projects: [
    { name: "light", use: { ...devices["Desktop Chrome"], colorScheme: "light", viewport: { width: 1200, height: 800 } } },
    { name: "dark", use: { ...devices["Desktop Chrome"], colorScheme: "dark", viewport: { width: 1200, height: 800 } } },
  ],
  webServer: {
    command: "npm run build && npx vite preview --port 4173 --strictPort",
    url: "http://localhost:4173",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
