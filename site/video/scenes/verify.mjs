// Checks the exit cut: same screens, their own recordings.
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";
import recs from "../montage/recs.json" with { type: "json" };
import { moment } from "./timeline.ts";
import { slabs, turned } from "./turn.ts";

const here = dirname(fileURLToPath(import.meta.url));
const server = await createServer({ root: resolve(here, "../.."), configFile: false, logLevel: "error", server: { port: 0 } });
await server.listen();
const base = server.resolvedUrls.local[0];
const browser = await chromium.launch();

/** A film-sized page; `asked` lists the recordings it fetched. */
async function open(path, asked = []) {
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
  page.on("pageerror", (e) => {
    throw e;
  });
  await page.route("**/rec/**", (route) => {
    asked.push(new URL(route.request().url()).pathname);
    return route.fulfill({ path: resolve(here, "../../public/film/poster.jpg") });
  });
  await page.goto(`${base}${path}`, { waitUntil: "networkidle" });
  return page;
}

/** Each drawn recording: its source, its corners and its slab's. */
const drawn = (page, faces) =>
  page.$$eval(
    "#world image",
    (els, faces) =>
      els.map((el) => {
        const href = el.getAttribute("href");
        const at = (m, [x, y]) => [m.a * x + m.c * y + m.e, m.b * x + m.d * y + m.f];
        const own = el.getScreenCTM();
        // The group's matrix, without the image's own.
        const group = el.parentElement.getScreenCTM();
        const face = faces[href.split("/")[1]];
        return { href, corners: [[0, 0], [1280, 0], [0, 720]].map((p) => at(own, p)), face: face.map((p) => at(group, p)) };
      }),
    faces,
  );

/** Each slab's top-left, top-right and bottom-left screen corners. */
const faces = (angle) =>
  Object.fromEntries(
    recs.apps.map(({ rec }, i) => {
      const b = slabs[i];
      const Y = b.y + b.d;
      return [rec, [turned(angle, b.x, Y, b.z + b.h), turned(angle, b.x + b.w, Y, b.z + b.h), turned(angle, b.x, Y, b.z)]];
    }),
  );
const apart = (p, q) => Math.max(...p.map((c, i) => Math.hypot(c[0] - q[i][0], c[1] - q[i][1])));

try {
  const cut = recs.first + 129;
  const asked = [];
  const montage = await open("video/montage/index.html", asked);
  await montage.waitForFunction(() => window.montage);
  asked.length = 0;
  const { pins } = await montage.evaluate((f) => window.montage.render(f), cut - 1);
  // The frame each recording showed last in the montage.
  const last = Object.fromEntries(
    asked.map((path) => {
      const [, rec, name] = path.match(/\/rec\/([^/]+)\/(\d+)\.jpg$/);
      return [rec, Number(name)];
    }),
  );
  if (Object.keys(last).length !== recs.apps.length) throw new Error(`the montage loaded ${JSON.stringify(asked)}`);
  const exit = await open("video/scenes/index.html");
  // Every exit frame until the last screen has gone.
  for (let f = cut; ; f++) {
    await exit.evaluate((f) => window.ely.render("files", f), f);
    const m = moment(f / 30);
    const shown = await drawn(exit, faces(m.angle));
    // Each slab still up plays on, one frame a frame.
    const want = m.pieces
      .filter(({ box, land }) => land > 0 && slabs.includes(box))
      .map(({ box }) => recs.apps[slabs.indexOf(box)].rec)
      .map((rec) => `rec/${rec}/${String(last[rec] + f - cut + 1).padStart(5, "0")}.jpg`);
    const got = shown.map((s) => s.href);
    if (JSON.stringify([...got].sort()) !== JSON.stringify([...want].sort())) {
      throw new Error(`frame ${f}: drew ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
    }
    for (const { href, corners, face } of shown) {
      if (apart(corners, face) > 0.5) throw new Error(`frame ${f}: ${href} leaves its screen: ${JSON.stringify(corners)}, not ${JSON.stringify(face)}`);
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
