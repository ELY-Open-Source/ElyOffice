// Hero line: wrong brush hand settles to serif, then sans.
import { kit } from '../lib/kit/index.js';
import { resample } from '../lib/geom.js';
import { rnd } from '../lib/rand.js';

export const LINE = '春眠不觉晓';
export const HERO = { x: 960, y: 372, size: 160 };
export const PIECES = 12;
const PER = 5;
const M = PIECES * PER + 1;
const BRUSH = 1.1;

const near = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);
const mean = (pts) => pts.reduce((s, p) => [s[0] + p[0] / pts.length, s[1] + p[1] / pts.length], [0, 0]);

function glyphs(font, size) {
  const out = [];
  for (const s of kit.text(LINE, { ...HERO, size, font, align: 'center' })) (out[s.g] ??= []).push(s.pts);
  return out;
}

function cost(a, b) {
  const pa = resample(a, 12);
  const pb = resample(b, 12);
  return pa.reduce((s, p, i) => s + near(p, pb[i]), 0) / 12 / HERO.size;
}

function match(A, B) {
  const pairs = [];
  A.forEach((a, i) => B.forEach((b, j) => pairs.push([cost(a, b), i, j])));
  pairs.sort((p, q) => p[0] - q[0]);
  const ab = new Map();
  const taken = new Set();
  for (const [c, i, j] of pairs) {
    if (c > 0.3 || ab.has(i) || taken.has(j)) continue;
    ab.set(i, j);
    taken.add(j);
  }
  return ab;
}

function nearest(curves, to) {
  let best = curves[0][0];
  for (const c of curves) for (const p of c) if (near(p, to) < near(best, to)) best = p;
  return best;
}

// Slots pair strokes; missing ones become twins or points.
function slots(A, B, C) {
  const ab = match(A, B);
  const bc = match(B, C);
  const usedB = new Set(ab.values());
  const usedC = new Set(bc.values());
  const list = [];
  A.forEach((a, i) => {
    const j = ab.get(i);
    list.push({ a, b: j === undefined ? null : B[j], c: j === undefined || !bc.has(j) ? null : C[bc.get(j)] });
  });
  B.forEach((b, j) => {
    if (!usedB.has(j)) list.push({ a: null, b, c: bc.has(j) ? C[bc.get(j)] : null });
  });
  C.forEach((c, k) => {
    if (!usedC.has(k)) list.push({ a: null, b: null, c });
  });
  const point = (fonts, from) => Array(M).fill(nearest(fonts, mean(from)));
  const twin = (fonts, from) => resample(fonts.reduce((p, q) => (cost(q, from) < cost(p, from) ? q : p)), M);
  return list.map((s) => {
    const ref = s.b ?? s.a ?? s.c;
    return {
      a: s.a ? resample(s.a, M) : point(A, ref),
      b: s.b ? resample(s.b, M) : s.a ? point(B, ref) : twin(B, ref),
      c: s.c ? resample(s.c, M) : s.b ? twin(C, ref) : point(C, ref),
      real: !!s.a,
    };
  });
}

// The wrong hand: each glyph leans, turns and drifts.
function hand(pts, g, box) {
  const cx = (box[0] + box[2]) / 2;
  const cy = (box[1] + box[3]) / 2;
  const r = (rnd('c09-hand', g, 0) - 0.5) * 0.26;
  const dy = (rnd('c09-hand', g, 1) - 0.5) * 44;
  const s = 0.94 + rnd('c09-hand', g, 2) * 0.2;
  const c = Math.cos(r) * s;
  const d = Math.sin(r) * s;
  return pts.map(([x, y]) => {
    const px = x - cx + 0.2 * (cy - y);
    const py = y - cy;
    return [cx + c * px - d * py, cy + dy + d * px + c * py];
  });
}

function bounds(curves) {
  const all = curves.flat();
  const xs = all.map((p) => p[0]);
  const ys = all.map((p) => p[1]);
  return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
}

const bump = (f, at, wd) => Math.exp(-(((f - at) / wd) ** 2));

function brushWidth(pts, g, n, j) {
  const f = (j + 0.5) / PIECES;
  const e = pts[M - 1];
  const p = pts[M - 9];
  const L = near(e, p) || 1;
  const dx = (e[0] - p[0]) / L;
  const dy = (e[1] - p[1]) / L;
  const base = 1.15 + 0.5 * rnd('c09-brush', g, n);
  let w = 0.62 + 1.8 * bump(f, 0.04, 0.16);
  if (dx < -0.3 && dy > 0.3) w *= 1.3 - 1.05 * f;
  else if (dx > 0.3 && dy > 0.3) w += 1.5 * bump(f, 0.84, 0.14);
  else w += 0.9 * bump(f, 0.95, 0.07);
  return base * w;
}

function serifWidth(pts, j) {
  const a = pts[j * PER];
  const b = pts[(j + 1) * PER];
  const h = Math.abs(b[0] - a[0]) / (Math.abs(b[0] - a[0]) + Math.abs(b[1] - a[1]) + 1e-6);
  const s = pts[0];
  const e = pts[M - 1];
  const across = Math.abs(e[0] - s[0]) / (Math.abs(e[0] - s[0]) + Math.abs(e[1] - s[1]) + 1e-6);
  if (across > 0.8 && j === PIECES - 1) return 1.6;
  if (across < 0.3 && j === 0) return 1.9;
  return 0.42 + 1.35 * (1 - h) ** 2;
}

function build() {
  const A = glyphs('brush', HERO.size * BRUSH);
  const B = glyphs('serif', HERO.size);
  const C = glyphs('sans', HERO.size);
  const x0 = HERO.x - kit.measure(LINE, { size: HERO.size, font: 'serif' }).w / 2;
  const span = 2 * (HERO.x - x0);
  const out = [];
  A.forEach((curves, g) => {
    const box = bounds(curves);
    slots(curves, B[g], C[g]).forEach((s, n) => {
      const a = hand(s.a, g, box);
      const [mx, my] = mean(s.b);
      const key = (mx - x0) / span + 0.12 * (my - HERO.y + HERO.size) / HERO.size;
      out.push({
        g, a, b: s.b, c: s.c, real: s.real, key,
        wa: [...Array(PIECES)].map((_, j) => brushWidth(s.a, g, n, j)),
        wb: [...Array(PIECES)].map((_, j) => serifWidth(s.b, j)),
      });
    });
  });
  const lo = Math.min(...out.map((s) => s.key));
  const hi = Math.max(...out.map((s) => s.key));
  return out.map((s) => ({ ...s, key: (s.key - lo) / (hi - lo) }));
}

export const SLOTS = build();
export const PIECE_COUNT = SLOTS.length * PIECES;

// In and out with a small wind-up and overshoot.
function swing(u) {
  const c = 0.9 * 1.525;
  if (u <= 0) return 0;
  if (u >= 1) return 1;
  return u < 0.5 ? ((2 * u) ** 2 * ((c + 1) * 2 * u - c)) / 2 : ((2 * u - 2) ** 2 * ((c + 1) * (2 * u - 2) + c) + 2) / 2;
}

export const WAVES = [
  { from: 79.5, spread: 0.25, dur: 0.25, lift: 18 },
  { from: 80.375, spread: 0.2, dur: 0.175, lift: 0 },
];

function phase(t, slot, wave) {
  const s = wave.from + wave.spread * slot.key;
  return kit.clamp((t - s) / wave.dur);
}

// Brush writing: slot n is written over [t0, t1].
export function writing(a, b) {
  const lens = SLOTS.map((s) => (s.real ? kit.length({ pts: s.a }) : 0));
  const gap = (i) => (i && SLOTS[i].g !== SLOTS[i - 1].g ? 0.6 : 0.25);
  const cost = lens.map((L, i) => (L ? L ** 0.8 + gap(i) * 40 : 0));
  const total = cost.reduce((p, q) => p + q, 0);
  let at = a;
  return lens.map((L, i) => {
    const d = ((b - a) * cost[i]) / total;
    const slot = L ? [at + (d * gap(i) * 40) / cost[i], at + d] : [a, a];
    at += d;
    return slot;
  });
}

export function hero(t, times) {
  const out = [];
  SLOTS.forEach((s, n) => {
    const v1 = phase(t, s, WAVES[0]);
    const v2 = phase(t, s, WAVES[1]);
    const e1 = s.real ? swing(v1) : kit.ease.out(v1);
    const e2 = swing(v2);
    const lift = WAVES[0].lift * Math.sin(Math.PI * v1);
    const pts = s.a.map((p, i) => [
      p[0] + (s.b[i][0] - p[0]) * e1 + (s.c[i][0] - s.b[i][0]) * e2,
      p[1] + (s.b[i][1] - p[1]) * e1 + (s.c[i][1] - s.b[i][1]) * e2 - lift,
    ]);
    const [t0, t1] = times[n];
    const r = s.real ? kit.ease.sine(kit.span(t, t0, t1)) : 1;
    for (let j = 0; j < PIECES; j++) {
      const w = s.wa[j] + (s.wb[j] - s.wa[j]) * kit.clamp(e1) + (1.1 - s.wb[j]) * kit.clamp(e2);
      const u = kit.clamp(r * PIECES - j);
      out.push({
        pts: pts.slice(j * PER, (j + 1) * PER + 1), w, alpha: 1, show: u > 0 ? [0, u] : [0, 0], app: 'doc',
        link: j ? 1 : 0.5, g: s.g,
      });
    }
  });
  return out;
}
