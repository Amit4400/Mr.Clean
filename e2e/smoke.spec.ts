import { expect, test, type Page } from "@playwright/test";

// Every page must render without console errors, and the main flows must work
// end to end against the mock backend.

function watchErrors(page: Page) {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  return errors;
}

// Sidebar buttons only (the Clean page also has a "Clean" button).
const nav = (page: Page, name: string) => page.locator("aside").getByRole("button", { name, exact: true }).click();

test("overview scans everything and shows what it found", async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto("/");
  await expect(page.getByText("What we found")).toBeVisible();
  await expect(page.getByText("MacBook Pro")).toBeVisible();
  await expect(page.getByText(/used of 1 TB/)).toBeVisible();
  await page.getByRole("button", { name: /Scan everything/ }).click();
  await expect(page.getByRole("button", { name: /Scan again/ })).toBeVisible({ timeout: 10_000 });
  await expect(page.getByText("Quick actions")).toBeVisible();
  await expect(page.getByText(/needs attention|in good shape|little care/)).toBeVisible();
  await expect(page.getByText("4 issues")).toBeVisible();
  expect(errors).toEqual([]);
});

test("dev cleaner pre-selects only safe items and confirms before cleaning", async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto("/");
  await nav(page, "Clean");
  await page.getByRole("button", { name: "Scan", exact: true }).click();
  await expect(page.getByText("iOS simulators", { exact: true })).toBeVisible({ timeout: 10_000 });

  // Review items are never pre-selected; safe ones are.
  await expect(page.getByRole("checkbox", { name: "iOS simulators" })).not.toBeChecked();
  await expect(page.getByRole("checkbox", { name: "npm cache" })).toBeChecked();
  // Info-only items can't be selected at all.
  await expect(page.getByRole("checkbox", { name: "Docker disk image" })).toBeDisabled();

  await page.getByRole("main").getByRole("button", { name: "Clean", exact: true }).click();
  await expect(page.getByRole("button", { name: "Move to Trash" })).toBeVisible();
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(page.getByRole("button", { name: "Move to Trash" })).toBeHidden();
  expect(errors).toEqual([]);
});

test("large files: protected folders can't be selected", async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto("/");
  await nav(page, "Files");
  await page.getByRole("button", { name: "Scan home folder" }).click();
  await expect(page.getByText(/in [\d,]+ files/)).toBeVisible({ timeout: 10_000 });
  await expect(page.getByRole("checkbox", { name: "Documents" })).toBeDisabled();
  await expect(page.getByRole("checkbox", { name: "notes.txt" })).toBeEnabled();
  await page.getByRole("radio", { name: "Biggest files" }).click();
  await expect(page.getByText("Xcode_15.4.xip", { exact: true })).toBeVisible();
  expect(errors).toEqual([]);
});

test("security findings render with severity and quarantine", async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto("/");
  await nav(page, "Security");
  await page.getByRole("button", { name: "Scan", exact: true }).click();
  await expect(page.getByText(/need your attention/)).toBeVisible({ timeout: 10_000 });
  await expect(page.getByText("Suspicious startup item: com.apple.sysupdate")).toBeVisible();
  await expect(page.getByRole("button", { name: "Quarantine" }).first()).toBeVisible();
  expect(errors).toEqual([]);
});

test("memory page lists dev leftovers and settings persist delete mode", async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto("/");
  await nav(page, "Memory");
  await expect(page.getByText("Gradle daemon", { exact: true })).toBeVisible({ timeout: 10_000 });
  await expect(page.getByText("Protected").first()).toBeVisible();
  await expect(page.getByText("Compressed")).toBeVisible();
  await expect(page.getByText(/Last 10 minutes/)).toBeVisible();

  await nav(page, "Settings");
  await page.getByRole("button", { name: /Delete permanently/ }).click();
  await page.reload();
  await nav(page, "Clean");
  await page.getByRole("button", { name: "Scan", exact: true }).click();
  await expect(page.getByText(/items are deleted permanently/)).toBeVisible({ timeout: 10_000 });
  expect(errors).toEqual([]);
});
