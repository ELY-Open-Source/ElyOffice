// Checks the exit cut: same screens, their own recordings.
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";
import recs from "../montage/recs.json" with { type: "json" };
import { moment } from "./timeline.ts";
import { recording } from "./turn.ts";

const here = dirname(fileURLToPath(import.meta.url));
const server = await createServer({ root: resolve(here, "../.."), configFile: false, logLevel: "error", server: { port: 0 } });
await server.listen();
const base = server.resolvedUrls.local[0];
const browser = await chromium.launch();

/** A film-sized page whose recordings are any one frame. */
async function open(path) {
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
  page.on("pageerror", (e) => {
    throw e;
  });
  await page.route("**/rec/**", (route) => route.fulfill({ path: resolve(here, "../../public/film/poster.jpg") }));
  await page.goto(`${base}${path}`, { waitUntil: "networkidle" });
  return page;
}

/** Each drawn recording: its source and three screen corners. */
const drawn = (page) =>
  page.$$eval("#world image", (els) =>
    els.map((el) => {
      const m = el.getScreenCTM();
      const at = (x, y) => [m.a * x + m.c * y + m.e, m.b * x + m.d * y + m.f];
      return { href: el.getAttribute("href"), corners: [at(0, 0), at(1280, 0), at(0, 720)] };
    }),
  );

try {
  const cut = recs.first + 129;
  const montage = await open("video/montage/index.html");
  await montage.waitForFunction(() => window.montage);
  const { pins } = await montage.evaluate((f) => window.montage.render(f), cut - 1);
  const exit = await open("video/scenes/index.html");
  // Every exit frame until the last screen has gone.
  for (let f = cut; ; f++) {
    await exit.evaluate((f) => window.ely.render("files", f), f);
    const shown = await drawn(exit);
    const want = moment(f / 30).pieces.map(({ box, land }) => recording(box, land, f)).filter(Boolean);
    const got = shown.map((s) => s.href);
    if (JSON.stringify([...got].sort()) !== JSON.stringify([...want].sort())) {
      throw new Error(`frame ${f}: drew ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
    }
    if (f === cut) {
      if (got.length !== 6) throw new Error(`the cut shows ${got.length} screens`);
      // The montage pins top-left, top-right, bottom-right, bottom-left.
      for (const { href, corners } of shown) {
        const app = recs.apps.find((a) => href.startsWith(`rec/${a.rec}/`)).app;
        const pin = pins[app];
        const near = [pin[0], pin[1], pin[3]].every((p, i) => Math.hypot(p[0] - corners[i][0], p[1] - corners[i][1]) < 1);
        if (!near) throw new Error(`${app} moves at the cut: ${JSON.stringify(pin)} to ${JSON.stringify(corners)}`);
      }
    }
    if (!want.length) break;
  }
  console.log("verify: screens hold still across the cut and play their own frames");
} finally {
  await browser.close();
  await server.close();
}
