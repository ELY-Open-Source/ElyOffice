import { expect, test } from "./fixtures.ts";

for (const [label, path] of [
  ["Command line", "/docs"],
  ["Changelog", "/changelog"],
] as const) {
  test(`the footer's ${label} opens its page in both languages`, async ({ page }) => {
    await page.addInitScript(() => Object.defineProperty(navigator, "language", { get: () => "en-US" }));
    await page.goto("/");
    const link = page.locator("footer").getByRole("link", { name: label, exact: true });
    await expect(link).toHaveAttribute("href", path);
    await link.click();
    await expect(page).toHaveURL(new RegExp(`${path}$`));
    // Judged on its own load, not the home page's.
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(String(e)));
    page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
    page.on("requestfailed", (r) => errors.push(`${r.url()} ${r.failure()?.errorText}`));
    await page.reload();
    await expect(page.getByRole("heading", { level: 1, name: label })).toBeVisible();
    // Its stylesheet applied: the page column is narrowed.
    expect(await page.locator("main").evaluate((m) => getComputedStyle(m).maxWidth)).not.toBe("none");
    await page.getByRole("link", { name: "中文" }).click();
    await expect(page.locator("#zh h1")).toBeInViewport();
    expect(errors).toEqual([]);
  });
}
