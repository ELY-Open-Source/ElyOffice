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

/** Each drawn screen: its slab, opacity, front face and recordings. */
const drawn = (page) =>
  page.$$eval("#world g[data-slab]", (groups) =>
    groups.map((g) => {
      const at = (m, x, y) => [m.a * x + m.c * y + m.e, m.b * x + m.d * y + m.f];
      const front = g.querySelector("polygon[data-front]");
      const points = front ? [...front.points].map((p) => at(front.getScreenCTM(), p.x, p.y)) : null;
      const films = [...g.querySelectorAll("image")].map((el) => {
        const m = el.getScreenCTM();
        // How much shows of it, against its group.
        let shows = getComputedStyle(el).visibility === "visible" ? 1 : 0;
        for (let n = el; n !== g; n = n.parentElement) {
          const style = getComputedStyle(n);
          shows *= style.display === "none" ? 0 : Number(style.opacity);
        }
        return { href: el.getAttribute("href"), shows, corners: [[0, 0], [1280, 0], [0, 720], [1280, 720]].map(([x, y]) => at(m, x, y)) };
      });
      const m = g.getScreenCTM();
      const group = [m.a, m.b, m.c, m.d, m.e, m.f];
      return { slab: Number(g.dataset.slab), opacity: Number(g.getAttribute("opacity") ?? 1), group, points, films };
    }),
  );

/** A slab's top-left, top-right and bottom-left at `angle`, through `m`. */
function face(slab, angle, m) {
  const b = slabs[slab];
  const Y = b.y + b.d;
  const on = ([x, y]) => [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]];
  return [turned(angle, b.x, Y, b.z + b.h), turned(angle, b.x + b.w, Y, b.z + b.h), turned(angle, b.x, Y, b.z)].map(on);
}
const near = (p, q) => Math.hypot(p[0] - q[0], p[1] - q[1]) < 0.5;
/** Two corner lists naming the same points, in any order. */
const same = (ps, qs) => ps.length === qs.length && ps.every((p) => qs.some((q) => near(p, q)));

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
  // Screens drawn last frame, by slab, with their opacity.
  let before = new Map();
  const gone = new Set();
  for (let f = cut; f === cut || before.size; f++) {
    if (f > cut + 300) throw new Error("the screens never leave");
    await exit.evaluate((f) => window.ely.render("files", f), f);
    // A screen faded to nothing has gone.
    const shown = (await drawn(exit)).filter(({ opacity }) => opacity > 0);
    const angle = moment(f / 30).angle;
    for (const { slab, opacity, group, points, films } of shown) {
      const { app, rec } = recs.apps[slab];
      if (gone.has(slab)) throw new Error(`frame ${f}: ${app} comes back`);
      if (opacity > (before.get(slab) ?? 1) + 1e-6) throw new Error(`frame ${f}: ${app} brightens to ${opacity}`);
      // One recording a screen, one frame a frame on.
      const want = `rec/${rec}/${String(last[rec] + f - cut + 1).padStart(5, "0")}.jpg`;
      if (films.length !== 1 || films[0].href !== want) throw new Error(`frame ${f}: ${app} drew ${JSON.stringify(films.map((x) => x.href))}, not ${want}`);
      if (!points) throw new Error(`frame ${f}: ${app} draws no screen face`);
      const { corners, shows } = films[0];
      if (shows !== 1) throw new Error(`frame ${f}: ${app}'s recording shows at ${shows}`);
      if (!face(slab, angle, group).every((p, i) => near(p, corners[i]))) throw new Error(`frame ${f}: ${app}'s recording turns off its slab`);
      if (!same(corners, points)) throw new Error(`frame ${f}: ${app}'s recording is not on its screen: ${JSON.stringify(corners)} on ${JSON.stringify(points)}`);
    }
    if (f === cut) {
      if (shown.length !== recs.apps.length) throw new Error(`the cut shows ${shown.length} screens`);
      for (const { slab, points } of shown) {
        const { app } = recs.apps[slab];
        if (!same(points, pins[app])) throw new Error(`${app} moves at the cut: ${JSON.stringify(pins[app])} to ${JSON.stringify(points)}`);
      }
    }
    const now = new Map(shown.map(({ slab, opacity }) => [slab, opacity]));
    // A screen leaves only once faded out.
    for (const [slab, opacity] of before) {
      if (now.has(slab)) continue;
      if (opacity > 0.1) throw new Error(`frame ${f}: ${recs.apps[slab].app} vanishes at opacity ${opacity}`);
      gone.add(slab);
    }
    before = now;
  }
  if (gone.size !== recs.apps.length) throw new Error(`only ${gone.size} screens left`);
  console.log("verify: screens hold still across the cut, play their own frames and fade out");
} finally {
  await browser.close();
  await server.close();
}
