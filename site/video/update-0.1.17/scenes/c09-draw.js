// Drawing by reveal: schedules, travel lines, the pen table.
import { kit } from '../lib/kit/index.js';
import { cubic, startTangent } from '../lib/geom.js';
import { FPS } from '../lib/time.js';

// Each stroke gets a slot of time, by length.
export function plan(strokes, a, b, gap = 18, exp = 0.85) {
  const lens = strokes.map((s) => kit.length(s) * (s.show[1] - s.show[0]));
  const cost = lens.map((L) => (L > 0.5 ? L ** exp + gap : 0));
  const total = cost.reduce((p, q) => p + q, 0);
  let at = a;
  return cost.map((c) => {
    const d = total ? ((b - a) * c) / total : 0;
    const slot = [at + (d * gap) / Math.max(c, 1e-9), at + d];
    at += d;
    return c ? slot : [a, a];
  });
}

export function reveal(strokes, times, t, how = 'sine') {
  return strokes.map((s, i) => {
    const [t0, t1] = times[i];
    const u = t1 > t0 ? kit.ease[how](kit.span(t, t0, t1)) : t >= t0 ? 1 : 0;
    const [a, b] = s.show;
    return { ...s, show: u > 0 && b > a ? [a, a + (b - a) * u] : [0, 0] };
  });
}

// A light pen-up line leaving along out.
export function travel(p0, p3, tin, out) {
  const h = Math.hypot(p3[0] - p0[0], p3[1] - p0[1]) * 0.45;
  const n = Math.hypot(...out);
  const p1 = [p0[0] + (out[0] / n) * h, p0[1] + (out[1] / n) * h];
  const g = Math.min(h, 140);
  const p2 = [p3[0] - tin[0] * g, p3[1] - tin[1] * g];
  return { pts: cubic(p0, p1, p2, p3, 8), w: 0.4, alpha: 0.45, show: [0, 1], app: null, link: 1 };
}

export const tangentIn = (s) => startTangent(s.pts);

// Pen: the running longest of what has been revealed.
export function penTable(id, start, end, length) {
  let table = null;
  return (t) => {
    if (!table) {
      const f0 = Math.round(start * FPS);
      const n = Math.round(end * FPS) - f0 + 1;
      const lens = [];
      for (let i = 0; i < n; i++) {
        const L = length((f0 + i) / FPS);
        if (!Number.isFinite(L)) throw new Error(`${id}: path length at ${(f0 + i) / FPS} is ${L}`);
        lens.push(L);
      }
      // The gate samples even frames; odd ones stay below.
      table = new Float64Array(n);
      const even = new Float64Array(n);
      for (let i = 0; i < n; i++) even[i] = Math.max(i ? even[i - 1] : 0, (f0 + i) % 2 ? 0 : lens[i]);
      let top = 0;
      for (let i = 0; i < n; i++) {
        top = Math.max(top, lens[i]);
        table[i] = (f0 + i) % 2 && i + 1 < n ? Math.min(top, even[i + 1]) : even[i];
      }
    }
    const i = Math.round(t * FPS - start * FPS);
    return table[Math.max(0, Math.min(table.length - 1, i))];
  };
}

// Cursor from timed waypoints; null outside its spans.
export function track(spans, t) {
  for (const s of spans) {
    if (t >= s.from && t < s.to) return { x: kit.keys(t, s.keys.map(([k, p, e]) => [k, p[0], e])), y: kit.keys(t, s.keys.map(([k, p, e]) => [k, p[1], e])) };
  }
  return null;
}
