// Orthographic boxes turning about the desk's vertical axis.

const COS = Math.cos(Math.PI / 6);
const SIN = Math.sin(Math.PI / 6);
const EPS = 1e-6;

/** A box in grid units, with its light tone. */
export interface Box {
  x: number;
  y: number;
  z: number;
  w: number;
  d: number;
  h: number;
  tone: string;
}

/** A box as drawn: alpha and seam stroke. */
export interface Shown {
  box: Box;
  alpha: number;
  seam?: boolean;
}

/** World turn and the camera's scale and offset. */
export interface View {
  turn: number;
  cx: number;
  cy: number;
  s: number;
  ox: number;
  oy: number;
}

type Point = [number, number];

/** Grid point to frame pixels. */
export function project(v: View, x: number, y: number, z: number): Point {
  const [c, s] = [Math.cos(v.turn), Math.sin(v.turn)];
  const [dx, dy] = [x - v.cx, y - v.cy];
  const X = c * dx - s * dy;
  const Y = s * dx + c * dy;
  return [v.ox + v.s * (X - Y) * COS, v.oy + v.s * ((X + Y) * SIN - z)];
}

/** Frame pixel to the ground plane at height z. */
export function ground(v: View, px: number, py: number, z = 0): Point {
  const a = (px - v.ox) / (v.s * COS);
  const b = ((py - v.oy) / v.s + z) / SIN;
  const [X, Y] = [(a + b) / 2, (b - a) / 2];
  const [c, s] = [Math.cos(v.turn), Math.sin(v.turn)];
  return [v.cx + c * X + s * Y, v.cy - s * X + c * Y];
}

/** Hex colour to linear oklab. */
function oklab(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  const lin = (u: number) => (u <= 0.04045 ? u / 12.92 : ((u + 0.055) / 1.055) ** 2.4);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((u) => lin(u / 255));
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const q = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * q,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * q,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * q,
  ];
}

function rgb([L, A, B]: [number, number, number]): string {
  const l = (L + 0.3963377774 * A + 0.2158037573 * B) ** 3;
  const m = (L - 0.1055613458 * A - 0.0638541728 * B) ** 3;
  const q = (L - 0.0894841775 * A - 1.291485548 * B) ** 3;
  const out = [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * q,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * q,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * q,
  ].map((u) => {
    const c = Math.min(1, Math.max(0, u));
    const s = c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055;
    return Math.round(s * 255);
  });
  return `rgb(${out.join(",")})`;
}

const shades = new Map<string, string>();

/** A tone mixed toward black by `k`, in oklab. */
export function shade(hex: string, k: number): string {
  const key = `${hex}:${k.toFixed(3)}`;
  let found = shades.get(key);
  if (!found) {
    found = rgb(oklab(hex).map((u) => u * (1 - k)) as [number, number, number]);
    shades.set(key, found);
  }
  return found;
}

/** Two tones mixed in oklab. */
export function mix(a: string, b: string, u: number): string {
  const [p, q] = [oklab(a), oklab(b)];
  return rgb(p.map((v, i) => v + (q[i] - v) * u) as [number, number, number]);
}

/** Side darkness: 9% facing +y, 20% facing +x. */
function darkness(nx: number, ny: number): number {
  return 0.145 - 0.0778 * Math.cos(Math.atan2(ny, nx) - (3 * Math.PI) / 4);
}

function hull(points: Point[]): Point[] {
  const p = [...points].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const cross = (o: Point, a: Point, b: Point) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const half = (list: Point[]) => {
    const out: Point[] = [];
    for (const q of list) {
      while (out.length >= 2 && cross(out[out.length - 2], out[out.length - 1], q) <= 0) out.pop();
      out.push(q);
    }
    out.pop();
    return out;
  };
  return [...half(p), ...half([...p].reverse())];
}

function fill(ctx: CanvasRenderingContext2D, pts: Point[], color: string, seam = 0) {
  ctx.beginPath();
  pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
  ctx.closePath();
  ctx.fillStyle = color;
  ctx.fill();
  if (seam) {
    ctx.strokeStyle = color;
    ctx.lineWidth = seam;
    ctx.stroke();
  }
}

/** One box: silhouette, then its visible faces. */
function paint(ctx: CanvasRenderingContext2D, v: View, { box: b, alpha, seam }: Shown) {
  if (b.h <= EPS || alpha <= 0) return;
  const p = (i: number, j: number, k: number) => project(v, b.x + i * b.w, b.y + j * b.d, b.z + k * b.h);
  const [c, s] = [Math.cos(v.turn), Math.sin(v.turn)];
  const sides: [number, number, Point[]][] = [
    [1, 0, [p(1, 0, 0), p(1, 1, 0), p(1, 1, 1), p(1, 0, 1)]],
    [-1, 0, [p(0, 0, 0), p(0, 1, 0), p(0, 1, 1), p(0, 0, 1)]],
    [0, 1, [p(0, 1, 0), p(1, 1, 0), p(1, 1, 1), p(0, 1, 1)]],
    [0, -1, [p(0, 0, 0), p(1, 0, 0), p(1, 0, 1), p(0, 0, 1)]],
  ];
  const seen = sides.flatMap(([x, y, pts]) => {
    const [nx, ny] = [c * x - s * y, s * x + c * y];
    return nx + ny > EPS ? [{ pts, k: darkness(nx, ny) }] : [];
  });
  ctx.globalAlpha = alpha;
  if (alpha >= 1) {
    const all = [0, 1].flatMap((i) => [0, 1].flatMap((j) => [0, 1].map((k) => p(i, j, k))));
    fill(ctx, hull(all), shade(b.tone, Math.max(...seen.map((f) => f.k))));
  }
  for (const f of seen) fill(ctx, f.pts, shade(b.tone, f.k), seam ? 1 : 0);
  fill(ctx, [p(0, 0, 1), p(1, 0, 1), p(1, 1, 1), p(0, 1, 1)], b.tone, seam ? 1.2 : 0);
  ctx.globalAlpha = 1;
}

function bounds(v: View, b: Box): [number, number, number, number] {
  let [x0, x1, y0, y1] = [Infinity, -Infinity, Infinity, -Infinity];
  for (const i of [0, 1])
    for (const j of [0, 1])
      for (const k of [0, 1]) {
        const [x, y] = project(v, b.x + i * b.w, b.y + j * b.d, b.z + k * b.h);
        [x0, x1, y0, y1] = [Math.min(x0, x), Math.max(x1, x), Math.min(y0, y), Math.max(y1, y)];
      }
  return [x0, x1, y0, y1];
}

/** True if `a` draws first, false if `b`, else null. */
function behind(a: Box, b: Box, vx: number, vy: number): boolean | null {
  const axes: [number, number, number, number, number][] = [
    [a.x, a.w, b.x, b.w, vx],
    [a.y, a.d, b.y, b.d, vy],
    [a.z, a.h, b.z, b.h, 1],
  ];
  for (const [p, pw, q, qw, dir] of axes) {
    if (Math.abs(dir) < EPS) continue;
    if (p + pw <= q + EPS) return dir > 0;
    if (q + qw <= p + EPS) return dir < 0;
  }
  return null;
}

/** Nearest bottom corner's depth toward the viewer. */
function depth(v: View, b: Box): number {
  const [c, s] = [Math.cos(v.turn), Math.sin(v.turn)];
  let best = -Infinity;
  for (const i of [0, 1])
    for (const j of [0, 1]) {
      const [dx, dy] = [b.x + i * b.w - v.cx, b.y + j * b.d - v.cy];
      best = Math.max(best, (c + s) * dx + (c - s) * dy);
    }
  return best + b.z;
}

/** Back to front: overlaps decide, depth breaks ties. */
function order(v: View, list: Shown[]): Shown[] {
  const [c, s] = [Math.cos(v.turn), Math.sin(v.turn)];
  const [vx, vy] = [c + s, c - s];
  const rects = list.map((e) => bounds(v, e.box));
  const keys = list.map((e) => depth(v, e.box));
  const after: number[][] = list.map(() => []);
  const waits = list.map(() => 0);
  for (let i = 0; i < list.length; i++)
    for (let j = i + 1; j < list.length; j++) {
      const [a, b] = [rects[i], rects[j]];
      if (a[1] <= b[0] + 0.5 || b[1] <= a[0] + 0.5 || a[3] <= b[2] + 0.5 || b[3] <= a[2] + 0.5) continue;
      const first = behind(list[i].box, list[j].box, vx, vy);
      if (first === null) continue;
      const [from, to] = first ? [i, j] : [j, i];
      after[from].push(to);
      waits[to]++;
    }
  const out: Shown[] = [];
  const done = list.map(() => false);
  while (out.length < list.length) {
    let next = -1;
    for (let i = 0; i < list.length; i++)
      if (!done[i] && waits[i] === 0 && (next < 0 || keys[i] < keys[next])) next = i;
    if (next < 0) {
      for (let i = 0; i < list.length; i++) if (!done[i] && (next < 0 || keys[i] < keys[next])) next = i;
      waits[next] = 0;
    }
    done[next] = true;
    out.push(list[next]);
    for (const k of after[next]) waits[k]--;
  }
  return out;
}

/** Draws every box in painter's order. */
export function boxes(ctx: CanvasRenderingContext2D, v: View, list: Shown[]): void {
  for (const e of order(v, list)) paint(ctx, v, e);
}

/** A ground cell's diamond, squashed about its centre. */
export function cell(ctx: CanvasRenderingContext2D, v: View, i: number, j: number, u: number, color: string): void {
  const pts = [project(v, i, j, 0), project(v, i + 1, j, 0), project(v, i + 1, j + 1, 0), project(v, i, j + 1, 0)];
  const my = pts.reduce((sum, [, y]) => sum + y, 0) / 4;
  fill(ctx, pts.map(([x, y]) => [x, my + (y - my) * u] as Point), color, 1);
}
