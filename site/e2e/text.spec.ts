import { DOWNLOADS, expect, INSTALLERS, SOURCES, test, VERSION, type Page } from "./fixtures.ts";
import { TEXT, word, type Key, type Lang } from "../src/text.ts";

const REPO = "https://github.com/ELY-Open-Source/ElyOffice";
const APPS = ["ElyDoc", "ElyTable", "ElyShow", "ElyPDF", "ElyMail", "ElyTree", "ElyMark"];
const RELEASES = `${REPO}/releases/latest`;
const X = "https://x.com/ZacharyZhang";

test.use({ reducedMotion: "reduce" });

let errors: string[] = [];

test.beforeEach(async ({ page }) => {
  errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
});

test.afterEach(() => expect(errors).toEqual([]));

/** Each keyed element's key and text. */
const keyed = (page: Page) =>
  page.locator("[data-t]").evaluateAll((els) => els.map((e) => [(e as HTMLElement).dataset.t!, e.textContent!] as const));

/** Asserts every keyed element reads in `lang`. */
async function reads(page: Page, lang: Lang) {
  await expect(page.locator("html")).toHaveAttribute("lang", lang === "zh" ? "zh-CN" : "en");
  await expect(page.locator("[data-t=count]")).toHaveCount(2);
  for (const c of await page.locator("[data-t=count]").all()) await expect(c).toBeVisible();
  const found = await keyed(page);
  expect(found.length).toBeGreaterThan(0);
  for (const [key, text] of found) {
    expect(text, key).not.toBe("");
    expect(text, key).toBe(word(lang, key as Key, key === "count" ? DOWNLOADS : undefined));
  }
}

test.describe("Chinese locale", () => {
  test.use({ locale: "zh-CN" });

  test("opens in Chinese", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
    await expect(page.locator("[data-t=tagline]")).toHaveText(TEXT.zh.tagline);
    await reads(page, "zh");
  });
});

test.describe("English locale", () => {
  test.use({ locale: "en-US" });

  test("opens in English", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("[data-t=tagline]")).toHaveText(TEXT.en.tagline);
    await reads(page, "en");
  });

  test("the language button hides until intent", async ({ page }) => {
    await page.goto("/");
    const lang = page.locator(".lang");
    await expect(lang).toHaveCSS("opacity", "0");
    await lang.hover();
    await expect(lang).toHaveCSS("opacity", "1");
    await page.evaluate(() => scrollTo(0, document.body.scrollHeight));
    await page.mouse.move(5, 500);
    await expect(lang).toHaveCSS("opacity", "0");
    await page.keyboard.press("Tab");
    await page.keyboard.press("Tab");
    await expect(lang).toBeFocused();
    await expect(lang).toHaveCSS("opacity", "1");
  });

  test("the hero links keep their icons in either language", async ({ page }) => {
    await page.goto("/");
    const icons = page.locator(".hero :is(a.button, .links a.icon) > svg[aria-hidden=true]");
    await expect(icons).toHaveCount(2);
    await page.locator(".lang").click();
    await expect(icons).toHaveCount(2);
    await expect(page.locator(".hero a.button")).toHaveText("下载 Windows 版");
    await expect(page.locator(".hero .links a")).toHaveText(["全部平台", "命令行", "更新日志", "源码"]);
  });

  test("the language button switches every text and back", async ({ page }) => {
    await page.goto("/");
    const lang = page.locator(".lang");
    await expect(lang).toHaveText("中文");
    await lang.click();
    await reads(page, "zh");
    await expect(lang).toHaveText("English");
    await lang.click();
    await reads(page, "en");
    await expect(lang).toHaveText("中文");
  });

  test("the buttons download the full installer; single apps stay on the release", async ({ page }) => {
    await page.goto("/");
    for (const button of await page.locator("a.button").all()) await expect(button).toHaveAttribute("href", INSTALLERS.windows);
    await expect(page.locator("a.button")).toHaveCount(2);
    const cards = page.locator("#download a.card");
    await expect(cards).toHaveText(["Mac, Apple siliconFull installer", "Mac, IntelFull installer", "WindowsFull installer", "LinuxPackages on GitHub"]);
    const hrefs = [INSTALLERS.macArm, INSTALLERS.macIntel, INSTALLERS.windows, RELEASES];
    for (const [i, href] of hrefs.entries()) await expect(cards.nth(i)).toHaveAttribute("href", href);
    for (const a of await page.getByRole("link", { name: "GitHub Releases" }).all()) await expect(a).toHaveAttribute("href", RELEASES);
    await expect(page.getByRole("link", { name: "GitHub Releases" })).toHaveCount(2);
    await expect(page.locator(".tour .get")).toHaveCount(APPS.length);
    for (const i of APPS.keys()) await expect(page.locator(".tour .get").nth(i)).toHaveAttribute("href", RELEASES);
    // Source is this release's archive, never a moving branch.
    await expect(page.locator(".hero .links a").last()).toHaveAttribute("href", SOURCES.own);
    await expect(page.locator("#download .sources a")).toHaveText(["Source", "Upstream source"]);
    for (const [i, href] of [SOURCES.own, SOURCES.upstream].entries())
      await expect(page.locator("#download .sources a").nth(i)).toHaveAttribute("href", href);
    await expect(page.locator(".hero .links a").first()).toHaveAttribute("href", "#download");
  });

  test("the hero asks for a follow on X and a star", async ({ page }) => {
    await page.goto("/");
    const like = page.locator(".hero .like");
    await expect(like).toHaveText("Like it? Follow @ZacharyZhang on X and star the project on GitHub.");
    await expect(like.getByRole("link", { name: "@ZacharyZhang" })).toHaveAttribute("href", X);
    await expect(like.getByRole("link", { name: "star the project" })).toHaveAttribute("href", REPO);
    await page.locator(".lang").click();
    await expect(like).toHaveText("喜欢的话，欢迎在 X 关注 @ZacharyZhang，并给项目点个 Star。");
  });

  test("a release still building sends its missing installers to the release", async ({ page }) => {
    const json = { version: VERSION, installers: { macArm: INSTALLERS.macArm }, sources: { own: SOURCES.own }, downloads: DOWNLOADS };
    await page.route("**/release.json", (r) => r.fulfill({ json }));
    await page.goto("/");
    await expect(page.locator("#download h2")).toHaveText(`Ely Office ${VERSION}`);
    await expect(page.locator(".hero a.button")).toHaveAttribute("href", RELEASES);
    await expect(page.locator("#download a.card").first()).toHaveAttribute("href", INSTALLERS.macArm);
    await expect(page.locator("#download .sources a").first()).toHaveAttribute("href", SOURCES.own);
    await expect(page.locator("#download .sources a").last()).toHaveAttribute("href", `${REPO}/releases/tag/v${VERSION}`);
  });

  test("the hero and download area name the newest version and all downloads", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("#download h2")).toHaveText(`Ely Office ${VERSION}`);
    const counts = page.locator("[data-t=count]");
    await expect(counts).toHaveText(["12,345 downloads so far.", "12,345 downloads so far."]);
    await page.locator(".lang").click();
    await expect(counts).toHaveText(["已下载 12,345 次。", "已下载 12,345 次。"]);
  });

  test("a failed version read shows as an error", async ({ page }) => {
    await page.route("**/release.json", (r) => r.fulfill({ status: 502, body: "GitHub answered 500" }));
    await page.goto("/");
    await expect.poll(() => errors.length).toBe(1);
    expect(errors[0]).toContain("GitHub answered 500");
    await expect(page.locator("#download h2")).toHaveText("Ely Office");
    for (const c of await page.locator("[data-t=count]").all()) await expect(c).toBeHidden();
    errors = [];
  });

  test("every external link answers @net", async ({ page, request }) => {
    const res = await request.get("https://api.github.com/repos/ELY-Open-Source/ElyOffice/releases/latest");
    const { tag_name: tag, assets } = (await res.json()) as { tag_name: string; assets: { name: string }[] };
    const v = tag.slice(1);
    const dl = `${REPO}/releases/download/${tag}`;
    const installers = { macArm: `${dl}/ElyOffice-${v}-arm64.dmg`, macIntel: `${dl}/ElyOffice-${v}-x86_64.dmg`, windows: `${dl}/ElyOffice-${v}-windows-x86_64-setup.exe` };
    const named = (end: string) => `${dl}/${assets.find((a) => a.name.endsWith(end) && !a.name.slice(0, -end.length).endsWith("-upstream"))!.name}`;
    const sources = { own: named("-source.tar.gz"), upstream: named("-upstream-source.tar.gz") };
    await page.route("**/release.json", (r) => r.fulfill({ json: { version: v, installers, sources, downloads: DOWNLOADS } }));
    await page.goto("/");
    await expect(page.locator(".hero a.button")).toHaveAttribute("href", installers.windows);
    const hrefs = await page.locator("a[href^='http']").evaluateAll((as) => as.map((a) => (a as HTMLAnchorElement).href));
    const unique = [...new Set(hrefs)];
    expect(unique.sort()).toEqual([REPO, RELEASES, X, ...Object.values(installers), ...Object.values(sources)].sort());
    for (const url of unique) {
      // X refuses HEAD and browser-like clients; a plain GET answers.
      const res = url === X ? await request.get(url, { headers: { "user-agent": "curl/8" } }) : await request.head(url);
      expect(res.status(), url).toBeLessThan(400);
    }
  });

  test("each app's copy arrives at its anchor; its download shows on intent", async ({ page }) => {
    for (const [i, app] of APPS.entries()) {
      await page.goto(`/#${app.toLowerCase()}`);
      const article = page.locator(".copy article").nth(i);
      await expect(article).toHaveClass(/current/);
      await expect(page.locator(".copy article.current")).toHaveCount(1);
      await expect(page.locator(".rail a").nth(i)).toHaveAttribute("aria-current", "step");
      const get = article.locator(".get");
      await page.mouse.move(1000, 5);
      await expect(get).toHaveCSS("opacity", "0");
      await article.locator("h2").hover();
      await expect(get).toHaveCSS("opacity", "1");
      await page.mouse.move(1000, 5);
      await expect(get).toHaveCSS("opacity", "0");
      await get.focus();
      await expect(get).toHaveCSS("opacity", "1");
      await get.blur();
    }
  });
});

test("the hero links share one baseline in either language", async ({ browser }) => {
  for (const locale of ["en-US", "zh-CN"]) {
    const page = await browser.newPage({ locale, reducedMotion: "reduce" });
    await page.goto("/");
    const lines = await page.locator(".hero .links a").evaluateAll((as) =>
      as.map((a) => {
        const mark = document.createElement("span");
        mark.style.display = "inline-block";
        a.prepend(mark);
        const y = mark.getBoundingClientRect().bottom;
        mark.remove();
        return y;
      }),
    );
    expect(lines).toHaveLength(4);
    for (const y of lines) expect(Math.abs(y - lines[0]), locale).toBeLessThan(0.5);
    await page.close();
  }
});

const MAC = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0 Safari/537.36";
const LINUX = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0 Safari/537.36";
// The chip hint Chromium gives; Safari and Firefox give none.
for (const [name, ua, chip, button, href] of [
  ["a Mac without a chip hint", MAC, null, "Download for Mac (Apple silicon)", INSTALLERS.macArm],
  ["an Apple silicon Mac", MAC, "arm", "Download for Mac (Apple silicon)", INSTALLERS.macArm],
  ["an Intel Mac", MAC, "x86", "Download for Mac (Intel)", INSTALLERS.macIntel],
  ["Linux", LINUX, null, "Download for Linux", RELEASES],
] as const) {
  test.describe(`on ${name}`, () => {
    test.use({ userAgent: ua, locale: "en-US" });

    test("the button offers this platform", async ({ page }) => {
      await page.addInitScript((arch) => {
        const hints = arch && { platform: "macOS", getHighEntropyValues: async () => ({ architecture: arch }) };
        Object.defineProperty(Navigator.prototype, "userAgentData", { get: () => hints || undefined });
      }, chip);
      await page.goto("/");
      await expect(page.locator("#download h2")).toHaveText(`Ely Office ${VERSION}`);
      await expect(page.locator(".hero a.button")).toHaveText(button);
      await expect(page.locator(".hero a.button")).toHaveAttribute("href", href);
      await expect(page.locator(".hero [data-t=whole]")).toBeVisible({ visible: href !== RELEASES });
    });
  });
}
