// Panels on one sheet: placement, transforms, the scene table.
import scenes from '../scenes/index.js';
import { kitFor } from './kit/index.js';
import { FPS, W, H } from './time.js';

export function toWorld(pl, x, y) {
  const c = Math.cos(pl.r) * pl.s;
  const d = Math.sin(pl.r) * pl.s;
  return [pl.x + c * x - d * y, pl.y + d * x + c * y];
}

export function toLocal(pl, X, Y) {
  const c = Math.cos(pl.r) / pl.s;
  const d = Math.sin(pl.r) / pl.s;
  const dx = X - pl.x;
  const dy = Y - pl.y;
  return [c * dx + d * dy, -d * dx + c * dy];
}

export function rotate(pl, v) {
  const c = Math.cos(pl.r);
  const d = Math.sin(pl.r);
  return [c * v[0] - d * v[1], d * v[0] + c * v[1]];
}

function corners(pl) {
  return [[0, 0], [W, 0], [W, H], [0, H]].map(([x, y]) => toWorld(pl, x, y));
}

function sheetIn(pl) {
  const pts = scenes.flatMap((s) => corners(s.place)).map(([X, Y]) => toLocal(pl, X, Y));
  const xs = pts.map((p) => p[0]);
  const ys = pts.map((p) => p[1]);
  const x0 = Math.min(...xs);
  const y0 = Math.min(...ys);
  return { x: x0, y: y0, w: Math.max(...xs) - x0, h: Math.max(...ys) - y0 };
}

function centreIn(pl, other) {
  return other ? toLocal(pl, ...toWorld(other.place, W / 2, H / 2)) : null;
}

export const entries = scenes.map((s, i) => {
  for (const key of ['id', 'start', 'end', 'app', 'place']) if (s[key] === undefined) throw new Error(`scene ${i} has no ${key}`);
  for (const key of ['strokes', 'pen']) if (typeof s[key] !== 'function') throw new Error(`${s.id} has no ${key}()`);
  const k = kitFor({ sheet: sheetIn(s.place), prev: centreIn(s.place, scenes[i - 1]), next: centreIn(s.place, scenes[i + 1]) });
  return { s, i, k, f0: Math.round(s.start * FPS), f1: Math.round(s.end * FPS) };
});

export function entryAt(t) {
  for (let i = entries.length - 1; i >= 0; i--) if (t >= entries[i].s.start) return entries[i];
  return entries[0];
}

export const filmEnd = () => entries[entries.length - 1].s.end;
