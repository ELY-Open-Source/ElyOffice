// Renders the app callout layers, punch-in tracks and margin grid.
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { calloutFrame, gridFrame, lifts } from "./draw.mjs";
import { APPS, EDIT_PLAN, FPS, frames, view, zoomList } from "./plan.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(resolve(here, "../scenes/package.json"));
const { chromium } = require("playwright");
const extra = resolve(here, "../out/extra");
const PAGES = 6;

const browser = await chromium.launch();

/** A transparent film-sized page that shows one SVG. */
async function open() {
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });
  page.on("pageerror", (e) => {
    throw e;
  });
  await page.setContent(
    `<!doctype html><meta charset="utf-8"><style>html,body{margin:0;background:transparent}
    svg{display:block;-webkit-font-smoothing:antialiased}</style><body></body>`,
  );
  return page;
}

const pages = await Promise.all(Array.from({ length: PAGES }, open));

/** Writes PNGs for global frames, spreading work over pages. */
async function paint(dir, list, draw) {
  let next = 0;
  await Promise.all(
    pages.map(async (page) => {
      while (next < list.length) {
        const frame = list[next++];
        await page.evaluate((s) => {
          document.body.innerHTML = s;
          return document.fonts.ready;
        }, draw(frame / FPS));
        await page.screenshot({ path: join(dir, `${String(frame).padStart(5, "0")}.png`), omitBackground: true });
      }
    }),
  );
}

function ffmpeg(args) {
  const run = spawnSync("ffmpeg", ["-hide_banner", "-v", "error", "-y", ...args], { stdio: "inherit" });
  if (run.status !== 0) throw new Error(`ffmpeg failed: ${args.join(" ")}`);
}

/** H.264 preview of an over layer on the light field. */
function preview(name, first, count) {
  const dir = join(extra, name);
  ffmpeg([
    "-f", "lavfi", "-i", `color=c=0xfbfbfa:s=1920x1080:r=${FPS}`,
    "-framerate", String(FPS), "-start_number", String(first), "-i", join(dir, "%05d.png"),
    "-filter_complex", "[0][1]overlay=format=auto:shortest=1,format=yuv420p",
    "-frames:v", String(count), "-c:v", "libx264", "-crf", "18", join(extra, `${name}.mp4`),
  ]);
}

function fresh(name) {
  const dir = join(extra, name);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  return dir;
}

const manifest = (dir, name, first, count) =>
  writeFileSync(join(dir, "manifest.json"), JSON.stringify({ name, first, count, fps: FPS, kind: "over" }, null, 1));

const wanted = process.argv.slice(2);
const want = (name) => !wanted.length || wanted.includes(name);
console.log(`plan: ${EDIT_PLAN}`);

for (const a of APPS) {
  const name = `${a.app}-fx`;
  if (!want(name)) continue;
  const { first, count } = frames(a);
  const dir = fresh(name);
  const list = Array.from({ length: count }, (_, k) => first + k);
  writeFileSync(join(dir, "zoom.json"), JSON.stringify(zoomList(a), null, 1));
  const rects = list.map((fr) => view(a, fr / FPS).map((v) => Math.round(v * 100) / 100));
  writeFileSync(join(dir, "view.json"), JSON.stringify({ first, rects }));
  await paint(dir, list, (t) => calloutFrame(a, t));
  manifest(dir, name, first, count);
  preview(name, first, count);
  console.log(`${name}: frames ${first}–${first + count - 1}`);
}

if (want("fx-grid")) {
  const dir = fresh("fx-grid");
  const spans = APPS.map((a, i) => ({ ...frames(a), lifts: lifts(a, 0x5eed + i) }));
  const first = spans[0].first;
  const last = spans.at(-1).first + spans.at(-1).count - 1;
  for (const s of spans) {
    const list = Array.from({ length: s.count }, (_, k) => s.first + k);
    await paint(dir, list, (t) => gridFrame(s.lifts, t));
  }
  const empty = join(dir, "empty.png");
  await pages[0].evaluate(() => (document.body.innerHTML = ""));
  await pages[0].screenshot({ path: empty, omitBackground: true });
  for (let fr = first; fr <= last; fr++) {
    if (!spans.some((s) => fr >= s.first && fr < s.first + s.count)) {
      copyFileSync(empty, join(dir, `${String(fr).padStart(5, "0")}.png`));
    }
  }
  rmSync(empty);
  manifest(dir, "fx-grid", first, last - first + 1);
  preview("fx-grid", first, last - first + 1);
  console.log(`fx-grid: frames ${first}–${last}`);
}

await browser.close();
