// The touch: "0.1.16" is written, then its 6 becomes 7.
import { kit } from '../lib/kit/index.js';
import { slice } from '../lib/geom.js';
import { wire, bloom } from './c01-wire.js';

const SUITE = ['doc', 'table', 'show', 'pdf', 'mail', 'tree', 'mark'];
const AT = { x: 930, y: 615, size: 260, align: 'center' };
const BLOOM = { r: 3.5, turns: 2 };
const RETRACE = 2.0;
const DOT = { r: 6, turns: 2.5, w: 1.5 };

// The 0 starts at its top and runs clockwise.
function seatTop(s) {
  const pts = s.pts.slice(0, -1);
  let top = 0;
  pts.forEach((p, i) => { if (p[1] < pts[top][1]) top = i; });
  return { ...s, pts: [...pts.slice(top), ...pts.slice(0, top), pts[top]].reverse() };
}

// A full stop as a solid spiral, not a ring.
function stop(s) {
  const n = s.pts.length;
  const c = s.pts.reduce((a, p) => [a[0] + p[0] / n, a[1] + p[1] / n], [0, 0]);
  return { ...s, pts: bloom([c[0], c[1] - DOT.r], DOT.r, DOT.turns, 1), w: DOT.w };
}

const text = kit.text('0.1.16', { ...AT, link: 0.35 }).map((s, i) => (i === 0 ? seatTop(s) : s.ch === '.' ? stop(s) : s));
const sixAt = text.findIndex((s) => s.g === 5);
const seven = kit.text('0.1.17', AT).find((s) => s.g === 5);
const morph = wire(text[sixAt].pts, [...seven.pts].reverse());
const touch = text[0].pts[0];

// The settled 7 in seven wet colours, top to foot.
const WET = SUITE.map((app, i) => ({ pts: slice(seven.pts, i / 7, (i + 1) / 7), w: RETRACE, alpha: 1, show: [0, 1], app, link: 0 }));

// Unseen under the ink: down the 7 and back up.
function back(pts) {
  return { pts: [...[...pts].reverse(), ...pts.slice(1)], w: 0.3, alpha: 0.002, show: [0, 1], app: null, link: 0 };
}

function layout(g, u) {
  const six = morph(u);
  const head = { pts: bloom(touch, BLOOM.r, BLOOM.turns, g), w: 1.7, alpha: 1, show: [0, 1], app: null };
  const body = text.map((s, i) => (i === sixAt ? { ...s, pts: six } : s));
  return [head, ...body, back(six), ...WET];
}

const before = kit.marks(layout(1, 0)).list;
const after = kit.marks(layout(1, 1));
const glyph = (g) => {
  const ids = text.map((s, i) => (s.g === g ? i + 1 : -1)).filter((i) => i > 0);
  return [before[ids[0]].start, before[ids.at(-1)].end, ids.map((i) => before[i])];
};
const L16 = before[sixAt + 1].end;
const ink = after.list[text.length + 2].start;
if (ink < L16) throw new Error(`c01: the retrace starts at ${ink.toFixed(0)}, inside ${L16.toFixed(0)}`);
const LB = before[0].end;
const FOOT = seven.pts.at(-1);

// Each glyph lands on a beat; the 1 lifts once.
function writing() {
  const keys = [[1.0, 0], [1.375, LB, 'out'], [1.5, LB]];
  const plan = [[0, 1.5, 2.375], [1, 2.5, 2.625], [2, 3.0, 3.375], [3, 3.5, 3.625], [4, 4.0, 4.375], [5, 4.5, 5.25]];
  for (const [g, a, b] of plan) {
    const [s, e, parts] = glyph(g);
    keys.push([a, s, 'sine']);
    if (parts.length === 2) {
      const mid = a + (b - a) * 0.62;
      keys.push([mid, parts[0].end], [mid + 0.0625, parts[1].start, 'linear']);
    }
    keys.push([b, e]);
  }
  return keys;
}
const PEN = [...writing(), [6.5, L16], [6.6875, ink], [6.75, ink], [7.25, after.total + 2]];

// Wind up, release on the beat, settle with a spring.
function unwind(t, k) {
  if (t < 6.0) return k.keys(t, [[5.75, 0], [6.0, -0.045, 'in']]);
  const tau = t - 6.0;
  const rise = 1 - Math.exp(-8.5 * tau) * (Math.cos(10 * tau) + (8.5 / 10) * Math.sin(10 * tau));
  return k.lerp(-0.045, 1, rise);
}

const growth = (t, k) => k.ease.out(k.span(t, 1.0, 1.375));

const lg = (t, k, frames) => Math.exp(k.keys(t, frames.map(([a, w, e]) => [a, Math.log(w), e])));

export default {
  id: 'c01', start: 0, end: 8, app: 'suite',
  strokes(t, k) {
    return layout(growth(t, k), unwind(t, k));
  },
  pen(t, k) {
    if (t <= 1.0) return 0;
    if (t < 1.375) return growth(t, k) * LB;
    return k.keys(t, PEN);
  },
  cursor(t, k) {
    if (t < 7.25) return null;
    const dip = [FOOT[0] - 8, FOOT[1] + 26];
    const dir = k.next ? Math.atan2(k.next[1] - dip[1], k.next[0] - dip[0]) : 0;
    const out = [2080, dip[1] + 30 + (2080 - dip[0]) * Math.tan(dir)];
    return { x: k.keys(t, [[7.25, FOOT[0]], [7.5, dip[0]], [8.0, out[0], 'in']]), y: k.keys(t, [[7.25, FOOT[1]], [7.5, dip[1]], [8.0, out[1], 'in']]) };
  },
  camera(t, k) {
    const near = { x: touch[0] + 100, y: touch[1] + 56, w: 600 };
    const x = k.keys(t, [[1.375, near.x], [5.0, 960], [5.75, 960], [6.5, 1010, 'out'], [7.0, 1010], [7.75, 1060]]);
    const y = k.keys(t, [[1.375, near.y], [5.0, 540], [5.75, 540], [6.5, 545, 'out']]);
    const w = lg(t, k, [[1.375, near.w], [5.0, 1920], [5.75, 1920], [6.5, 1760, 'out'], [7.0, 1760], [7.75, 1840]]);
    const tau = t - 1.0;
    const press = tau > 0 ? 1 - 0.018 * Math.exp(-5 * tau) * Math.sin(11 * tau) : 1;
    return { x, y, w: w * press, r: 0 };
  },
};
