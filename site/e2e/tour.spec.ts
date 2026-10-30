import { expect, test, type Page } from "./fixtures.ts";

const APPS = ["ElyDoc", "ElyTable", "ElyShow", "ElyPDF", "ElyMail", "ElyTree", "ElyMark"];

let errors: string[] = [];

test.beforeEach(async ({ page }) => {
  errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  page.on("requestfailed", (r) => errors.push(`${r.url()} ${r.failure()?.errorText}`));
  page.on("response", (r) => r.status() >= 400 && errors.push(`${r.url()} ${r.status()}`));
});

test.afterEach(() => expect(errors).toEqual([]));

const frame = (page: Page) => page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));

/** Scrolls to `spans` stage heights into the tour. */
async function at(page: Page, spans: number) {
  await page.evaluate((s) => {
    document.documentElement.style.scrollBehavior = "auto";
    scrollTo(0, s * document.querySelector<HTMLElement>(".pin")!.clientHeight);
  }, spans);
  await frame(page);
}

/** Largest gap in px between clip and face corners. */
const drift = (page: Page, i: number) =>
  page.evaluate((i) => {
    const video = document.querySelectorAll<HTMLVideoElement>(".clip")[i];
    const world = video.parentElement!;
    const host = world.parentElement!.getBoundingClientRect();
    const m = new DOMMatrix(getComputedStyle(world).transform).multiply(new DOMMatrix(getComputedStyle(video).transform));
    const [w, h] = [video.offsetWidth, video.offsetHeight];
    const clip = [[0, 0], [w, 0], [w, h], [0, h]].map(([x, y]) => m.transformPoint(new DOMPoint(x, y)));
    const row = (k: number) => document.querySelector<SVGGElement>(`.block.slab[data-app="${i}"][data-row="${k}"]`)!;
    const ctm = row(0).ownerSVGElement!.getScreenCTM()!;
    const corners = (g: SVGGElement) =>
      g.querySelector(".face.left")!.getAttribute("points")!.split(" ").map((p) => {
        const [x, y] = p.split(",").map(Number);
        return new DOMPoint(x, y).matrixTransform(ctm);
      });
    // Left face points: bottom-left, bottom-right, top-right, top-left.
    const [bl, br] = corners(row(0));
    const [, , tr, tl] = corners(row(3));
    const face = [tl, tr, br, bl];
    return Math.max(...clip.map((p, k) => Math.hypot(p.x + host.left - face[k].x, p.y + host.top - face[k].y)));
  }, i);

test("each clip stays on its screen face, held or gliding", async ({ page }) => {
  await page.goto("/");
  for (const [i] of APPS.entries()) {
    for (const s of [i + 1, i + 1.5]) {
      await at(page, s);
      await page.waitForTimeout(800);
      expect(await drift(page, i), `${APPS[i]} at ${s}`).toBeLessThan(1);
    }
  }
});

test("only the current app's clip loads and plays", async ({ page }) => {
  await page.goto("/");
  await at(page, 3);
  const clips = page.locator(".clip");
  await expect.poll(() => clips.nth(2).evaluate((v: HTMLVideoElement) => !v.paused && v.currentTime > 0)).toBe(true);
  for (const i of [0, 1, 3, 4, 5, 6]) await expect(clips.nth(i).locator("source")).toHaveCount(0);
  await at(page, 4);
  await expect(clips.nth(2).locator("source")).toHaveCount(0);
  await expect.poll(() => clips.nth(3).evaluate((v: HTMLVideoElement) => !v.paused)).toBe(true);
  await at(page, 0);
  await expect(page.locator(".clip source")).toHaveCount(0);
});

test("the rail scrolls to an app and marks it", async ({ page }) => {
  await page.goto("/");
  await at(page, 1);
  await page.locator(".rail a").nth(5).click();
  await expect(page.locator(".rail a").nth(5)).toHaveAttribute("aria-current", "step");
  await expect(page.locator(".copy article.current h2")).toHaveText("ElyTree");
});

test.describe("reduced motion", () => {
  test.use({ reducedMotion: "reduce" });

  test("clips wait for a press; the camera jumps", async ({ page }) => {
    await page.goto("/#elyshow");
    const clip = page.locator(".clip").nth(2);
    await expect(page.locator(".copy article.current h2")).toHaveText("ElyShow");
    await expect(clip).toHaveAttribute("poster", "/clips/show.jpg");
    expect(await clip.evaluate((v: HTMLVideoElement) => v.paused)).toBe(true);
    const held = await page.evaluate(() => getComputedStyle(document.querySelector(".world")!).transform);
    await at(page, 3.4);
    expect(await page.evaluate(() => getComputedStyle(document.querySelector(".world")!).transform)).toBe(held);
    await page.locator(".copy article.current .toggle-clip").click();
    await expect.poll(() => clip.evaluate((v: HTMLVideoElement) => !v.paused)).toBe(true);
    await expect(page.locator(".copy article.current .toggle-clip")).toHaveText("Pause");
  });
});

test.describe("phone", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test("no sideways scroll; copy sits below the stage", async ({ page }) => {
    await page.goto("/");
    for (const s of [0, 1, 2.5, 4, 6, 8]) {
      await at(page, s);
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(390);
    }
    await at(page, 2);
    const copy = await page.locator(".copy article.current").boundingBox();
    expect(copy!.y).toBeGreaterThan(844 * 0.55);
  });
});

test.describe("reduced motion, every screen", () => {
  test.use({ reducedMotion: "reduce" });

  test("each app's poster is a real picture that decodes", async ({ page }) => {
    for (const [i, app] of APPS.entries()) {
      await page.goto(`/#${app.toLowerCase()}`);
      await expect(page.locator(".copy article").nth(i)).toHaveClass(/current/);
      // A missing file comes back as HTML: decode it.
      const size = await page.locator(".clip").nth(i).evaluate(async (video) => {
        const poster = (video as HTMLVideoElement).poster;
        if (!poster) throw new Error("no poster");
        const img = new Image();
        img.src = poster;
        await img.decode();
        return [img.naturalWidth, img.naturalHeight];
      });
      expect(Math.min(...size), app).toBeGreaterThan(200);
    }
  });
});

for (const [w, h] of [[568, 320], [320, 568], [844, 390], [390, 844]]) {
  test.describe(`${w} by ${h}`, () => {
    test.use({ viewport: { width: w, height: h }, reducedMotion: "reduce", hasTouch: true });

    test("each app's download and play are on screen and pressable", async ({ page }) => {
      for (const [i, app] of APPS.entries()) {
        await page.goto(`/#${app.toLowerCase()}`);
        const article = page.locator(".copy article").nth(i);
        await expect(article).toHaveClass(/current/);
        for (const control of [article.locator(".get"), article.locator(".toggle-clip")]) {
          // The copy may scroll inside itself; the page must not.
          await control.evaluate((c) => c.scrollIntoView({ block: "nearest" }));
          const hit = await control.evaluate((c) => {
            const r = c.getBoundingClientRect();
            const inside = r.top >= 0 && r.bottom <= innerHeight && r.left >= 0 && r.right <= innerWidth;
            const top = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
            return inside && !!top && c.contains(top);
          });
          expect(hit, `${app} ${await control.getAttribute("class")}`).toBe(true);
        }
        await expect(article).toHaveClass(/current/);
      }
    });
  });
}
