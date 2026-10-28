// Turns the model about the desk centre, like the montage.
import { project } from "../../src/iso.ts";
import { model } from "../../src/model.ts";
import recs from "../montage/recs.json" with { type: "json" };
import { type Box, grown } from "./parts.ts";

const SVG = "http://www.w3.org/2000/svg";
export const PIVOT = [8, 5.5];
const COS = Math.cos(Math.PI / 6);

/** A world point turned by `a`, then projected. */
export function turned(a: number, x: number, y: number, z: number): [number, number] {
  const [dx, dy] = [x - PIVOT[0], y - PIVOT[1]];
  return project(PIVOT[0] + dx * Math.cos(a) - dy * Math.sin(a), PIVOT[1] + dx * Math.sin(a) + dy * Math.cos(a), z);
}

/** The montage's grown screens, which leave in bar 35. */
export const slabs: Box[] = recs.apps.map(({ app }) => grown(app));

/** A leaving slab's recording at frame `f`, if any. */
export function recording(box: Box, land: number, f: number): string | null {
  const slab = slabs.indexOf(box);
  if (slab < 0 || land <= 0) return null;
  return `rec/${recs.apps[slab].rec}/${String(f - recs.first + 1).padStart(5, "0")}.jpg`;
}

/** A slab's recording frame, pinned affinely on its +y face. */
export function film(b: Box, a: number, href: string): { el: SVGImageElement; ready: Promise<void> } {
  const Y = b.y + b.d;
  const [o, r, d] = [turned(a, b.x, Y, b.z + b.h), turned(a, b.x + b.w, Y, b.z + b.h), turned(a, b.x, Y, b.z)];
  const [w, h] = [1280, 720];
  const el = document.createElementNS(SVG, "image");
  const m = [(r[0] - o[0]) / w, (r[1] - o[1]) / w, (d[0] - o[0]) / h, (d[1] - o[1]) / h, o[0], o[1]];
  el.setAttribute("width", String(w));
  el.setAttribute("height", String(h));
  el.setAttribute("preserveAspectRatio", "none");
  el.setAttribute("transform", `matrix(${m.map((v) => v.toFixed(5)).join(" ")})`);
  const ready = new Promise<void>((ok, fail) => {
    el.addEventListener("load", () => ok());
    el.addEventListener("error", () => fail(new Error(`no frame ${href}`)));
  });
  el.setAttribute("href", href);
  return { el, ready };
}

/** The montage's fixed framing, in this renderer's camera terms. */
export const montage = (() => {
  const boxes = [...model.flatMap((p) => p.blocks), ...slabs, ...slabs.map((b) => ({ ...b, z: b.z + 1 }))];
  const xs: number[] = [];
  const ys: number[] = [];
  for (let i = 0; i <= 90; i++) {
    const a = (-i * Math.PI) / 180;
    for (const b of boxes) for (const [x, y, z] of corners(b)) {
      const [dx, dy] = [x - PIVOT[0], y - PIVOT[1]];
      const [rx, ry] = [dx * Math.cos(a) - dy * Math.sin(a), dx * Math.sin(a) + dy * Math.cos(a)];
      xs.push((rx - ry) * COS);
      ys.push((rx + ry) / 2 - z);
    }
  }
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const S = Math.min(1800 / (x1 - x0), 1000 / (y1 - y0));
  const [X, Y] = [960 - (S * (x0 + x1)) / 2, 540 - (S * (y0 + y1)) / 2];
  const [cx, cy] = [(PIVOT[0] - PIVOT[1]) * COS, (PIVOT[0] + PIVOT[1]) / 2];
  const [, unit] = project(0, 0, -1);
  return { s: S / unit, x: X - S * cx, y: Y - S * cy };
})();

function corners(b: Box): [number, number, number][] {
  return [0, 1].flatMap((i) => [0, 1].flatMap((j) => [0, 1].map((k) => [b.x + i * b.w, b.y + j * b.d, b.z + k * b.h] as [number, number, number])));
}

/** Side darkness by turned normal: +y 9%, +x 20%. */
function darkness(nx: number, ny: number): number {
  const u = Math.atan2(ny, nx) - Math.PI / 2;
  return 0.145 - 0.055 * (Math.cos(u) + Math.sin(u));
}

function polygon(points: [number, number][], fill: string): SVGPolygonElement {
  const el = document.createElementNS(SVG, "polygon");
  el.setAttribute("points", points.map(([x, y]) => `${x.toFixed(2)},${y.toFixed(2)}`).join(" "));
  el.style.fill = fill;
  return el;
}

/** Convex hull of points, for the seam-hiding outline. */
function hull(points: [number, number][]): [number, number][] {
  const p = [...points].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const cross = (o: number[], a: number[], b: number[]) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const half = (list: [number, number][]) => {
    const out: [number, number][] = [];
    for (const q of list) {
      while (out.length >= 2 && cross(out[out.length - 2], out[out.length - 1], q) <= 0) out.pop();
      out.push(q);
    }
    return out.slice(0, -1);
  };
  return [...half(p), ...half([...p].reverse())];
}

/** A block turned by `a`: visible sides, then the top. */
export function turnedBlock(b: Box, a: number): SVGGElement {
  const P = (x: number, y: number, z: number) => turned(a, x, y, z);
  const [x, y, z, X, Y, Z] = [b.x, b.y, b.z, b.x + b.w, b.y + b.d, b.z + b.h];
  const sides: { n: [number, number]; c: [number, number, number][] }[] = [
    { n: [1, 0], c: [[X, y, Z], [X, Y, Z], [X, Y, z], [X, y, z]] },
    { n: [-1, 0], c: [[x, Y, Z], [x, y, Z], [x, y, z], [x, Y, z]] },
    { n: [0, 1], c: [[X, Y, Z], [x, Y, Z], [x, Y, z], [X, Y, z]] },
    { n: [0, -1], c: [[x, y, Z], [X, y, Z], [X, y, z], [x, y, z]] },
  ];
  const shade = (d: number) => `color-mix(in oklab, ${b.tone}, black ${(d * 100).toFixed(2)}%)`;
  const g = document.createElementNS(SVG, "g");
  g.append(polygon(hull(corners(b).map(([px, py, pz]) => P(px, py, pz))), shade(0.09)));
  for (const { n, c } of sides) {
    const [nx, ny] = [n[0] * Math.cos(a) - n[1] * Math.sin(a), n[0] * Math.sin(a) + n[1] * Math.cos(a)];
    if (nx + ny <= 1e-9) continue;
    g.append(polygon(c.map(([px, py, pz]) => P(px, py, pz)), shade(darkness(nx, ny))));
  }
  g.append(polygon([P(x, y, Z), P(X, y, Z), P(X, Y, Z), P(x, Y, Z)], b.tone));
  return g;
}

/** Back to front for a view turned by `a`. */
export function turnedOrder(list: Box[], a: number): number[] {
  const view = [Math.cos(a) + Math.sin(a), Math.cos(a) - Math.sin(a)];
  const rect = (b: Box) => {
    const pts = corners(b).map(([x, y, z]) => turned(a, x, y, z));
    const xs = pts.map(([px]) => px);
    const ys = pts.map(([, py]) => py);
    return [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  };
  const rects = list.map(rect);
  const meet = (i: number, j: number) =>
    rects[i][0] < rects[j][1] - 0.01 && rects[j][0] < rects[i][1] - 0.01 && rects[i][2] < rects[j][3] - 0.01 && rects[j][2] < rects[i][3] - 0.01;
  const lo = (b: Box) => [b.x, b.y, b.z];
  const hi = (b: Box) => [b.x + b.w, b.y + b.d, b.z + b.h];
  // Separating axes agree when the blocks truly overlap.
  const behind = (p: Box, q: Box): boolean | null => {
    const sign = [view[0], view[1], 1];
    const votes = new Set<boolean>();
    for (let k = 0; k < 3; k++) {
      if (Math.abs(sign[k]) < 1e-6) continue;
      if (hi(p)[k] <= lo(q)[k]) votes.add(sign[k] > 0);
      if (hi(q)[k] <= lo(p)[k]) votes.add(sign[k] < 0);
    }
    return votes.size === 1 ? [...votes][0] : null;
  };
  const before = list.map((q, i) => list.flatMap((p, j) => (j !== i && meet(i, j) && behind(p, q) === true ? [j] : [])));
  const done = new Set<number>();
  const out: number[] = [];
  while (out.length < list.length) {
    const next = list.findIndex((_, i) => !done.has(i) && before[i].every((j) => done.has(j)));
    if (next < 0) throw new Error("turned blocks occlude in a cycle");
    done.add(next);
    out.push(next);
  }
  return out;
}
