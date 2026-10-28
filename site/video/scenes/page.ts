// Draws one film frame with the site's own iso renderer.
import "../../src/iso.css";
import { UNIT, block, order, project } from "../../src/iso.ts";
import { ACCENT, model } from "../../src/model.ts";
import { type Box, DESK, PAPER, SCREEN, SHEET, lamp, props, screens, tiles } from "./parts.ts";
import { BAR, BEAT, glide, toward } from "./time.ts";
import { DESK_END, type Moment, SEED, TURN, moment } from "./timeline.ts";
import { film, montage, recording, turnedBlock, turnedOrder } from "./turn.ts";
import { view } from "../opening/plan.ts";

export const WIDTH = 1920;
export const HEIGHT = 1080;
export const FPS = 30;

/** Bars [from, to); plates push into one app. */
export interface Scene {
  name: string;
  from: number;
  to: number;
  app?: string;
}

const PLATES = ["ElyDoc", "ElyTable", "ElyShow", "ElyPDF", "ElyTree", "ElyMark"];
export const scenes: Scene[] = [
  { name: "opening", from: 0, to: 4 },
  { name: "rise", from: 4, to: 8 },
  ...PLATES.map((app, i) => ({ name: `plate-${app.slice(3).toLowerCase()}`, from: 8 + i * 4, to: 9 + i * 4, app })),
  { name: "files", from: 32, to: 36 },
  { name: "props", from: 36, to: 40 },
  { name: "end", from: 40, to: 42 },
];

/** Global frames [first, end) whose time falls in the scene. */
export function frames(scene: Scene): [number, number] {
  const first = (bar: number) => Math.ceil((bar * BAR * FPS) - 1e-9);
  return [first(scene.from), first(scene.to)];
}

/** Light tone to dark tone; three faces derive from it. */
const DARK: Record<string, string> = {
  [DESK]: "#3a3a3a",
  [PAPER]: "#5c5c5a",
  [SCREEN]: "#3a3a3c",
  [SHEET]: "#7a7a78",
  [ACCENT]: ACCENT,
};
const PAGE = ["#fbfbfa", "#252525"];
const INK = ["#1d1d1f", "#f5f5f7"];
const SOFT = ["#6e6e73", "#a1a1a6"];

const mix = (a: string, b: string, u: number) =>
  u <= 0 ? a : u >= 1 ? b : `color-mix(in oklab, ${a}, ${b} ${(u * 100).toFixed(2)}%)`;

function tone(light: string, dark: number): string {
  const night = DARK[light];
  if (!night) throw new Error(`no dark tone for ${light}`);
  return mix(light, night, dark);
}

/** World to frame pixels: scale, then the origin's pixel. */
interface Camera {
  s: number;
  x: number;
  y: number;
}

const corners = (b: Box): [number, number][] =>
  [0, 1].flatMap((i) => [0, 1].flatMap((j) => [0, 1].map((k) => project(b.x + i * b.w, b.y + j * b.d, b.z + k * b.h))));

/** Frames `boxes` inside w by h, centred at (cx, cy). */
function fit(boxes: Box[], w: number, h: number, cx: number, cy: number): Camera {
  const all = boxes.flatMap(corners);
  const xs = all.map(([x]) => x);
  const ys = all.map(([, y]) => y);
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const s = Math.min(w / (x1 - x0), h / (y1 - y0));
  return { s, x: cx - (s * (x0 + x1)) / 2, y: cy - (s * (y0 + y1)) / 2 };
}

/** Eases between cameras by log scale and centre point. */
function blend(a: Camera, b: Camera, u: number): Camera {
  if (u <= 0) return a;
  if (u >= 1) return b;
  const s = a.s * (b.s / a.s) ** u;
  const ca = [(WIDTH / 2 - a.x) / a.s, (HEIGHT / 2 - a.y) / a.s];
  const cb = [(WIDTH / 2 - b.x) / b.s, (HEIGHT / 2 - b.y) / b.s];
  const [cx, cy] = [ca[0] + (cb[0] - ca[0]) * u, ca[1] + (cb[1] - ca[1]) * u];
  return { s, x: WIDTH / 2 - s * cx, y: HEIGHT / 2 - s * cy };
}

const everything = [...model.flatMap((p) => p.blocks), ...lamp, ...props.flatMap((p) => p.blocks)];
/** Model under the title; whole model; end card. */
const hero = fit(everything, 1720, 790, WIDTH / 2, 640);
const wide = fit(everything, 1800, 960, WIDTH / 2, 545);
/** End card: below the title, or above delivered voxel type. */
const ending = () => (hidden.has("name") ? fit(everything, 1500, 340, WIDTH / 2, 235) : fit(everything, 1440, 600, WIDTH / 2, 575));
/** The opening's last framing, in this renderer's terms. */
const opened = (() => {
  const v = view(514);
  const [cx, cy] = project(v.cx, v.cy, 0);
  return { s: v.s / UNIT, x: v.ox - (v.s / UNIT) * cx, y: v.oy - (v.s / UNIT) * cy };
})();
const close = props.map((p) => fit(p.blocks, 1200, 640, WIDTH / 2, 420));
const seed = (() => {
  const [px, py] = project(SEED[0] + 0.5, SEED[1] + 0.5, model[0].blocks[0].h);
  return { s: 10, x: WIDTH / 2 - 10 * px, y: HEIGHT / 2 - 10 * py };
})();

/** The camera for model scenes at time `t`. */
function camera(t: number): Camera {
  const beat = t / BEAT;
  if (beat < 16) {
    const x = Math.min(beat / (DESK_END + 2), 1);
    const v = 1 - (1 - x) ** 1.6;
    const s = 1 / (1 / seed.s + (1 / hero.s - 1 / seed.s) * v);
    const c = blend(seed, hero, Math.log(s / seed.s) / Math.log(hero.s / seed.s));
    return { ...c, s };
  }
  if (beat < 128) return blend(hero, { ...hero, s: hero.s * 1.05 }, (beat - 16) / 16);
  if (beat < TURN[1]) return blend(montage, wide, glide((beat - TURN[0]) / (TURN[1] - TURN[0])));
  if (beat < 144) return blend(wide, close[0], toward(t, 144, BEAT / 2, glide));
  if (beat < 148) return blend(close[0], close[1], toward(t, 148, BEAT / 2, glide));
  if (beat < 152) return blend(close[1], close[2], toward(t, 152, BEAT / 2, glide));
  return blend(close[2], ending(), toward(t, 160, BAR, glide));
}

/** Pinned face corners: TL, TR, BL, BR of +y side. */
function face(b: Box): [number, number][] {
  const y = b.y + b.d;
  return [project(b.x, y, b.z + b.h), project(b.x + b.w, y, b.z + b.h), project(b.x, y, b.z), project(b.x + b.w, y, b.z)];
}

/** The flat window the recording lands in; matches edit/plan.json. */
const WINDOW = { x: 144, y: 40, w: 1632, h: 918 };
const RECT: [number, number][] = [
  [WINDOW.x, WINDOW.y],
  [WINDOW.x + WINDOW.w, WINDOW.y],
  [WINDOW.x, WINDOW.y + WINDOW.h],
  [WINDOW.x + WINDOW.w, WINDOW.y + WINDOW.h],
];

/** Face bounds in world pixels. */
function bounds(app: string) {
  const pts = face(screens[app]);
  const xs = pts.map(([x]) => x);
  const ys = pts.map(([, y]) => y);
  return { x0: Math.min(...xs), x1: Math.max(...xs), y0: Math.min(...ys), y1: Math.max(...ys) };
}

/** Pushes toward the screen, into it on the last beat. */
function push(app: string, t: number, beat: number): { cam: Camera; flat: number } {
  const f = bounds(app);
  const [cx, cy] = [(f.x0 + f.x1) / 2, (f.y0 + f.y1) / 2];
  const at = (s: number, px: number, py: number): Camera => ({ s, x: px - s * cx, y: py - s * cy });
  const near = Math.min((0.62 * HEIGHT) / (f.y1 - f.y0), 1060 / (f.x1 - f.x0));
  const into = WINDOW.w / (f.x1 - f.x0);
  const start = opened;
  const aim = at(near, 1250, 520);
  const full = at(into, WINDOW.x + WINDOW.w / 2, WINDOW.y + WINDOW.h / 2);
  const flat = toward(t, beat + 4, BEAT, glide);
  const cam = flat > 0 ? blend(aim, full, flat) : blend(start, aim, toward(t, beat + 3, 3 * BEAT, glide));
  return { cam, flat };
}

const svg = document.querySelector<SVGSVGElement>("#frame");
const world = document.querySelector<SVGGElement>("#world");
if (!svg || !world) throw new Error("index.html lacks #frame or #world");

/** Draws the moment; leaving slabs carry their recordings. */
function draw(m: Moment, cam: Camera, f: number): Promise<void>[] {
  const page = mix(PAGE[0], PAGE[1], m.dark);
  document.body.style.background = page;
  svg!.style.background = page;
  world!.setAttribute("transform", `matrix(${cam.s} 0 0 ${cam.s} ${cam.x} ${cam.y})`);
  const loads: Promise<void>[] = [];
  const groups = m.pieces.map(({ box, land, dark }) => {
    const toned = { ...box, tone: tone(box.tone, dark) };
    const g = m.angle ? turnedBlock(toned, m.angle) : block(toned);
    const href = recording(box, land, f);
    if (href) {
      const { el, ready } = film(box, m.angle, href);
      g.append(el);
      loads.push(ready);
    }
    if (tiles.includes(box)) g.classList.add("tile");
    if (land < 1) {
      g.setAttribute("transform", `translate(0 ${(-UNIT * (1 - land)).toFixed(3)})`);
      g.setAttribute("opacity", land.toFixed(4));
    }
    return g;
  });
  const boxes = m.pieces.map((p) => p.box);
  world!.replaceChildren(...(m.angle ? turnedOrder(boxes, m.angle) : order(boxes)).map((i) => groups[i]));
  for (const el of document.querySelectorAll<HTMLElement>("[data-text]")) {
    const shown = hidden.has(el.dataset.text!) ? 0 : (m.texts[el.dataset.text!] ?? 0);
    el.style.opacity = shown.toFixed(4);
    const [day, dark] = el.dataset.soft === undefined ? INK : SOFT;
    el.style.color = mix(day, dark, m.dark);
  }
  return loads;
}

/** Texts another layer draws instead. */
const hidden = new Set<string>();

/** Screen-face corners in frame pixels, and the flattening. */
export interface Face {
  opacity: number;
  flat: number;
  corners: [number, number][];
}

/** Draws global frame `f` of `scene`; plates return the face. */
async function render(name: string, f: number): Promise<Face | null> {
  const scene = scenes.find((s) => s.name === name);
  if (!scene) throw new Error(`no scene ${name}`);
  const [first, end] = frames(scene);
  if (f < first || f >= end) throw new Error(`frame ${f} is outside ${name}`);
  const t = f / FPS;
  const app = scene.app;
  const beat = scene.from * 4;
  const m = moment(t, app ? { app, beat: beat + 1 } : undefined);
  const { cam, flat } = app ? push(app, t, beat) : { cam: camera(t), flat: 0 };
  await Promise.all(draw(m, cam, f));
  await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  if (!app) return null;
  const piece = m.pieces.find((p) => p.box === screens[app]);
  if (!piece) return { opacity: 0, flat, corners: [] };
  const lift = -UNIT * (1 - piece.land);
  const pinned = face(piece.box).map(([x, y]) => [cam.x + cam.s * x, cam.y + cam.s * (y + lift)]);
  return {
    opacity: piece.land,
    flat,
    corners: pinned.map(([x, y], i) => [x + (RECT[i][0] - x) * flat, y + (RECT[i][1] - y) * flat]),
  };
}

declare global {
  interface Window {
    ely: { scenes: Scene[]; frames: typeof frames; render: typeof render; hide: (names: string[]) => void };
  }
}
window.ely = { scenes, frames, render, hide: (names) => names.forEach((n) => hidden.add(n)) };
