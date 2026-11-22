// The 6 unwinds like wire into a 7; ink blooms.
import { resample } from '../lib/geom.js';

const N = 140;
const STAGGER = 0.8;
const OVER = 0.35;
const lerp = (a, b, u) => a + (b - a) * u;
const unit = (v) => Math.max(0, Math.min(1, v));

function intrinsic(pts) {
  const p = resample(pts, N);
  const h = [];
  const c = [];
  for (let i = 0; i < N - 1; i++) {
    const dx = p[i + 1][0] - p[i][0];
    const dy = p[i + 1][1] - p[i][1];
    let a = Math.atan2(dy, dx);
    if (i) a += 2 * Math.PI * Math.round((h[i - 1] - a) / (2 * Math.PI));
    h.push(a);
    c.push(Math.hypot(dx, dy));
  }
  return { p, h, c };
}

// Headings blend head first: the 6 coils, then unfurls.
export function wire(from, to) {
  const A = intrinsic(from);
  const B = intrinsic(to);
  const turn = 2 * Math.PI * Math.round((A.h[0] - B.h[0]) / (2 * Math.PI));
  const hb = B.h.map((v) => v + turn);
  return (u) => {
    const lu = unit(u);
    const ends = (i) => [lerp(A.p[i][0], B.p[i][0], u), lerp(A.p[i][1], B.p[i][1], u)];
    const out = [ends(0)];
    for (let i = 0; i < N - 1; i++) {
      const w = u < 0 ? u : u > 1 ? 1 + (u - 1) * OVER : unit(u * (1 + STAGGER) - (STAGGER * i) / (N - 2));
      const a = lerp(A.h[i], hb[i], w);
      const l = lerp(A.c[i], B.c[i], lu);
      out.push([out[i][0] + l * Math.cos(a), out[i][1] + l * Math.sin(a)]);
    }
    const end = ends(N - 1);
    const ex = end[0] - out[N - 1][0];
    const ey = end[1] - out[N - 1][1];
    return out.map((q, i) => [q[0] + (ex * i) / (N - 1), q[1] + (ey * i) / (N - 1)]);
  };
}

// A clockwise spiral spreading from the touch, ending on target.
export function bloom(at, r, turns, g) {
  const c = [at[0], at[1] + r];
  const pts = [];
  const n = 48;
  for (let i = 0; i <= n; i++) {
    const s = i / n;
    const a = -Math.PI / 2 - 2 * Math.PI * turns * (1 - s);
    const rr = g * r * s ** 0.7;
    pts.push([c[0] + rr * Math.cos(a), c[1] + rr * Math.sin(a)]);
  }
  return pts;
}
