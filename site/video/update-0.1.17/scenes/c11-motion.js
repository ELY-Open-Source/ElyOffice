// c11 motion: eases, presses, shakes, cursor routes, tether copies.
import { kit } from '../lib/kit/index.js';
import { cubic } from '../lib/geom.js';

const { clamp, span, ease } = kit;

export function swing(u, s = 1.2) {
  const c = s * 1.525;
  const x = clamp(u);
  if (x < 0.5) return ((2 * x) ** 2 * ((c + 1) * 2 * x - c)) / 2;
  return ((2 * x - 2) ** 2 * ((c + 1) * (2 * x - 2) + c) + 2) / 2;
}

export function backOut(u, s = 1.4) {
  const x = clamp(u) - 1;
  return 1 + (s + 1) * x ** 3 + s * x ** 2;
}

export function press(t, at, down = 0.0625, up = 0.1875) {
  if (t < at - down || t > at + up) return 0;
  return t <= at ? ease.out(span(t, at - down, at)) : 1 - ease.inOut(span(t, at, at + up));
}

export const shake = (tau, amp, freq = 42, damp = 9) => (tau < 0 ? 0 : amp * Math.exp(-damp * tau) * Math.sin(freq * tau));

function bowed(a, b, v, bow) {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const n = Math.sin(Math.PI * clamp(v)) * bow;
  return [a[0] + dx * v - dy * n, a[1] + dy * v + dx * n];
}

// Cursor moves with eases and bows; holds between.
export function route(t, from, moves) {
  let p = from;
  for (const m of moves) {
    if (t < m.t0) return p;
    if (t < m.t1) return bowed(p, m.to, (m.ease ?? swing)(span(t, m.t0, m.t1)), m.bow ?? 0);
    p = m.to;
  }
  return p;
}

// The engine's tether curve, frozen into a lead stroke.
export function tetherCurve(tip, c, lag) {
  const d = Math.hypot(c[0] - tip.at[0], c[1] - tip.at[1]);
  if (d < 1) return [tip.at, tip.at];
  const h = clamp(d * 0.35, 6, 320);
  const back = [(tip.at[0] - c[0]) / d, (tip.at[1] - c[1]) / d];
  const p1 = [tip.at[0] + tip.tan[0] * h, tip.at[1] + tip.tan[1] * h];
  const p2 = [c[0] + (lag[0] - c[0]) * 1.4 + back[0] * h * 0.5, c[1] + (lag[1] - c[1]) * 1.4 + back[1] * h * 0.5];
  return cubic(tip.at, p1, p2, c, 6);
}

export function scaleAbout(strokes, s, [ox, oy], [dx, dy] = [0, 0]) {
  return kit.move(strokes, { s, ox, oy, x: dx, y: dy });
}

export const hide = (strokes) => strokes.map((st) => ({ ...st, show: [0, 0] }));
