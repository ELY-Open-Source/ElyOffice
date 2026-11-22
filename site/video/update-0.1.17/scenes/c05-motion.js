// Tape folds, tumbling glyphs, the reserve coil, small transforms.
import { densify, cumulative, at } from '../lib/geom.js';
import { ease, clamp } from '../lib/time.js';

const PI = Math.PI;

export function fold(pts, c0, pc) {
  const dx = c0[0] - pc[0];
  const dy = c0[1] - pc[1];
  const D = Math.hypot(dx, dy);
  if (D < 0.05) return pts;
  const n = [dx / D, dy / D];
  const r = Math.min(13, D / 4);
  const s0 = (D + PI * r) / 2;
  const fx = c0[0] - n[0] * s0;
  const fy = c0[1] - n[1] * s0;
  return densify(pts, 4).map(([x, y]) => {
    const s = (x - fx) * n[0] + (y - fy) * n[1];
    if (s <= 0) return [x, y];
    const d = s < PI * r ? r * Math.sin(s / r) : PI * r - s;
    return [x + n[0] * (d - s), y + n[1] * (d - s)];
  });
}

export function foldAll(strokes, c0, pc) {
  return strokes.map((s) => ({ ...s, pts: fold(s.pts, c0, pc) }));
}

export function foldX(c0, pc) {
  const D = Math.hypot(c0[0] - pc[0], c0[1] - pc[1]);
  return c0[0] + (D + PI * Math.min(13, D / 4)) / 2;
}

export const settle = (tau, amp, f = 17, d = 9) => (tau <= 0 ? 0 : amp * Math.exp(-d * tau) * Math.sin(f * tau));

export function shift(strokes, dx, dy) {
  if (!dx && !dy) return strokes;
  return strokes.map((s) => ({ ...s, pts: s.pts.map(([x, y]) => [x + dx, y + dy]) }));
}

export function fade(strokes, a) {
  return a >= 1 ? strokes : strokes.map((s) => ({ ...s, alpha: (s.alpha ?? 1) * a }));
}

export function box(strokes) {
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const s of strokes) for (const [x, y] of s.pts) {
    x0 = Math.min(x0, x);
    y0 = Math.min(y0, y);
    x1 = Math.max(x1, x);
    y1 = Math.max(y1, y);
  }
  return { x0, y0, x1, y1, cx: (x0 + x1) / 2, cy: (y0 + y1) / 2 };
}

export function byGlyph(strokes) {
  const map = new Map();
  strokes.forEach((s, i) => {
    if (!map.has(s.g)) map.set(s.g, []);
    map.get(s.g).push(i);
  });
  return [...map.values()];
}

export function tumble(strokes, c, to, u, spin) {
  const e = ease.inOut(clamp(u));
  const flip = Math.cos(2 * PI * ease.inOut(clamp(u * 1.08)));
  const hop = 2 * (1 - e) * e;
  const ox = hop * to[0] * 0.5 + e * e * to[0];
  const oy = hop * -70 + e * e * to[1];
  const r = spin * Math.sin(PI * e);
  const cr = Math.cos(r);
  const sr = Math.sin(r);
  return strokes.map((s) => ({
    ...s,
    pts: s.pts.map(([x, y]) => {
      const qx = x - c[0];
      const qy = (y - c[1]) * flip;
      return [c[0] + ox + cr * qx - sr * qy, c[1] + oy + sr * qx + cr * qy];
    }),
  }));
}

export function tipOf(s) {
  const c = cumulative(s.pts);
  return at(s.pts, c, c[c.length - 1] * s.show[1]);
}

export const TOOTH = 8;

export function coil(P, dir, R) {
  const m = Math.floor(R / (2 * TOOTH));
  const rem = (R - 2 * TOOTH * m) / 2;
  const back = (l) => [P[0] - dir[0] * l, P[1] - dir[1] * l];
  const pts = [P];
  for (let i = 0; i < m; i++) pts.push(back(TOOTH), P);
  if (rem > 1e-6) pts.push(back(rem), P);
  return pts;
}

export function rectFrom(rect, at) {
  const pts = rect.pts;
  let best = 0;
  let q = pts[0];
  let cost = Infinity;
  for (let i = 0; i < pts.length - 1; i++) {
    const [ax, ay] = pts[i];
    const dx = pts[i + 1][0] - ax;
    const dy = pts[i + 1][1] - ay;
    const u = clamp(((at[0] - ax) * dx + (at[1] - ay) * dy) / (dx * dx + dy * dy || 1));
    const p = [ax + dx * u, ay + dy * u];
    const c = Math.hypot(p[0] - at[0], p[1] - at[1]);
    if (c < cost) {
      cost = c;
      best = i;
      q = p;
    }
  }
  return { ...rect, pts: [q, ...pts.slice(best + 1, -1), ...pts.slice(0, best + 1), q] };
}

export function unit(dx, dy, fx = 1, fy = 0) {
  const d = Math.hypot(dx, dy);
  return d > 1e-6 ? [dx / d, dy / d] : [fx, fy];
}
