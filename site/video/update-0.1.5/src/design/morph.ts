import { interpolate as flubber } from 'flubber';
import { interpolate } from 'remotion';
import { type AppId, EASE, ICON, MORPH, STAGE, STAGE_CENTER } from '../theme';
import {
  type Point,
  circle,
  contour,
  roundedRect,
  sdfCapsule,
  sdfCircle,
  sdfCubicStroke,
  sdfRoundedBox,
  toPath,
  union,
  arc,
} from './geometry';

export type ShapeId = AppId | 'dot' | 'cell';

// Silhouettes in Papirus grid units, centred on the grid centre (96, 96).
const centre = (points: Point[]): Point[] => points.map(([x, y]) => [x - 96, y - 96]);
const Q = Math.PI / 2;

const page = centre([
  [40, 16],
  [112, 16],
  [160, 64],
  ...arc(152, 168, 8, 0, Q, 8),
  ...arc(40, 168, 8, Q, 2 * Q, 8),
  ...arc(40, 24, 8, 2 * Q, 3 * Q, 8),
]);

const board = centre(
  contour(
    union([sdfRoundedBox(16, 28, 160, 112, 10), sdfCapsule(96, 136, 64, 176, 6), sdfCapsule(96, 136, 128, 176, 6)]),
    0,
    0,
    192,
    192,
    0.5,
  ),
);

const branch = centre(
  contour(
    union([
      sdfCapsule(64, 44, 64, 148, 10),
      sdfCubicStroke(
        [
          [64, 128],
          [64, 92],
          [128, 104],
          [128, 72],
        ],
        10,
      ),
      sdfCircle(64, 40, 24),
      sdfCircle(128, 64, 24),
      sdfCircle(64, 152, 24),
    ]),
    0,
    0,
    192,
    192,
    0.5,
  ),
);

// `color` fills the silhouette; `accent` marks the section heading.
export const SHAPES: Record<ShapeId, { points: Point[]; color: string; accent: string }> = {
  'ely-doc': { points: page, color: '#3D7FE5', accent: '#3D7FE5' },
  'ely-table': { points: page, color: '#2E9E5B', accent: '#2E9E5B' },
  'ely-pdf': { points: page, color: '#E0344F', accent: '#E0344F' },
  'ely-show': { points: board, color: '#F07B2A', accent: '#F07B2A' },
  'ely-tree': { points: branch, color: '#7E57C2', accent: '#7E57C2' },
  'ely-mark': { points: centre(roundedRect(28, 24, 136, 152, 10)), color: '#3E4A5C', accent: '#F2A900' },
  cell: { points: centre(roundedRect(16, 51, 160, 90, 8)), color: '#3E4A5C', accent: '#F2A900' },
  dot: { points: circle(15), color: '#3D7FE5', accent: '#3D7FE5' },
};

// The open stage, shrunk to icon size so shape and size can morph separately.
const PROXY_W = 168;
const PROXY_H = (PROXY_W * STAGE.h) / STAGE.w;
const PROXY_R = (STAGE.r * PROXY_W) / STAGE.w;
const proxy = roundedRect(-PROXY_W / 2, -PROXY_H / 2, PROXY_W, PROXY_H, PROXY_R, 10);
// Zoom at which the proxy is exactly the stage.
export const STAGE_ZOOM = STAGE.w / (PROXY_W * ICON.unit);

// Rings are resampled into four runs between corresponding anchors (the
// shape's outer corners), so corners travel to corners and nothing shears.
const RUN = 128;
const ANCHORS: Partial<Record<ShapeId, Point[]>> = {
  // The board's corners, not the stand's feet.
  'ely-show': centre([
    [16, 28],
    [176, 28],
    [176, 140],
    [16, 140],
  ]),
};

const resample = (line: Point[], count: number): Point[] => {
  const lengths = [0];
  for (let i = 1; i < line.length; i++) lengths.push(lengths[i - 1] + Math.hypot(line[i][0] - line[i - 1][0], line[i][1] - line[i - 1][1]));
  const total = lengths[lengths.length - 1];
  const out: Point[] = [];
  let j = 1;
  for (let i = 0; i < count; i++) {
    const at = (total * i) / count;
    while (j < line.length - 1 && lengths[j] < at) j++;
    const span = lengths[j] - lengths[j - 1] || 1;
    const u = (at - lengths[j - 1]) / span;
    out.push([line[j - 1][0] + (line[j][0] - line[j - 1][0]) * u, line[j - 1][1] + (line[j][1] - line[j - 1][1]) * u]);
  }
  return out;
};

const prepared = new Map<string, Point[]>();
const prepare = (id: ShapeId | 'stage'): Point[] => {
  const found = prepared.get(id);
  if (found) return found;
  let ring = id === 'stage' ? proxy : SHAPES[id].points;
  let area = 0;
  for (let i = 0; i < ring.length; i++) {
    const [ax, ay] = ring[i];
    const [bx, by] = ring[(i + 1) % ring.length];
    area += ax * by - bx * ay;
  }
  // Clockwise on screen.
  if (area < 0) ring = [...ring].reverse();
  const xs = ring.map((p) => p[0]);
  const ys = ring.map((p) => p[1]);
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const anchors = (id !== 'stage' && ANCHORS[id]) || [
    [x0, y0],
    [x1, y0],
    [x1, y1],
    [x0, y1],
  ];
  const nearest = ([ax, ay]: Point) => ring.reduce((best, p, i) => (Math.hypot(p[0] - ax, p[1] - ay) < Math.hypot(ring[best][0] - ax, ring[best][1] - ay) ? i : best), 0);
  const first = nearest(anchors[0]);
  ring = [...ring.slice(first), ...ring.slice(0, first)];
  const cuts = [0, ...anchors.slice(1).map(nearest), ring.length];
  if (!cuts.every((c, i) => i === 0 || c > cuts[i - 1])) throw new Error(`Anchors of ${id} are out of order`);
  const out: Point[] = [];
  for (let k = 0; k < 4; k++) out.push(...resample([...ring.slice(cuts[k], cuts[k + 1]), ring[cuts[k + 1] % ring.length]], RUN));
  prepared.set(id, out);
  return out;
};

type Ring = (t: number) => Point[];
const cache = new Map<string, Ring>();

const ring = (from: ShapeId | 'stage', to: ShapeId | 'stage'): Ring => {
  const key = `${from}>${to}`;
  let found = cache.get(key);
  if (!found) {
    // Equal, aligned rings: flubber neither bisects nor needs to rotate them.
    found = flubber(prepare(from), prepare(to), { string: false, maxSegmentLength: 0 });
    cache.set(key, found);
  }
  return found;
};

// Path of `from` turned `t` of the way into `to`, drawn at zoom `k` around (cx, cy).
export const shapePath = (from: ShapeId | 'stage', to: ShapeId | 'stage', t: number, k: number, cx: number, cy: number) => {
  const points = from === to ? prepare(from) : ring(from, to)(t);
  return toPath(points, ICON.unit * k, cx, cy);
};

// ---- Boundary choreography ------------------------------------------------

export type MorphState = {
  // 0: icon silhouette, 1: stage rectangle.
  s: number;
  // Zoom of the shape (1 = icon size, STAGE_ZOOM = stage size).
  k: number;
  cx: number;
  cy: number;
  // Opacity of the full-colour icon artwork over the silhouette.
  details: number;
  // Opacity of the app-colour wash over the stage content.
  fill: number;
  content: number;
  contentScale: number;
  blur: number;
  // 0: icon shadow, 1: stage shadow.
  lift: number;
  border: number;
  // 0..1 travel of the light sweep along the stage edge; null when idle.
  sweep: number | null;
};

const clamp = (v: number) => Math.min(1, Math.max(0, v));
const smooth = (v: number, a: number, b: number) => {
  const x = clamp((v - a) / (b - a));
  return x * x * (3 - 2 * x);
};
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

// The downbeat swell: the icon breathes in on the cut.
export const boundaryPulse = (d: number) => 0.04 * Math.exp(-((d / 6.5) ** 2));

// Icon resting on a cut, `d` frames from it (negative: before it).
export const restState = (d: number): MorphState => ({
  s: 0,
  k: 1 + boundaryPulse(d),
  cx: ICON.x,
  cy: ICON.y,
  details: 1 - smooth(Math.abs(d), 10, MORPH.hold),
  fill: 1,
  content: 0,
  contentScale: 0.72,
  blur: 14,
  lift: 0,
  border: 0,
  sweep: null,
});

// `o` is how open the stage is (0 icon, 1 stage), in each direction's own time.
const openState = (shape: number, zoom: number, o: number, sweep: number | null): MorphState => ({
  s: shape,
  k: Math.exp(Math.log(STAGE_ZOOM) * zoom),
  cx: lerp(ICON.x, STAGE_CENTER.x, zoom),
  cy: lerp(ICON.y, STAGE_CENTER.y, zoom),
  details: 0,
  fill: 1 - smooth(o, 0.32, 0.82),
  content: smooth(o, 0.22, 0.7),
  contentScale: lerp(0.72, 1, EASE.out(o)),
  blur: 14 * (1 - o) ** 2,
  lift: o,
  border: smooth(o, 0.8, 1),
  sweep,
});

// Section-local frame `f` of an entering stage.
export const enterState = (f: number): MorphState => {
  if (f < MORPH.hold) return restState(f);
  const o = clamp((f - MORPH.hold) / MORPH.morph);
  const shape = EASE.inOut(clamp(o / 0.6));
  const zoom = EASE.bloom(o);
  const sweep = o > 0.55 ? clamp((o - 0.55) / 0.45) : null;
  return openState(shape, zoom, zoom, sweep);
};

// Section-local frame `f` of a stage in a section `duration` frames long.
export const exitState = (f: number, duration: number): MorphState => {
  const d = f - duration;
  if (d >= -MORPH.hold) return restState(d);
  const q = clamp((d + MORPH.hold + MORPH.morph) / MORPH.morph);
  const zoom = 1 - EASE.land(q);
  const shape = 1 - EASE.inOut(clamp((q - 0.22) / 0.78));
  return openState(shape, zoom, zoom, null);
};

export const isOpen = (state: MorphState) => state.s >= 1 && state.k >= STAGE_ZOOM - 1e-6;

export const fade = (f: number, from: number, to: number, ease = EASE.out) =>
  interpolate(f, [from, to], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp', easing: ease });
