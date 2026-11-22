// c10 pen and cursor tables, computed once per frame index.
import { kit } from '../lib/kit/index.js';
import { join } from '../lib/line.js';
import { FPS } from '../lib/time.js';

const near = (t) => Math.round(t * FPS);

function pointAt(J, D) {
  let lo = 0;
  let hi = J.s.length - 1;
  if (hi < 0) return null;
  if (D >= J.s[hi]) return [J.x[hi], J.y[hi]];
  while (hi - lo > 1) {
    const m = (lo + hi) >> 1;
    if (J.s[m] < D) lo = m;
    else hi = m;
  }
  const u = (D - J.s[lo]) / (J.s[hi] - J.s[lo] || 1);
  return [J.x[lo] + (J.x[hi] - J.x[lo]) * u, J.y[lo] + (J.y[hi] - J.y[lo]) * u];
}

function weighted(m, list, a, b, u, weight) {
  const parts = [];
  let prev = a;
  m.list.forEach((r, i) => {
    if (r.end <= a || r.start >= b || r.end - r.start < 1e-9) return;
    const s = Math.max(a, prev);
    const e = Math.min(b, r.end);
    if (e > s) parts.push([s, e, weight(list[i])]);
    prev = Math.max(prev, r.end);
  });
  const total = parts.reduce((acc, [s, e, k]) => acc + (e - s) * k, 0);
  let want = u * total;
  for (const [s, e, k] of parts) {
    if (want <= (e - s) * k) return s + want / k;
    want -= (e - s) * k;
  }
  return b;
}

const startOf = (phase, m, ends) => {
  const i = ends[phase.keys[0][1]][0];
  return phase.from === 'jump' ? m.list[i].start : m.list[i - 1]?.end ?? 0;
};

function target(phase, t, m, list, ends) {
  let a = startOf(phase, m, ends);
  if (t <= phase.keys[0][0]) return a;
  for (let i = 1; i < phase.keys.length; i++) {
    const ta = phase.keys[i - 1][0];
    const [tb, name, how = 'inOut'] = phase.keys[i];
    const b = m.list[ends[name][1] - 1].end;
    if (t <= tb) return weighted(m, list, a, b, kit.ease[how]((t - ta) / (tb - ta)), phase.weight);
    a = b;
  }
  return a;
}

export function tables({ start, end, build, ends, phases, way }) {
  let memo = null;
  return () => {
    if (memo) return memo;
    const f0 = near(start);
    const n = near(end) - f0 + 1;
    const pen = new Float64Array(n);
    const tip = new Array(n).fill(null);
    const curls = {};
    let top = 0;
    for (let f = f0; f < f0 + n; f++) {
      const t = f / FPS;
      const p = phases.find((q) => t >= q.keys[0][0] - 1e-9 && t <= q.keys.at(-1)[0] + 1e-9);
      let L = build(t, curls);
      let m = kit.marks(L);
      if (p && p.from === 'jump' && !(p.id in curls)) {
        const owe = top - startOf(p, m, ends);
        curls[p.id] = owe > -2 ? owe + 6 : 0;
        L = build(t, curls);
        m = kit.marks(L);
      }
      const v = p ? target(p, t, m, L, ends) : m.total;
      top = Math.max(top, Math.min(v, m.total));
      pen[f - f0] = t < phases[0].keys[0][0] + 1e-9 ? 0 : top;
      if (p && pen[f - f0] < m.total - 0.5) tip[f - f0] = pointAt(join(L), pen[f - f0]);
    }
    const anchor = (v, t) => {
      if (typeof v !== 'string') return v;
      const [side, name] = v.split(':');
      const L = build(t, curls);
      const [i0, i1] = ends[name];
      return side === 'start' ? L[i0].pts[0] : L[i1 - 1].pts.at(-1);
    };
    const keys = way.map(([t, v, how]) => [t, typeof v === 'function' ? v : anchor(v, t), how]);
    memo = { f0, pen, tip, keys, curls };
    return memo;
  };
}

const frameOf = (m, t) => Math.min(Math.max(near(t) - m.f0, 0), m.pen.length - 1);

export const penOf = (get, t) => get().pen[frameOf(get(), t)];

function along(keys, t) {
  const val = (v) => (typeof v === 'function' ? v(t) : v);
  if (t <= keys[0][0]) return val(keys[0][1]);
  for (let i = 1; i < keys.length; i++) {
    const [tb, vb, how = 'inOut'] = keys[i];
    if (t <= tb) {
      const [ta, va] = keys[i - 1];
      const u = kit.ease[how]((t - ta) / (tb - ta));
      const a = val(va);
      const b = val(vb);
      return [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u];
    }
  }
  return val(keys.at(-1)[1]);
}

export function cursorOf(get, t) {
  const m = get();
  const p = m.tip[frameOf(m, t)] ?? along(m.keys, t);
  return { x: p[0], y: p[1] };
}
