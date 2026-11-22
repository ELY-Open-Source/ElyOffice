// The one line per frame: frozen scenes, the live one.
import { entries, entryAt, toWorld, rotate } from './world.js';
import { join } from './line.js';
import { penAt, drawTime, speedAt, pressure } from './pen.js';
import { bridge, cubic, endTangent } from './geom.js';
import { FPS, ease, span, clamp, smooth } from './time.js';
import { tether } from './tether.js';

export const APPS = ['suite', 'doc', 'table', 'show', 'pdf', 'mail', 'tree', 'mark'];
export const BASE = 3.1;
export const ENTRY = { w: 0.55, alpha: 0.7, time: 0.75, land: 260 };
const TAPER = 40;

const colour = (app, id) => {
  const i = APPS.indexOf(app);
  if (i < 0) throw new Error(`${id}: unknown app ${app}; known: ${APPS.join(' ')}`);
  return i;
};

const inOutInverse = (q) => (q < 0.5 ? Math.cbrt(q / 4) : 1 - Math.cbrt(2 * (1 - q)) / 2);

function blank() {
  return { x: [], y: [], wd: [], a: [], c: [], tau: [] };
}

function add(b, p, wd, a, c, tau) {
  b.x.push(p[0]);
  b.y.push(p[1]);
  b.wd.push(wd);
  b.a.push(a);
  b.c.push(c);
  b.tau.push(tau);
}

function entryLine(b, e, t, from, target) {
  const S = e.s.start;
  // A landing across the approach hooks short, not loops.
  const dx = target.at[0] - from.at[0];
  const dy = target.at[1] - from.at[1];
  const along = Math.max(0, (dx * target.tan[0] + dy * target.tan[1]) / (Math.hypot(dx, dy) || 1));
  const land = ENTRY.land * e.s.place.s * (0.35 + 0.65 * along);
  const pts = bridge(from.at, from.tan, target.at, target.tan, 8, land);
  const cum = [0];
  for (let i = 1; i < pts.length; i++) cum.push(cum[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]));
  const L = cum[cum.length - 1];
  const u = span(t, S, S + ENTRY.time);
  const drawn = ease.inOut(u) * L;
  const c = colour(e.s.app, e.s.id);
  let last = from;
  for (let i = 0; i < pts.length; i++) {
    let p = pts[i];
    if (cum[i] > drawn) {
      if (i === 0) break;
      const k = (drawn - cum[i - 1]) / (cum[i] - cum[i - 1] || 1);
      p = [pts[i - 1][0] + (p[0] - pts[i - 1][0]) * k, pts[i - 1][1] + (p[1] - pts[i - 1][1]) * k];
    }
    const q = L > 0 ? Math.min(cum[i], drawn) / L : 1;
    const v = inOutInverse(q);
    const slope = v < 0.5 ? 12 * v * v : 3 * (2 - 2 * v) ** 2;
    const speed = (L * slope) / ENTRY.time / e.s.place.s;
    add(b, p, BASE * ENTRY.w * pressure(speed) * e.s.place.s, ENTRY.alpha, c, S + ENTRY.time * v);
    last = { at: p, tan: i > 0 ? endTangent([pts[i - 1], p], 0) : from.tan };
    if (cum[i] > drawn) break;
  }
  return { done: u >= 1, tip: last };
}

export function block(e, f, from) {
  const t = f / FPS;
  const { s, k } = e;
  const pl = s.place;
  const J = join(s.strokes(t, k), s.id);
  const b = blank();
  const base = colour(s.app, s.id);
  const W = (p) => toWorld(pl, p[0], p[1]);
  const cursor = s.cursor ? s.cursor(t, k) : null;
  let tip = null;
  if (from) {
    const target = J.first ? { at: W(J.first.at), tan: rotate(pl, J.first.tan) }
      : { at: W(cursor ? [cursor.x, cursor.y] : [960, 540]), tan: [1, 0] };
    const r = entryLine(b, e, t, from, target);
    tip = r.tip;
    if (!r.done) return { b, tip };
  }
  const D = Math.min(penAt(e, f), J.len);
  let prev = null;
  for (let i = 0; D > 0 && i < J.x.length; i++) {
    let p = [J.x[i], J.y[i]];
    let d = J.s[i];
    if (d > D) {
      if (!prev || D <= prev.d) break;
      const u = (D - prev.d) / (d - prev.d);
      p = [prev.p[0] + (p[0] - prev.p[0]) * u, prev.p[1] + (p[1] - prev.p[1]) * u];
      d = D;
    }
    const tau = drawTime(e, d);
    let wd = BASE * J.w[i] * pressure(speedAt(e, tau)) * pl.s;
    if (e.i === 0) wd *= 0.3 + 0.7 * smooth(clamp(d / TAPER));
    const c = J.app[i] ? colour(J.app[i], s.id) : base;
    add(b, W(p), wd, J.a[i], c, tau);
    if (prev) tip = { at: W(p), tan: rotate(pl, endTangent([prev.p, p], 0)) };
    else if (!tip) tip = { at: W(p), tan: rotate(pl, J.first.tan) };
    prev = { p, d };
    if (d >= D) break;
  }
  const complete = J.len === 0 || D >= J.len - 0.5;
  if (cursor && complete && tip) {
    const lag = tether(s, k, f);
    const pts = cubicTether(tip, W([cursor.x, cursor.y]), W(lag));
    for (let i = 1; i < pts.length; i++) add(b, pts[i], BASE * 0.75 * pl.s, 1, base, t - 0.35 * (1 - i / (pts.length - 1)));
    if (pts.length > 1) tip = { at: pts[pts.length - 1], tan: endTangent(pts, 0) };
  }
  return { b, tip };
}

function cubicTether(tip, c, lag) {
  const d = Math.hypot(c[0] - tip.at[0], c[1] - tip.at[1]);
  if (d < 1) return [tip.at];
  const h = clamp(d * 0.35, 6, 320);
  const back = [(tip.at[0] - c[0]) / d, (tip.at[1] - c[1]) / d];
  const p1 = [tip.at[0] + tip.tan[0] * h, tip.at[1] + tip.tan[1] * h];
  const p2 = [c[0] + (lag[0] - c[0]) * 1.4 + back[0] * h * 0.5, c[1] + (lag[1] - c[1]) * 1.4 + back[1] * h * 0.5];
  return cubic(tip.at, p1, p2, c, 6);
}

const frozen = [];
function freeze(e) {
  if (!frozen[e.i]) frozen[e.i] = block(e, e.f1, e.i ? freeze(entries[e.i - 1]).tip : null);
  return frozen[e.i];
}

export function lineAt(n) {
  const t = n / FPS;
  const cur = entryAt(t);
  const blocks = [];
  for (let i = 0; i < cur.i; i++) blocks.push(freeze(entries[i]).b);
  const live = block(cur, Math.min(n, cur.f1), cur.i ? freeze(entries[cur.i - 1]).tip : null);
  blocks.push(live.b);
  return { blocks, tip: live.tip, entry: cur, t };
}
