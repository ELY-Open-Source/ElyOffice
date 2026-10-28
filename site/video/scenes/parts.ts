// Film additions: desk tiles, screens, files, props.
import type { Block } from "../../src/iso.ts";
import { ACCENT, model } from "../../src/model.ts";

export type Box = Required<Block>;

const desk = model[0].blocks[0];
export const DESK = desk.tone;
export const PAPER = model[1].blocks[0].tone;
/** The blank face a recording is later pinned onto. */
export const SCREEN = "#3a3a3c";
const TOP = desk.h;

const on = (x: number, y: number, w: number, d: number, h: number, z = 0, tone = PAPER): Box => ({
  x, y, z: TOP + z, w, d, h, tone,
});

/** The desk as one-cell tiles, for growing it. */
export const tiles: Box[] = Array.from({ length: desk.w * desk.d }, (_, i) => ({
  x: i % desk.w, y: Math.floor(i / desk.w), z: 0, w: 1, d: 1, h: desk.h, tone: DESK,
}));

/** Upright 16:9 screens on each app; +y face is pinned. */
const screen = (x: number, y: number, w: number, z: number) => on(x, y, w, 0.08, (w * 9) / 16, z, SCREEN);
export const screens: Record<string, Box> = {
  ElyDoc: screen(1.24, 2.2, 2.6, 1),
  ElyTable: screen(5, 1.4, 2.9, 2.4),
  ElyShow: on(9.4, 1.25, 3.2, 0.05, 1.8, 0.4, SCREEN),
  ElyPDF: screen(13, 4.3, 2.4, 1),
  // Flush with the trunk front: hides the road edge behind.
  ElyTree: screen(6.5, 8.22, 2.9, 2.8),
  ElyMark: screen(1, 7.4, 3, 2),
};

/** A file travelling a road, one hop per beat. */
export interface Trip {
  path: [number, number][];
  lands: number;
  hops: number[];
}

const FILE = 0.48;
const ROAD = TOP + 0.08;

/** Mark to Tree first, then four exports queue at PDF. */
export const trips: Trip[] = [
  { path: [[4.26, 7.9], [5.2, 7.9]], lands: 136.5, hops: [137, 138] },
  { path: [[11, 3.1], [11, 4.9], [12.74, 4.9]], lands: 136.75, hops: [139, 140] },
  { path: [[6.3, 4.16], [6.3, 4.9], [12.14, 4.9]], lands: 137, hops: [139, 140, 141, 142] },
  { path: [[2.3, 4.26], [2.3, 4.9], [11.54, 4.9]], lands: 137.5, hops: [139, 140, 141, 142, 143, 144] },
  { path: [[2.3, 6.74], [2.3, 4.9], [10.94, 4.9]], lands: 137.75, hops: [139, 140, 141, 142, 143, 144] },
];

/** The point `s` along a polyline. */
function along(path: [number, number][], s: number): [number, number] {
  for (let i = 1; i < path.length; i++) {
    const [[ax, ay], [bx, by]] = [path[i - 1], path[i]];
    const length = Math.hypot(bx - ax, by - ay);
    if (s <= length || i === path.length - 1) {
      const u = Math.min(s / length, 1);
      return [ax + (bx - ax) * u, ay + (by - ay) * u];
    }
    s -= length;
  }
  throw new Error("a path needs two points");
}

/** Total length of a polyline. */
export function length(path: [number, number][]): number {
  return path.slice(1).reduce((sum, [x, y], i) => sum + Math.hypot(x - path[i][0], y - path[i][1]), 0);
}

/** The file box with its centre `s` along the trip. */
export function file(trip: Trip, s: number): Box {
  const [x, y] = along(trip.path, s);
  return { x: x - FILE / 2, y: y - FILE / 2, z: ROAD, w: FILE, d: FILE, h: FILE, tone: ACCENT };
}

/** A lighter tone for paper sheets. */
export const SHEET = "#fbfbfa";

/** The lamp the dark spreads from; lands in bar 35. */
export const LAMP = { beat: 138, centre: [14.4, 1.9] as [number, number] };
export const lamp: Box[] = [
  on(14.4, 1.4, 1, 1, 0.15),
  on(14.8, 1.8, 0.2, 0.2, 2.4, 0.15),
  on(13.9, 1.8, 0.9, 0.2, 0.15, 2.4),
  on(13.6, 1.55, 0.7, 0.7, 0.45, 1.95),
];

/** Three props on plinths before the desk, built on sixteenths. */
export interface Prop {
  bar: number;
  caption: string;
  blocks: Box[];
}

/** A prop's blocks on a 4x4 plinth at (ox, oy). */
function plinth(ox: number, oy: number, list: [number, number, number, number, number, number, string?][]): Box[] {
  const base: Box = { x: ox, y: oy, z: 0, w: 4, d: 4, h: TOP, tone: DESK };
  const boxes = [base, ...list.map(([x, y, w, d, h, z, tone]) => on(ox + x, oy + y, w, d, h, z, tone ?? PAPER))];
  return boxes.map(snap);
}

/** Snaps to 1/64 so touching faces compare exactly. */
function snap(b: Box): Box {
  const r = (v: number) => Math.round(v * 64) / 64;
  const [x, y, z] = [r(b.x), r(b.y), r(b.z)];
  return { x, y, z, w: r(b.x + b.w) - x, d: r(b.y + b.d) - y, h: r(b.z + b.h) - z, tone: b.tone };
}

export const props: Prop[] = [
  {
    bar: 36,
    caption: "offline",
    blocks: plinth(1.5, 12.2, [
      [0.3, 0.4, 0.3, 1.8, 1, 0, SHEET],
      [0.3, 0.4, 0.3, 1.8, 1, 1, SHEET],
      [0.6, 1, 0.2, 0.6, 0.6, 0.3, SCREEN],
      [3.4, 3.45, 0.3, 0.45, 0.4, 0, ACCENT],
      [3.4, 2.45, 0.3, 1, 0.4, 0, ACCENT],
      [3.4, 1.45, 0.3, 1, 0.4, 0, ACCENT],
      [2.9, 1.15, 0.8, 0.3, 0.4, 0, ACCENT],
      [1.85, 0.95, 1.05, 0.7, 1.2, 0, SHEET],
      [1.6, 1.1, 0.25, 0.4, 0.4, 0.4, SCREEN],
    ]),
  },
  {
    bar: 37,
    caption: "open",
    blocks: plinth(6.7, 12.2, [
      [0.6, 0.6, 1.3, 1.3, 0.15, 0],
      [1.9, 0.6, 1.3, 1.3, 0.15, 0],
      [0.6, 1.9, 1.3, 1.3, 0.15, 0],
      [1.9, 1.9, 1.3, 1.3, 0.15, 0],
      [0.6, 0.6, 2.6, 0.15, 1.3, 0.15],
      [0.6, 0.75, 0.15, 2.45, 1.3, 0.15],
      [3.05, 0.75, 0.15, 2.3, 1.3, 0.15],
      [0.75, 3.05, 2.45, 0.15, 1.3, 0.15],
      [1.4, 1.4, 1, 1, 1.7, 0.15, ACCENT],
      [0.6, 0.45, 2.6, 0.15, 2, 1.45, SHEET],
      [0.75, 3.2, 2.45, 0.75, 0.06, 0],
      [3.2, 0.75, 0.75, 2.3, 0.06, 0],
    ]),
  },
  {
    bar: 38,
    caption: "local",
    blocks: plinth(11.9, 12.2, [
      [0.4, 1.6, 0.8, 0.2, 2.6, 0],
      [1.2, 1.6, 0.8, 0.2, 2.6, 0],
      [2, 1.6, 0.8, 0.2, 2.6, 0],
      [2.8, 1.6, 0.8, 0.2, 2.6, 0],
      [0.4, 1.6, 1.2, 0.2, 0.35, 2.6, ACCENT],
      [0.7, 1.8, 2.6, 0.15, 2.9, 0, SHEET],
      [1, 1.95, 2, 0.03, 0.1, 2.6],
      [1, 1.95, 1.6, 0.03, 0.1, 2.35],
      [1, 1.95, 1.8, 0.03, 0.1, 2.1],
      [0.4, 1.98, 0.8, 0.2, 2, 0],
      [1.2, 1.98, 0.8, 0.2, 2, 0],
      [2, 1.98, 0.8, 0.2, 2, 0],
      [2.8, 1.98, 0.8, 0.2, 2, 0],
    ]),
  },
];

/** A screen grown about its base; Show stands apart. */
export function grown(app: string): Box {
  if (app === "ElyShow") return { x: 8.6, y: 2, z: model[0].blocks[0].h + 0.25, w: 4.8, d: 0.08, h: 2.7, tone: SCREEN };
  const b = screens[app];
  const w = b.w * 1.5;
  return { ...b, x: b.x + (b.w - w) / 2, w, h: b.h * 1.5 };
}
