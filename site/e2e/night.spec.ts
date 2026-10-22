import { expect, test, type Page } from "./fixtures.ts";

const PAGE = "/";
const DESK = { light: "rgb(230, 230, 226)", dark: "rgb(58, 58, 58)" };

let errors: string[] = [];

test.beforeEach(async ({ page }) => {
  errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
});

test.afterEach(() => expect(errors).toEqual([]));

const open = async (page: Page) => {
  await page.goto(PAGE);
  await expect(page.locator(".model .block").first()).toBeAttached();
};

/** The hero desk's top face colour. */
const desk = (page: Page) =>
  page.evaluate(() => {
    const top = document.querySelector(".model .block .face.top")!;
    return getComputedStyle(top).fill;
  });

const lamp = (page: Page) => page.getByRole("button", { name: "Switch to dark" });

test("the lamp turns the page dark in a wave", async ({ page }) => {
  await open(page);
  expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe("light");
  expect(await desk(page)).toBe(DESK.light);

  // Read the nearest and farthest hero blocks mid-wave.
  const mid = await page.evaluate(async () => {
    const tops = [...document.querySelectorAll<SVGGElement>(".model .block")];
    document.querySelector<HTMLButtonElement>(".night")!.click();
    const wave = (g: SVGGElement) => parseFloat(g.style.getPropertyValue("--wave"));
    tops.sort((a, b) => wave(a) - wave(b));
    const [near, far] = [tops[0], tops[tops.length - 1]];
    const fill = (g: SVGGElement) => getComputedStyle(g.querySelector(".face.top")!).fill;
    // Sample between the near landing and the far start.
    const gap = (wave(near) + 0.2 + wave(far)) / 2;
    await new Promise((r) => setTimeout(r, gap * 1000));
    return {
      steps: [wave(near), wave(far)],
      near: [fill(near), near.style.getPropertyValue("--tone-dark")],
      far: [fill(far), far.style.getPropertyValue("--tone")],
    };
  });
  expect(mid.steps[0] + 0.25).toBeLessThan(mid.steps[1]);
  expect(mid.steps[1]).toBeLessThanOrEqual(1);
  const rgb = (hex: string) => `rgb(${[1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)).join(", ")})`;
  expect(mid.near[0]).toBe(rgb(mid.near[1]));
  expect(mid.far[0]).toBe(rgb(mid.far[1]));

  await expect.poll(() => desk(page), { timeout: 1200 }).toBe(DESK.dark);
  expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe("dark");
  await expect(page.getByRole("button", { name: "Switch to light" })).toBeAttached();
  await expect(page.locator(".night-sky")).toHaveCount(0);
  await expect(page.locator("html")).toHaveCSS("background-color", "rgb(37, 37, 37)");
});

for (const scheme of ["light", "dark"] as const) {
  test(`${scheme} text and links keep 4.5:1 against the page`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await open(page);
    const worst = await page.evaluate(() => {
      const parse = (c: string) => {
        const [r, g, b, a = 1] = c.match(/[\d.]+/g)!.map(Number);
        return { rgb: [r, g, b], a };
      };
      const lum = (rgb: number[]) => {
        const [r, g, b] = rgb.map((v) => {
          const s = v / 255;
          return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
        });
        return 0.2126 * r + 0.7152 * g + 0.0722 * b;
      };
      // Text sits on its nearest filled box, else on the page.
      const ground = (el: Element) => {
        for (let at: Element | null = el; at; at = at.parentElement) {
          const bg = parse(getComputedStyle(at).backgroundColor);
          if (bg.a === 1) return bg.rgb;
        }
        throw new Error("the page has no background");
      };
      const alpha = (el: HTMLElement) => {
        let a = parse(getComputedStyle(el).color).a;
        for (let at: Element | null = el; at; at = at.parentElement) a *= Number(getComputedStyle(at).opacity);
        return a;
      };
      const shown = (el: HTMLElement, under: number[]) => {
        const a = alpha(el);
        return parse(getComputedStyle(el).color).rgb.map((v, i) => v * a + under[i] * (1 - a));
      };
      const ratios = [...document.querySelectorAll<HTMLElement>("h1, h2, h3, p, a, li, button")]
        .filter((el) => el.textContent?.trim() && alpha(el) > 0)
        .map((el) => {
          const under = ground(el);
          const page = lum(under);
          const text = lum(shown(el, under));
          return { ratio: (Math.max(text, page) + 0.05) / (Math.min(text, page) + 0.05), text: el.textContent };
        });
      return ratios.sort((a, b) => a.ratio - b.ratio)[0];
    });
    expect(worst.ratio, String(worst.text)).toBeGreaterThanOrEqual(4.5);
});
}

test("the system scheme sets the first theme; the lamp's choice stays", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await open(page);
  expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe("dark");
  expect(await desk(page)).toBe(DESK.dark);

  await page.emulateMedia({ colorScheme: "light" });
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.theme)).toBe("light");

  await page.emulateMedia({ reducedMotion: "reduce" });
  await lamp(page).click();
  expect(await desk(page)).toBe(DESK.dark);
  await page.emulateMedia({ colorScheme: "light" });
  await page.reload();
  expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe("dark");
});

test("reduced motion switches at once", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "light" });
  await open(page);
  await lamp(page).click();
  expect(await desk(page)).toBe(DESK.dark);
  await expect(page.locator(".night-sky")).toHaveCount(0);
});
