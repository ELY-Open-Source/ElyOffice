// Finale: the pull-back, the line writes the name, the lift.
import { kit } from '../lib/kit/index.js';
import { cubic } from '../lib/geom.js';
import { FPS, LIFT } from '../lib/time.js';

export const BAND = 4;
const START = 106;
const END = 120;
const COLOURS = ['doc', 'table', 'show', 'pdf', 'mail', 'tree', 'mark'];
const TYPE = { V: 0.6, GAP: 0.36, RULE: 95, CAP: 190, UNDER: 0.42, w: 2.2 };
const CORNER = 40;
// The map's drawn ink, inset from its panel edges.
const INK = { left: 70, right: 100, top: 32 };
const LINK = 0.15;

const ink = (list) => list.flatMap((s) => s.pts);
const extent = (list, i) => [Math.min(...ink(list).map((p) => p[i])), Math.max(...ink(list).map((p) => p[i]))];

function shift(list, dx, dy) {
  return list.map((s) => ({ ...s, pts: s.pts.map(([x, y]) => [x + dx, y + dy]) }));
}

function paint(list, from) {
  const glyphs = [...new Set(list.map((s) => s.g))];
  return list.map((s, i) => ({ ...s, w: TYPE.w, link: i ? LINK : 1, app: COLOURS[(from + glyphs.indexOf(s.g)) % COLOURS.length] }));
}

// A full stop as a tight spiral, reading solid.
function dot(s) {
  if (s.ch !== '.') return s;
  const [x0, x1] = extent([s], 0);
  const [y0, y1] = extent([s], 1);
  const c = [(x0 + x1) / 2, (y0 + y1) / 2];
  const r = Math.max(x1 - x0, y1 - y0) / 2 + 1.5;
  const pts = [];
  for (let i = 0; i <= 96; i++) {
    const a = -Math.PI / 2 - (i / 96) * 6 * Math.PI;
    const d = r * (1 - (0.85 * i) / 96);
    pts.push([c[0] + d * Math.cos(a), c[1] + d * Math.sin(a)]);
  }
  return { ...s, pts };
}

// The E opens at its corner, where the rule lands.
function openE(list) {
  const [outline, ...rest] = list.filter((s) => s.ch === 'E');
  const c = outline.pts.reduce((b, p, i, a) => (p[0] + p[1] < a[b][0] + a[b][1] ? i : b), 0);
  const top = { ...outline, pts: outline.pts.slice(0, c + 1).reverse() };
  const stem = { ...outline, pts: outline.pts.slice(c), link: 0 };
  return [top, stem, ...rest, ...list.filter((s) => s.ch !== 'E')];
}

function rule(from, corner) {
  const [x0, y0] = from;
  const y = TYPE.RULE;
  const bend = cubic([x0, y0], [x0 - 34, y0 + 24], [x0 - 70, y], [x0 - 130, y], 4);
  const turn = kit.arc(corner[0] + CORNER, y + CORNER, CORNER, -Math.PI / 2, -Math.PI).pts;
  return kit.stroke([...bend, ...turn, corner], { w: 0.5 });
}

function tail(foot, left, y) {
  const [fx, fy] = foot;
  const drop = cubic([fx, fy], [fx - 10, fy + 90], [fx - 30, y], [fx - 150, y], 4);
  return kit.stroke([...drop, [left, y]], { w: 0.95 });
}

let built = null;
function build(k) {
  if (built) return built;
  const L = k.sheet.x + INK.left;
  const R = k.sheet.x + k.sheet.w - INK.right;
  const inkWidth = (str, size) => {
    const [a, b] = extent(kit.text(str, { size }), 0);
    return b - a;
  };
  const F = (R - L) / (inkWidth('Ely Office', 1) + TYPE.GAP + TYPE.V * inkWidth('0.1.17', 1));
  const raw = kit.text('Ely Office', { size: F });
  const base = TYPE.CAP - extent(raw.filter((s) => s.ch === 'E'), 1)[0];
  const name = openE(paint(shift(raw, L - extent(raw, 0)[0], base), 0));
  const rawV = kit.text('0.1.17', { size: F * TYPE.V });
  const version = paint(shift(rawV, R - extent(rawV, 0)[1], base), new Set(raw.map((s) => s.g)).size).map(dot);
  const corner = name[0].pts[0];
  const exit = [k.prev[0] + 77, k.prev[1] + 207];
  const strokes = [rule(exit, corner), ...name, ...version, tail(version.at(-1).pts.at(-1), L, base + TYPE.UNDER * F)];
  const m = kit.marks(strokes).list;
  const yEnd = m[strokes.findLastIndex((s) => s.ch === 'y')].end;
  const oStart = m[strokes.findIndex((s) => s.ch === 'O')].start;
  const nameEnd = m[name.length].end;
  const vStart = m[name.length + 1].start;
  const vEnd = m[strokes.length - 2].end;
  const total = m.at(-1).end;
  const settle = (u) => 1 - (1 - u) ** 2.2;
  const keys = [[START + 0.75, 0], [110.875, m[0].end], [111.0, m[0].end], [111.75, yEnd, 'linear'], [112.0, oStart, 'sine'],
    [113.25, nameEnd, 'linear'], [113.5, vStart, 'sine'], [114.0, vEnd, 'linear']];
  const pens = new Float64Array(Math.round((END - START) * FPS) + 1);
  for (let i = 0; i < pens.length; i++) {
    const t = START + i / FPS;
    pens[i] = t <= 114 ? kit.keys(t, keys) : vEnd + (total - vEnd) * settle(kit.span(t, 114, LIFT));
  }
  built = { strokes, pens, under: base + TYPE.UNDER * F };
  return built;
}

function view(k) {
  const top = k.sheet.y + INK.top;
  const bottom = build(k).under;
  const h = (bottom - top) * 1.15;
  return { x: k.sheet.x + k.sheet.w / 2, y: (top + bottom) / 2, w: (h * 16) / 9 };
}

export default {
  id: 'c12', start: START, end: END, app: 'suite',
  strokes: (t, k) => build(k).strokes,
  pen: (t, k) => build(k).pens[Math.min(Math.max(Math.round((t - START) * FPS), 0), build(k).pens.length - 1)],
  camera(t, k) {
    const a = { x: k.prev[0] + 730 / BAND, y: k.prev[1] + 260 / BAND, w: 1120 / BAND };
    const b = view(k);
    const wide = b.w * 1.06;
    const u = kit.ease.inOut(kit.span(t, START, 112));
    const w = Math.exp(kit.lerp(Math.log(a.w), Math.log(wide), u));
    const p = (w - a.w) / (wide - a.w);
    const push = kit.ease.out(kit.span(t, 112, LIFT));
    return {
      x: kit.lerp(a.x, b.x, p),
      y: kit.lerp(a.y, b.y, p),
      w: kit.lerp(w, b.w, push),
      r: -0.035 * Math.sin(Math.PI * u),
    };
  },
};
