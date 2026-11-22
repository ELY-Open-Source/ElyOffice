// c03 shapes: engine coil, pill, tick comb, corner rect.
import { kit } from '../lib/kit/index.js';
import { resample } from '../lib/geom.js';

export const TAU = Math.PI * 2;

export const COIL = { loops: 5, pitch: 19, rx: 12, ry: 21 };

// A sideways spring; from and to cut it by fraction.
export function coil(cx, cy, { pitch = COIL.pitch, ry = COIL.ry, from = 0, to = 1 } = {}) {
  const { loops, rx } = COIL;
  const span = pitch * loops;
  const n = Math.max(2, Math.ceil(loops * 48 * (to - from)));
  const pts = [];
  for (let i = 0; i <= n; i++) {
    const th = TAU * loops * (from + ((to - from) * i) / n);
    pts.push([cx - span / 2 + (pitch * th) / TAU - rx * Math.sin(th), cy - ry * Math.cos(th)]);
  }
  return pts;
}

function quarter(cx, cy, r, a0, a1, n, out) {
  for (let i = 1; i <= n; i++) {
    const a = a0 + ((a1 - a0) * i) / n;
    out.push([cx + r * Math.cos(a), cy + r * Math.sin(a)]);
  }
}

// Closed pill from its left end; flat at h 0.
export function pill(cx, cy, w, h) {
  const r = Math.max(0, h / 2);
  const x0 = cx - w / 2 + r;
  const x1 = cx + w / 2 - r;
  const q = Math.PI / 2;
  const pts = [[cx - w / 2, cy]];
  quarter(x0, cy, r, Math.PI, Math.PI + q, 10, pts);
  pts.push([x1, cy - r]);
  quarter(x1, cy, r, -q, q, 20, pts);
  pts.push([x0, cy + r]);
  quarter(x0, cy, r, q, Math.PI, 10, pts);
  return pts;
}

// Ticks scrolling left; total length never changes.
export function comb(x0, y, T, n, h, phase) {
  const xs = [];
  for (let j = 0; j < n; j++) xs.push((((j * T) / n - phase) % T + T) % T);
  xs.sort((a, b) => a - b);
  const pts = [[x0, y]];
  for (const x of xs) {
    const d = (h * (1 - Math.cos((TAU * x) / T))) / 2;
    pts.push([x0 + x, y], [x0 + x, y - d], [x0 + x, y]);
  }
  pts.push([x0 + T, y]);
  return pts;
}

// A rounded rect from below its top-right corner, clockwise.
export function cornerRect(x, y, w, h, r) {
  const pts = kit.rect(x, y, w, h, { r }).pts.slice(0, -1);
  const i = pts.findIndex((p) => Math.hypot(p[0] - (x + w), p[1] - (y + r)) < 1e-6);
  if (i < 0) throw new Error('c03: rect has no top-right start');
  const out = [...pts.slice(i), ...pts.slice(0, i)];
  return [...out, out[0]];
}

export function pair(a, b, n = 160) {
  return [resample(a, n), resample(b, n)];
}

// Unclamped blend, so eased overshoot carries through.
export function blend([a, b], u) {
  return a.map((p, i) => [p[0] + (b[i][0] - p[0]) * u, p[1] + (b[i][1] - p[1]) * u]);
}

export function scaleAbout(pts, cx, cy, s) {
  return pts.map(([x, y]) => [cx + (x - cx) * s, cy + (y - cy) * s]);
}

// Damped sine: zero at tau 0, so motion stays continuous.
export function wobble(tau, amount, freq = 12, damp = 6) {
  return tau < 0 ? 0 : amount * Math.exp(-damp * tau) * Math.sin(freq * tau);
}
