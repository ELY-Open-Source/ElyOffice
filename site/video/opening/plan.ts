// What the opening shows at each global frame.
import { ACCENT, model } from "../../src/model.ts";
import { bezier } from "../scenes/time.ts";
import { type Box, type Shown, type View, ground, project } from "./draw.ts";

export const FPS = 30;
export const WIDTH = 1920;
export const HEIGHT = 1080;
const BEAT = 60 / 112;
const LAND = 0.2 * FPS;

/** Bars 3 to 8: beats 8 to 32. */
export const FIRST = frameAt(8);
export const END = frameAt(32);

const settle = bezier(0.2, 0.8, 0.3, 1);
const glide = bezier(0.65, 0, 0.35, 1);

/** The frame nearest a global beat. */
export function frameAt(beat: number): number {
  return Math.round(beat * BEAT * FPS);
}

/** Progress of a move of `span` frames ending on `beat`. */
function toward(f: number, beat: number, span = LAND, ease = settle): number {
  const end = frameAt(beat);
  return ease(Math.min(1, Math.max(0, (f - (end - span)) / span)));
}

const desk = model[0].blocks[0];
const TOP = desk.h;
const CX = desk.w / 2;
const CY = desk.d / 2;
const SEED: [number, number] = [8, 5];

/** Chebyshev ring of a cell around the seed. */
const ring = (i: number, j: number) => Math.max(Math.abs(i - SEED[0]), Math.abs(j - SEED[1]));

/** Tiles landing per sixteenth from beat 8; rest on 16. */
const RAIN: Record<number, number> = {
  2: 1, 4: 1, 6: 2, 8: 2, 10: 2, 12: 3, 13: 1, 14: 3, 15: 2, 16: 3, 17: 3, 18: 4, 19: 4,
  20: 5, 21: 5, 22: 6, 23: 6, 24: 7, 25: 8, 26: 9, 27: 10, 28: 11, 29: 12, 30: 13, 31: 14,
};

interface Tile {
  box: Box;
  lands: number;
  falls: number;
}

const tiles: Tile[] = (() => {
  const cells = Array.from({ length: desk.w * desk.d }, (_, n) => [n % desk.w, Math.floor(n / desk.w)]);
  const angle = ([i, j]: number[]) => Math.atan2(j - SEED[1], i - SEED[0]);
  cells.sort((a, b) => ring(a[0], a[1]) - ring(b[0], b[1]) || angle(a) - angle(b));
  const slots = Object.entries(RAIN).flatMap(([k, n]) => Array<number>(n).fill(Number(k)));
  return cells.map(([i, j], n) => ({
    box: { x: i, y: j, z: 0, w: 1, d: 1, h: TOP, tone: desk.tone },
    lands: frameAt(8 + (slots[n] ?? 32) / 4),
    falls: Math.round(20 - (11 * n) / cells.length),
  }));
})();

/** Rise order by road: Doc, Mark, Tree, Table, Show, PDF. */
const RISE: [string, number][] = [
  ["ElyDoc", 16.25], ["ElyMark", 17.5], ["ElyTree", 18.5], ["ElyTable", 19.5], ["ElyShow", 20.75], ["ElyPDF", 22],
];
const parts = RISE.map(([name, beat]) => {
  const part = model.find((p) => p.name === name);
  if (!part) throw new Error(`no part ${name}`);
  return part.blocks.map((box, i) => ({ box, beat: beat + i / 4 }));
}).flat();

const roads = model.find((p) => p.name === "Roads")?.blocks;
if (!roads) throw new Error("model has no roads");

/** A road in cells, laid from one end on sixteenths. */
function lay(road: Box, n: number, beat: number, reverse = false): { box: Box; beat: number }[] {
  const long = road.w >= road.d;
  return Array.from({ length: n }, (_, i) => {
    const k = reverse ? n - 1 - i : i;
    const box = long
      ? { ...road, x: road.x + (k * road.w) / n, w: road.w / n }
      : { ...road, y: road.y + (k * road.d) / n, d: road.d / n };
    return { box, beat: beat + i / 4 };
  });
}

const main = lay(roads[3], 18, 19);
const paving = [
  ...lay(roads[0], 1, 16.75),
  { ...main[0], beat: 17 },
  ...lay(roads[4], 3, 17.25),
  ...lay(roads[5], 4, 18),
  ...main.slice(1).map((c, i) => ({ ...c, beat: 19 + i / 4 })),
  ...lay(roads[1], 1, 20.75),
  ...lay(roads[2], 3, 22.5, true),
];

/** Files hop one cell a beat toward their app. */
const STEP = 0.6;
const FILE = 0.36;
const streams: { path: [number, number][]; spawns: number[] }[] = [
  { path: [[4.3, 7.9], [6.2, 7.9]], spawns: [20, 23, 26, 29] },
  { path: [[2.3, 4.3], [2.3, 4.9], [12.7, 4.9]], spawns: [22, 25, 28] },
  { path: [[6.3, 4.2], [6.3, 4.9], [12.7, 4.9]], spawns: [24, 27, 30] },
  { path: [[11, 3.15], [11, 4.9], [12.7, 4.9]], spawns: [25, 28, 31] },
];

function along(path: [number, number][], s: number): [number, number] {
  for (let i = 1; i < path.length; i++) {
    const [[ax, ay], [bx, by]] = [path[i - 1], path[i]];
    const len = Math.hypot(bx - ax, by - ay);
    if (s <= len || i === path.length - 1) {
      const u = Math.min(s / len, 1);
      return [ax + (bx - ax) * u, ay + (by - ay) * u];
    }
    s -= len;
  }
  throw new Error("a path needs two points");
}

const length = (path: [number, number][]) =>
  path.slice(1).reduce((sum, [x, y], i) => sum + Math.hypot(x - path[i][0], y - path[i][1]), 0);

function files(f: number): Shown[] {
  const out: Shown[] = [];
  for (const { path, spawns } of streams) {
    const total = length(path);
    const hops = Math.ceil(total / STEP);
    for (const spawn of spawns) {
      if (f <= frameAt(spawn) - LAND) continue;
      let [s, lift] = [0, 0];
      for (let k = 1; k <= hops; k++) {
        const u = toward(f, spawn + k);
        s += STEP * u;
        if (u > 0 && u < 1) lift = 0.35 * Math.sin(Math.PI * toward(f, spawn + k, LAND, (x) => x));
      }
      const sink = 1 - toward(f, spawn + hops + 1);
      if (sink <= 0) continue;
      const [x, y] = along(path, Math.min(s, total));
      const drop = 1 - toward(f, spawn);
      const box = { x: x - FILE / 2, y: y - FILE / 2, z: TOP + 0.08 + lift + drop, w: FILE, d: FILE, h: FILE * sink, tone: ACCENT };
      out.push({ box, alpha: 1 - drop });
    }
  }
  return out;
}

/** World turn: quarter on bar 6, back on bar 7. */
function turn(f: number): number {
  const out = toward(f, 22, 2 * BEAT * FPS, glide);
  const back = toward(f, 28, 4 * BEAT * FPS, glide);
  return (Math.PI / 2) * (out - back);
}

const all = model.flatMap((p) => p.blocks);
const built = all.filter((b) => b !== desk);
const flat = (t: number): View => ({ turn: t, cx: CX, cy: CY, s: 1, ox: 0, oy: 0 });
const TURNS = Array.from({ length: 19 }, (_, i) => (i * Math.PI) / 36);

function extent(list: Box[], turns: number[]): [number, number, number, number] {
  let [x0, x1, y0, y1] = [Infinity, -Infinity, Infinity, -Infinity];
  for (const t of turns)
    for (const b of list)
      for (const [i, j, k] of [0, 1].flatMap((i) => [0, 1].flatMap((j) => [0, 1].map((k) => [i, j, k])))) {
        const [x, y] = project(flat(t), b.x + i * b.w, b.y + j * b.d, b.z + k * b.h);
        [x0, x1, y0, y1] = [Math.min(x0, x), Math.max(x1, x), Math.min(y0, y), Math.max(y1, y)];
      }
  return [x0, x1, y0, y1];
}

interface Frame {
  s: number;
  ox: number;
  oy: number;
}

/** Bars 5 and 6: the desk fills 70% width. */
const wide: Frame = (() => {
  const [x0, x1] = extent(all, [0]);
  const [u0, u1, v0, v1] = extent(all, TURNS);
  const s = Math.min((0.7 * WIDTH) / (x1 - x0), (0.92 * HEIGHT) / (v1 - v0));
  return { s, ox: WIDTH / 2 - (s * (u0 + u1)) / 2, oy: HEIGHT / 2 - (s * (v0 + v1)) / 2 };
})();

/** Bars 7 and 8: buildings above the lower 35%. */
const titled: Frame = (() => {
  const [, , top] = extent(all, TURNS);
  const [u0, u1, , bottom] = extent(built, TURNS);
  const [high, low] = [70, 0.63 * HEIGHT];
  const s = Math.min(wide.s, (low - high) / (bottom - top));
  return { s, ox: WIDTH / 2 - (s * (u0 + u1)) / 2, oy: low - s * bottom };
})();

/** Bar 3 opens close on the seed, low in frame. */
const close: Frame = (() => {
  const s = wide.s * 2.6;
  const [x, y] = project(flat(0), SEED[0] + 0.5, SEED[1] + 0.5, TOP);
  return { s, ox: WIDTH / 2 - s * x, oy: 0.66 * HEIGHT - s * y };
})();

function blend(a: Frame, b: Frame, u: number): Frame {
  const s = a.s * (b.s / a.s) ** u;
  const at = (o: number, fs: number) => (WIDTH / 2 - o) / fs;
  const py = (o: number, fs: number) => (HEIGHT / 2 - o) / fs;
  const [wx, wy] = [at(a.ox, a.s) + (at(b.ox, b.s) - at(a.ox, a.s)) * u, py(a.oy, a.s) + (py(b.oy, b.s) - py(a.oy, a.s)) * u];
  return { s, ox: WIDTH / 2 - s * wx, oy: HEIGHT / 2 - s * wy };
}

/** The camera and turn for frame `f`. */
export function view(f: number): View {
  const pull = toward(f, 16, 8 * BEAT * FPS, glide);
  const rest = toward(f, 24, 2 * BEAT * FPS, glide);
  const cam = blend(blend(close, wide, pull), titled, rest);
  return { turn: turn(f), cx: CX, cy: CY, ...cam };
}

/** Frame when field ring `r` finishes flipping light. */
export const flips: number[] = (() => {
  const fronts = Array.from({ length: 33 }, (_, k) => {
    const v = view(frameAt(8 + k / 4));
    const corners = [[0, 0], [WIDTH, 0], [0, HEIGHT], [WIDTH, HEIGHT]].map(([x, y]) => ground(v, x, y));
    const reach = Math.max(...corners.map(([i, j]) => ring(Math.floor(i), Math.floor(j)))) + 1;
    return k < 4 ? -1 : k === 32 ? Infinity : reach * ((k - 3) / 29) ** 2.4;
  });
  const out: number[] = [];
  for (let r = 0; out.length === r; r++) {
    const k = fronts.findIndex((front) => front >= r);
    out.push(frameAt(8 + k / 4));
    if (k === 32) break;
  }
  return out;
})();

/** Progress of field cell (i, j) toward light. */
export function flip(f: number, i: number, j: number): number {
  const r = ring(i, j);
  const end = flips[Math.min(r, flips.length - 1)];
  return settle(Math.min(1, Math.max(0, (f - (end - LAND)) / LAND)));
}

/** Every box on the desk at frame `f`. */
export function pieces(f: number, v: View): Shown[] {
  const out: Shown[] = [];
  if (f >= frameAt(16)) out.push({ box: desk, alpha: 1 });
  else
    for (const t of tiles) {
      if (f <= t.lands - t.falls) continue;
      const u = Math.min(1, (f - (t.lands - t.falls)) / t.falls);
      const [, y] = project(v, t.box.x + 0.5, t.box.y + 0.5, TOP);
      const off = ((y + 80) * (1 - u * u)) / v.s;
      out.push({ box: { ...t.box, z: off }, alpha: 1, seam: true });
    }
  for (const { box, beat } of parts) {
    const u = toward(f, beat);
    if (u <= 0) continue;
    if (box.z <= TOP + 1e-6) out.push({ box: { ...box, h: box.h * u }, alpha: 1 });
    else out.push({ box: { ...box, z: box.z + 1 - u }, alpha: u });
  }
  for (const { box, beat } of paving) {
    const u = toward(f, beat);
    if (u > 0) out.push({ box: { ...box, z: box.z + 0.6 * (1 - u) }, alpha: u });
  }
  return [...out, ...files(f)];
}
