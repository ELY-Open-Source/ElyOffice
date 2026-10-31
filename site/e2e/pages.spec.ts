import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures.ts";

const PAGES = [
  ["/docs", "Command line", "命令行"],
  ["/changelog", "Changelog", "更新日志"],
] as const;

/** The browser's own language, as the page reads it. */
const speaks = (page: Page, language: string) =>
  page.addInitScript((l) => Object.defineProperty(navigator, "language", { get: () => l }), language);

/** Only the article in `lang` shows, titled and tagged so. */
async function shows(page: Page, lang: "en" | "zh", en: string, zh: string) {
  const [shown, hidden] = lang === "en" ? [en, zh] : [zh, en];
  await expect(page.getByRole("heading", { level: 1, name: shown, exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { level: 1, name: hidden, exact: true })).toBeHidden();
  await expect(page).toHaveTitle(`${shown} · Ely Office`);
  await expect(page.locator("html")).toHaveAttribute("lang", lang === "en" ? "en" : "zh-CN");
  await expect(page.locator("nav button")).toHaveText(lang === "en" ? "中文" : "English");
}

for (const [path, en, zh] of PAGES) {
  for (const where of [".hero", "footer"]) {
    test(`${where}'s ${en} opens its page in the site's language`, async ({ page }) => {
      await speaks(page, "en-US");
      await page.goto("/");
      await page.locator(".lang").click();
      const link = page.locator(where).getByRole("link", { name: zh, exact: true });
      await expect(link).toHaveAttribute("href", `${path}?lang=zh`);
      await link.click();
      await expect(page).toHaveURL(new RegExp(`${path}\\?lang=zh$`));
      // Judged on its own load, not the home page's.
      const errors: string[] = [];
      page.on("pageerror", (e) => errors.push(String(e)));
      page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
      page.on("requestfailed", (r) => errors.push(`${r.url()} ${r.failure()?.errorText}`));
      await page.reload();
      await shows(page, "zh", en, zh);
      // Its stylesheet applied: the page column is narrowed.
      expect(await page.locator("main").evaluate((m) => getComputedStyle(m).maxWidth)).not.toBe("none");
      await page.locator("nav button").click();
      await shows(page, "en", en, zh);
      await expect(page).toHaveURL(new RegExp(`${path}\\?lang=en$`));
      expect(errors).toEqual([]);
    });
  }

  test(`${en} opened directly follows the browser's language`, async ({ page }) => {
    await speaks(page, "zh-CN");
    await page.goto(path);
    await shows(page, "zh", en, zh);
    await page.goto(`${path}?lang=en`);
    await shows(page, "en", en, zh);
  });
}
