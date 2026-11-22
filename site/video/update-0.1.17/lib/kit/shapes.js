// Primitive strokes, each starting where a hand would.
import { resample, length } from '../geom.js';
import { clamp, ease } from '../time.js';

export function stroke(pts, o = {}) {
  return { pts, w: o.w ?? 1, alpha: o.alpha ?? 1, show: o.show ?? [0, 1], app: o.app ?? null };
}

export function line(x0, y0, x1, y1, o) {
  return stroke([[x0, y0], [x1, y1]], o);
}

export function poly(points, o = {}) {
  return stroke(o.closed ? [...points, points[0]] : points, o);
}

function arcPts(cx, cy, r, from, to) {
  const n = Math.max(8, Math.ceil((Math.abs(to - from) * r) / 4));
  const out = [];
  for (let i = 0; i <= n; i++) {
    const a = from + ((to - from) * i) / n;
    out.push([cx + r * Math.cos(a), cy + r * Math.sin(a)]);
  }
  return out;
}

export function arc(x, y, r, from, to, o) {
  return stroke(arcPts(x, y, r, from, to), o);
}

export function circle(x, y, r, o = {}) {
  const from = o.from ?? -Math.PI / 2;
  return stroke(arcPts(x, y, r, from, o.to ?? from - Math.PI * 2), o);
}

export function rect(x, y, w, h, o = {}) {
  const r = Math.max(0, Math.min(o.r ?? 0, w / 2, h / 2));
  if (r === 0) return stroke([[x, y], [x + w, y], [x + w, y + h], [x, y + h], [x, y]], o);
  const q = Math.PI / 2;
  const pts = [
    [x + r, y],
    ...arcPts(x + w - r, y + r, r, -q, 0),
    ...arcPts(x + w - r, y + h - r, r, 0, q),
    ...arcPts(x + r, y + h - r, r, q, 2 * q),
    ...arcPts(x + r, y + r, r, 2 * q, 3 * q),
  ];
  return stroke(pts, o);
}

export function check(x, y, size, on, o = {}) {
  const u = typeof on === 'number' ? clamp(on) : on ? 1 : 0;
  const box = rect(x, y, size, size, { ...o, r: size * 0.22 });
  const tick = stroke([[x + size * 0.24, y + size * 0.53], [x + size * 0.43, y + size * 0.72], [x + size * 0.78, y + size * 0.3]],
    { ...o, show: [0, u] });
  return [box, tick];
}

export function morph(a, b, u) {
  const n = Math.max(16, Math.ceil(Math.max(length(a.pts), length(b.pts)) / 3));
  const pa = resample(a.pts, n);
  const pb = resample(b.pts, n);
  const v = clamp(u);
  return { ...a, pts: pa.map((p, i) => [p[0] + (pb[i][0] - p[0]) * v, p[1] + (pb[i][1] - p[1]) * v]) };
}

export function ripple(x, y, u, o = {}) {
  const r0 = o.r ?? 14;
  const grow = ease.out(clamp(u));
  const s = circle(x, y, r0 + r0 * 1.6 * grow, { w: 0.6, ...o });
  s.show = u <= 0 || u >= 1 ? [0, 0] : [clamp((u - 0.45) / 0.55), 1];
  return s;
}

export function move(strokes, m = {}) {
  const { x = 0, y = 0, s = 1, r = 0, ox = 0, oy = 0 } = m;
  const c = Math.cos(r) * s;
  const d = Math.sin(r) * s;
  return strokes.map((st) => ({
    ...st,
    pts: st.pts.map(([px, py]) => [ox + x + c * (px - ox) - d * (py - oy), oy + y + d * (px - ox) + c * (py - oy)]),
  }));
}

export function leave(strokes, u) {
  const lens = strokes.map((st) => length(st.pts) * (st.show[1] - st.show[0]));
  let cut = clamp(u) * lens.reduce((a, b) => a + b, 0);
  const out = strokes.map((st) => ({ ...st }));
  for (let i = out.length - 1; i >= 0 && cut > 0; i--) {
    const take = Math.min(cut, lens[i]);
    const [a, b] = out[i].show;
    const end = lens[i] > 0 ? b - ((b - a) * take) / lens[i] : a;
    out[i].show = end - a > 1e-6 ? [a, end] : [0, 0];
    cut -= take;
  }
  return out;
}
