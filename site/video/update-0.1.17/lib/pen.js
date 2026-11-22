// Pen tables: drawn length per frame, inverse draw times, speed.
import { FPS, clamp } from './time.js';

function table(e) {
  if (!e.table) {
    const n = e.f1 - e.f0 + 1;
    const v = new Float64Array(n);
    let top = 0;
    for (let i = 0; i < n; i++) {
      const p = e.s.pen((e.f0 + i) / FPS, e.k);
      if (!Number.isFinite(p)) throw new Error(`${e.s.id}: pen(${(e.f0 + i) / FPS}) is ${p}`);
      top = Math.max(top, p);
      v[i] = top;
    }
    e.table = v;
  }
  return e.table;
}

export function penAt(e, f) {
  const v = table(e);
  return v[clamp(f - e.f0, 0, v.length - 1)];
}

export function drawTime(e, d) {
  const v = table(e);
  if (d <= v[0]) return e.f0 / FPS;
  let lo = 0;
  let hi = v.length - 1;
  if (d > v[hi]) return e.f1 / FPS;
  while (hi - lo > 1) {
    const m = (lo + hi) >> 1;
    if (v[m] < d) lo = m;
    else hi = m;
  }
  const u = v[hi] > v[lo] ? (d - v[lo]) / (v[hi] - v[lo]) : 1;
  return (e.f0 + lo + u) / FPS;
}

export function speedAt(e, tau) {
  const v = table(e);
  const i = tau * FPS - e.f0;
  const a = clamp(Math.floor(i) - 3, 0, v.length - 1);
  const b = clamp(Math.floor(i) + 4, 0, v.length - 1);
  return b > a ? ((v[b] - v[a]) * FPS) / (b - a) : 0;
}

export const pressure = (speed) => clamp(1.35 - speed / 2600, 0.55, 1.35);
