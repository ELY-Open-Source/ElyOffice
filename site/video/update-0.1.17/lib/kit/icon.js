// App icons from assets/icons, redrawn as line art strokes.
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import { elements } from './svg.js';
import { stroke, rect, circle } from './shapes.js';
import { densify } from '../geom.js';

const DIR = fileURLToPath(new URL('../../../../../assets/icons/', import.meta.url));
const BOX = 192;

const RECIPES = {
  doc: [[0, 'outline'], [1, 'open'], [2, 'centre'], [3, 'centre'], [4, 'centre'], [5, 'centre'], [6, 'centre']],
  table: [[0, 'outline'], [1, 'open'], [2, 'outline']],
  show: [[2, 'outline'], [3, 'bottom'], [4, 'outline'], [5, 'outline'], [6, 'outline'], [7, 'outline'], [8, 'outline'],
    [0, 'centre', [2]], [1, 'centre', [2]]],
  pdf: [[0, 'outline'], [1, 'open'], [2, 'outline'], [3, 'outline'], [4, 'outline'], [5, 'centre']],
  mail: [[0, 'outline'], [2, 'centre'], [3, 'centre'], [4, 'centre']],
  tree: [[2, 'ring'], [0, 'centre', [2, 6]], [6, 'ring'], [1, 'centre', [2, 4, 6]], [4, 'ring']],
  mark: [[0, 'outline'], [1, 'bottom'], [2, 'outline'], [3, 'outline'], [4, 'outline'], [5, 'outline'], [6, 'outline'],
    [7, 'outline'], [8, 'outline']],
};
const RING = 0.73;
const parsed = new Map();

function load(app) {
  if (!RECIPES[app]) throw new Error(`no icon recipe for ${app}; known: ${Object.keys(RECIPES).join(' ')}`);
  if (!parsed.has(app)) parsed.set(app, elements(fs.readFileSync(`${DIR}ely-${app}.svg`, 'utf8')));
  return parsed.get(app);
}

function points(e) {
  if (e.kind === 'path') return e.subs.map((s) => (s.closed ? [...s.pts, s.pts[0]] : s.pts));
  if (e.kind === 'polyline') return [e.pts];
  if (e.kind === 'circle') return [circle(e.x, e.y, e.r).pts];
  return [rect(e.x, e.y, e.w, e.h, { r: e.r }).pts];
}

function bbox(list) {
  const all = list.flat();
  const xs = all.map((p) => p[0]);
  const ys = all.map((p) => p[1]);
  return { x0: Math.min(...xs), y0: Math.min(...ys), x1: Math.max(...xs), y1: Math.max(...ys) };
}

function inside(p, e) {
  if (e.kind === 'circle') return Math.hypot(p[0] - e.x, p[1] - e.y) < e.r * RING + 7;
  return p[0] > e.x && p[0] < e.x + e.w && p[1] > e.y && p[1] < e.y + e.h;
}

function outside(pts, shapes) {
  const runs = [];
  let cur = [];
  for (const p of densify(pts, 1)) {
    if (shapes.some((e) => inside(p, e))) {
      if (cur.length > 2) runs.push(cur);
      cur = [];
    } else cur.push(p);
  }
  if (cur.length > 2) runs.push(cur);
  return runs;
}

function shape(els, [index, mode, clip]) {
  const e = els[index];
  if (!e) throw new Error(`icon element ${index} is missing`);
  if (mode === 'ring') return [circle(e.x, e.y, e.r * RING).pts];
  if (mode === 'bottom') {
    const b = bbox(points(e));
    return [[[b.x0, b.y1], [b.x1, b.y1]]];
  }
  if (mode === 'centre' && e.kind === 'rect') {
    const flat = e.w >= e.h;
    const r = Math.min(e.w, e.h) / 2;
    return [flat ? [[e.x + r, e.y + r], [e.x + e.w - r, e.y + r]] : [[e.x + r, e.y + r], [e.x + r, e.y + e.h - r]]];
  }
  if (mode === 'open') return e.subs.map((s) => s.pts);
  const list = points(e);
  return clip ? list.flatMap((pts) => outside(pts, clip.map((i) => els[i]))) : list;
}

export function icon(app, x, y, size, o = {}) {
  const els = load(app);
  const k = size / BOX;
  const out = [];
  for (const step of RECIPES[app]) {
    for (const pts of shape(els, step)) out.push(stroke(pts.map(([px, py]) => [x + (px - BOX / 2) * k, y + (py - BOX / 2) * k]), o));
  }
  return out;
}

export const ICONS = Object.keys(RECIPES);
