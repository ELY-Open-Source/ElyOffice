// Camera: rests per scene, arcs between panels on cuts.
import { entries, entryAt, toWorld } from './world.js';
import { FPS, W, H, ease, lerp } from './time.js';

export const REST = { x: W / 2, y: H / 2, w: W, r: 0 };
const BEFORE = 0.25;
const AFTER = 0.75;

function rest(e, t) {
  const c = e.s.camera ? e.s.camera(t, e.k) ?? REST : REST;
  for (const key of ['x', 'y', 'w']) if (!Number.isFinite(c[key])) throw new Error(`${e.s.id}: camera(${t}).${key} is ${c[key]}`);
  const pl = e.s.place;
  const [x, y] = toWorld(pl, c.x, c.y);
  return { x, y, w: c.w * pl.s, r: (c.r ?? 0) + pl.r };
}

function flight(a, b, u) {
  const v = ease.inOut(u);
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const bow = Math.sin(Math.PI * v) * 0.14;
  const hop = 1 + 0.55 * Math.sin(Math.PI * v) * Math.min(1, Math.hypot(dx, dy) / Math.max(a.w, b.w));
  return {
    x: lerp(a.x, b.x, v) - dy * bow,
    y: lerp(a.y, b.y, v) + dx * bow,
    w: Math.exp(lerp(Math.log(a.w), Math.log(b.w), v)) * hop,
    r: lerp(a.r, b.r, v),
  };
}

// Both ends keep moving, so velocity carries through the flight.
function fly(a, b, t) {
  const S = b.s.start;
  const u = (t - (S - BEFORE)) / (BEFORE + AFTER);
  return flight(rest(a, Math.min(t, S - 1 / FPS)), rest(b, Math.max(t, S)), u);
}

export function cameraAt(n) {
  const t = n / FPS;
  const e = entryAt(t);
  const next = entries[e.i + 1];
  if (next && t >= next.s.start - BEFORE) return fly(e, next, t);
  if (e.i > 0 && t < e.s.start + AFTER) return fly(entries[e.i - 1], e, t);
  return rest(e, t);
}

export function matrix(cam) {
  const z = W / cam.w;
  const a = z * Math.cos(cam.r);
  const b = -z * Math.sin(cam.r);
  const c = z * Math.sin(cam.r);
  const d = z * Math.cos(cam.r);
  return { a, b, c, d, e: W / 2 - (a * cam.x + c * cam.y), f: H / 2 - (b * cam.x + d * cam.y), z };
}
