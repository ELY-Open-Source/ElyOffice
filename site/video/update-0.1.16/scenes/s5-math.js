// Handwritten math for s5: strokes, layout, a pen route.
import { spline } from '../lib/ink.js';
import { resample } from '../lib/stroke.js';
import { clamp } from '../lib/time.js';

const G = {
  a: { w: 0.56, s: [[[0.37, -0.4], [0.24, -0.5], [0.08, -0.4], [0.03, -0.17], [0.12, -0.01], [0.27, -0.05], [0.37, -0.25], [0.39, -0.5], [0.38, -0.2], [0.41, -0.02], [0.5, -0.05]]] },
  b: { w: 0.52, s: [[[0.12, -0.8], [0.09, -0.4], [0.06, 0], [0.09, -0.3], [0.24, -0.47], [0.4, -0.38], [0.43, -0.16], [0.3, -0.01], [0.1, -0.05]]] },
  c: { w: 0.48, s: [[[0.38, -0.42], [0.24, -0.5], [0.08, -0.38], [0.04, -0.16], [0.15, -0.01], [0.3, -0.02], [0.41, -0.1]]] },
  e: { w: 0.5, s: [[[0.06, -0.24], [0.38, -0.28], [0.36, -0.43], [0.22, -0.5], [0.07, -0.38], [0.04, -0.17], [0.15, -0.01], [0.32, -0.02], [0.43, -0.1]]] },
  i: { w: 0.3, s: [[[0.06, -0.46], [0.1, -0.22], [0.11, -0.03], [0.2, -0.06]], [[0.09, -0.7], [0.11, -0.67]]] },
  p: { w: 0.7, s: [[[0.0, -0.43], [0.14, -0.5], [0.48, -0.49], [0.62, -0.54]], [[0.18, -0.48], [0.16, -0.2], [0.09, 0]], [[0.44, -0.48], [0.43, -0.16], [0.48, -0.01], [0.57, -0.05]]] },
  2: { w: 0.48, s: [[[0.05, -0.55], [0.18, -0.71], [0.34, -0.68], [0.38, -0.52], [0.28, -0.34], [0.04, 0], [0.2, -0.03], [0.44, 0.0]]] },
  1: { w: 0.36, s: [[[0.06, -0.55], [0.2, -0.72], [0.19, -0.35], [0.18, 0]]] },
  0: { w: 0.5, s: [[[0.25, -0.72], [0.08, -0.6], [0.03, -0.3], [0.12, -0.02], [0.3, -0.01], [0.43, -0.3], [0.4, -0.6], [0.25, -0.72], [0.17, -0.67]]] },
  '+': { w: 0.56, s: [[[0.04, -0.3], [0.48, -0.32]], [[0.26, -0.54], [0.25, -0.07]]] },
  '=': { w: 0.6, s: [[[0.04, -0.41], [0.52, -0.42]], [[0.05, -0.2], [0.53, -0.22]]] },
  $: { w: 0.48, s: [[[0.38, -0.62], [0.24, -0.7], [0.08, -0.6], [0.1, -0.42], [0.3, -0.34], [0.4, -0.16], [0.26, -0.02], [0.06, -0.08]], [[0.23, -0.84], [0.22, 0.12]]] },
};

const SUP = { k: 0.58, up: 0.46 };
const pad = (ch) => (ch === '+' || ch === '=' ? 0.12 : 0.02);

// Items: a char, or [char, 'sup'].
export function lay(items, { x, y, size, align = 'left' }) {
  let pen = 0;
  const raw = items.map((it) => {
    const [ch, sup] = Array.isArray(it) ? it : [it];
    const k = sup ? SUP.k : 1;
    const up = sup ? SUP.up : 0;
    pen += pad(ch);
    const g = G[ch];
    const strokes = g.s.map((st) => st.map(([u, v]) => [pen + u * k, v * k - up]));
    const box = [pen, pen + g.w * k];
    pen += g.w * k + pad(ch);
    return { ch, strokes, box, sup: !!sup };
  });
  const dx = x - (align === 'center' ? (pen * size) / 2 : 0);
  return raw.map((g) => {
    const strokes = g.strokes.map((st) => {
      const pts = st.map(([u, v]) => [dx + u * size, y + v * size]);
      return pts.length > 2 ? spline(pts, 8) : pts;
    });
    let x0 = Infinity;
    let y0 = Infinity;
    let x1 = -Infinity;
    let y1 = -Infinity;
    for (const st of strokes) for (const [px, py] of st) {
      x0 = Math.min(x0, px);
      y0 = Math.min(y0, py);
      x1 = Math.max(x1, px);
      y1 = Math.max(y1, py);
    }
    return { ...g, strokes, at: { x0, y0, x1, y1 } };
  });
}

const shift = (pts, [dx, dy]) => pts.map(([x, y]) => [x + dx, y + dy]);
export const moved = (g, off) => ({ ...g, strokes: g.strokes.map((s) => shift(s, off)) });

// One pen route through every stroke; air moves faster.
export function route(list, air = 0.35) {
  const legs = [];
  let prev = null;
  list.forEach((g, gi) => g.strokes.forEach((st, si) => {
    if (prev) legs.push({ pts: [prev, st[0]], ink: false });
    legs.push({ pts: st, ink: true, gi, si });
    prev = st[st.length - 1];
  }));
  let total = 0;
  for (const l of legs) {
    l.path = resample(l.pts, 2);
    l.len = l.path[l.path.length - 1][2] || 1;
    l.cost = l.len * (l.ink ? 1 : air);
    l.a = total;
    total += l.cost;
  }
  for (const l of legs) {
    l.a /= total;
    l.b = l.a + l.cost / total;
  }
  return legs;
}

export function along(legs, u) {
  const v = clamp(u);
  const l = legs.find((g) => v <= g.b) ?? legs[legs.length - 1];
  const f = clamp((v - l.a) / (l.b - l.a));
  const s = f * l.len;
  const p = l.path;
  let k = 1;
  while (k < p.length - 1 && p[k][2] < s) k++;
  const [ax, ay, as] = p[k - 1];
  const [bx, by, bs] = p[k];
  const q = bs > as ? (s - as) / (bs - as) : 0;
  return { x: ax + (bx - ax) * q, y: ay + (by - ay) * q, leg: l, f };
}

// Drawn share of each stroke at u.
export function drawn(legs, u) {
  const out = new Map();
  for (const l of legs) if (l.ink) out.set(`${l.gi}.${l.si}`, clamp((u - l.a) / (l.b - l.a)));
  return out;
}

// When the route finishes each glyph, as route fractions.
export function finishes(legs) {
  const end = new Map();
  for (const l of legs) if (l.ink) end.set(l.gi, l.b);
  return [...end.values()];
}
