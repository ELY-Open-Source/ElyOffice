// ElyTree: commits, their lanes, a drag, a copied strip.
import { keys, span, clamp, ease, smooth, spring } from '../lib/time.js';
import { glyphs, advance } from '../lib/glyph.js';
import { noise1 } from '../lib/rand.js';

export const PURPLE = [126, 87, 194];
const AMBER = [242, 169, 0];
const BLUE = [61, 127, 229];
const GREEN = [46, 158, 91];
const SIZE = 28;
const ROW0 = 352;
const STEP = 56;
export const TX0 = 652;
export const TX1 = 474;
const HX = 1352;
export const EAT = [68.05, 69.8];
export const STRETCH = 70.0;
export const DRAG = [71.0, 72.0];
export const LIFT = 72.05;
export const SEL = [2, 4];
export const PAGE = { x0: 420, x1: 1500, y0: 236, y1: 830 };

const TITLES = [
  ["Merge branch 'photos'", '9c4e21a'],
  ['Add Kyoto chapter', '61d0b7f'],
  ['Fix captions on map', 'e5a38c2'],
  ['Trim intro', '3b97f04'],
  ['Rename drafts folder', 'a7f15d9'],
  ['Add packing list', '58c2e6b'],
  ['Fix typo in day 3', 'd04a9e3'],
  ['Initial commit', '7f6b3c1'],
];

export const base = (i) => ROW0 + i * STEP;
const mid = (i) => base(i) - 9;
const lx = (k) => 482 + k * 38;
export const ROWS = TITLES.length;
export const rowEnd = () => HX + advance('0000000', 24) + 10;

function lane(keys) {
  const pts = [];
  for (let j = 1; j < keys.length; j++) {
    const [ka, ra] = keys[j - 1];
    const [kb, rb] = keys[j];
    for (let y = mid(ra) - (j > 1 ? 4 : 0); y > mid(rb) - 0.1; y -= 4) {
      const u = smooth((mid(ra) - y) / (mid(ra) - mid(rb)));
      pts.push([lx(ka) + (lx(kb) - lx(ka)) * u, y]);
    }
  }
  return pts;
}

const LANES = [
  { color: PURPLE, keys: [[0, 7], [0, 0]], dots: [0, 3, 6, 7], k: 0 },
  { color: AMBER, keys: [[0, 7], [1, 6], [1, 1], [0, 0]], dots: [1, 4], k: 1 },
  { color: BLUE, keys: [[1, 4], [2, 3], [2, 2], [1, 1]], dots: [2], k: 2 },
  { color: GREEN, keys: [[0, 6], [2, 5], [1, 4]], dots: [5], k: 2 },
];

let lanes = null;
function plan() {
  lanes ??= LANES.map((l) => ({ ...l, fine: lane(l.keys) }));
  return lanes;
}

export const MOUTH = [lx(1), mid(3) + 22];
export const reach = (t) => keys(t, [[EAT[0], 0], [EAT[1], 330, 'in']]);
const dist = ([x, y]) => Math.hypot(x - MOUTH[0], y - MOUTH[1]);

function warp([x, y], r, pull) {
  const d = Math.hypot(x - MOUTH[0], y - MOUTH[1]) || 1;
  const s = pull * Math.exp(-(d - r) / 90);
  const a = Math.atan2(y - MOUTH[1], x - MOUTH[0]) + 1.1 * s;
  const k = d * (1 - 0.7 * s);
  return [MOUTH[0] + Math.cos(a) * k, MOUTH[1] + Math.sin(a) * k];
}

function runs(pts, r, pull) {
  const out = [];
  let cur = [];
  for (const p of pts) {
    if (dist(p) > r) cur.push(warp(p, r, pull));
    else if (cur.length) out.push(cur), (cur = []);
  }
  if (cur.length) out.push(cur);
  return out.filter((c) => c.length > 1);
}

const suck = (t) => (t < EAT[0] ? [0, 0] : [reach(t), smooth(span(t, EAT[0], EAT[0] + 0.35))]);

export function cutEnds(t) {
  const [r, pull] = suck(t);
  if (r <= 0 || t > EAT[1]) return [];
  const ends = [];
  for (const l of plan()) {
    l.fine.forEach((p, i) => {
      const q = l.fine[i + 1];
      if (q && (dist(p) - r) * (dist(q) - r) <= 0) ends.push({ p: warp(dist(p) > r ? p : q, r, pull), color: l.color });
    });
  }
  return ends;
}

function dot(ink, x, y, color, hollow) {
  const ring = [];
  for (let i = 0; i < 14; i++) ring.push([x + Math.cos((i / 14) * 6.283) * 9, y + Math.sin((i / 14) * 6.283) * 9]);
  if (!hollow) ink.wash([ring], { color, alpha: 0.9, bleed: 1.5 });
  ink.line([...ring, ring[0], ring[1]], { width: 2.4, overshoot: 4 });
}

export function graph(ink, t) {
  const [r, pull] = suck(t);
  if (t >= EAT[1]) return;
  plan().forEach((l, n) => {
    for (const run of runs(l.fine, r, pull)) {
      ink.line(run, { color: l.color, width: 7, alpha: 0.88, boil: 1.3, overshoot: 0, taper: [0.02, r > 0 ? 0.12 : 0.02] });
    }
    for (const d of l.dots) {
      const p = [lx(l.k), mid(d)];
      if (dist(p) > r) dot(ink, ...warp(p, r, pull), l.color, n === 0 && d === 0);
    }
  });
}

function shift(t, i, g) {
  const a = STRETCH + 0.03 * i + 0.006 * g;
  const u = span(t, a, a + 0.36);
  return (TX1 - TX0) * ease.back(u);
}

export function titleList(i, t) {
  const list = glyphs(TITLES[i][0], { x: TX0, y: base(i), size: SIZE });
  if (t < STRETCH) return list;
  return list.map((g, k) => {
    const dx = shift(t, i, k);
    return { ...g, contours: g.contours.map((c) => c.map(([x, y]) => [x + dx, y])), box: g.box };
  });
}

export function rows(ink, t) {
  for (let i = 0; i < ROWS; i++) {
    ink.write(titleList(i, t), { size: SIZE, fill: 'solid', alpha: 0.9 });
    ink.text(TITLES[i][1], { x: HX, y: base(i), size: 24, style: 'charcoal', alpha: 0.6, color: [70, 58, 50] });
  }
}

export function page(ink) {
  const { x0, x1, y0, y1 } = PAGE;
  ink.wash([[[x0, y0], [x1, y0], [x1, y0 + 54], [x0, y0 + 54]]], { color: PURPLE, alpha: 0.3, bleed: 4 });
  ink.text('main', { x: (x0 + x1) / 2, y: y0 + 37, size: 26, align: 'center', color: [50, 30, 80] });
  ink.line([[x0, y0], [x1, y0]], { width: 2.6 });
  ink.line([[x1, y0], [x1, y1]], { width: 2.4 });
  ink.line([[x1, y1], [x0, y1]], { width: 2.8 });
  ink.line([[x0, y1], [x0, y0]], { width: 2.2 });
  ink.smudge(x1 + 8, (y0 + y1) / 2, 70, { alpha: 0.1, blots: 4 });
}

export function smoke(ink, t) {
  const age = t - EAT[1] + 0.1;
  if (age < 0 || age > 1.4) return;
  const fade = clamp(1 - age / 1.4);
  [0, 1, 2].forEach((k) => {
    const x = lx(k) + 6;
    const rise = 30 + age * 210;
    const pts = [];
    for (let i = 0; i <= 8; i++) {
      const u = i / 8;
      pts.push([x + 16 * Math.sin(u * 4.6 + age * 3 + k * 2.1) * u + 8 * noise1(age * 2 + k, 17), MOUTH[1] - 20 - u * rise]);
    }
    ink.curve(pts, { style: 'charcoal', width: 4 - k * 0.6, alpha: 0.45 * fade, from: clamp(age * 0.4), taper: [0.3, 0.5] });
  });
}

export const dragPath = (t) => keys(t, [[DRAG[0], [TX1 - 14, base(SEL[0]) + 6]], [DRAG[0] + 0.38, [TX1 + 60, base(SEL[1]) + 6], 'inOut'],
  [DRAG[1], [rowEnd(), base(SEL[1]) + 6], 'inOut']]);

function band(i, xa, xb) {
  const y0 = base(i) - 30;
  const y1 = base(i) + 12;
  return [[[xa, y0], [xb, y0 + 1], [xb + 2, y1], [xa - 1, y1 - 1]]];
}

export function selection(ink, t) {
  if (t < DRAG[0]) return;
  const [fx, fy] = dragPath(Math.min(t, DRAG[1]));
  const cur = clamp(Math.round((fy - 6 - ROW0) / STEP), SEL[0], SEL[1]);
  const start = TX1 - 10;
  for (let i = SEL[0]; i <= cur; i++) {
    const end = i < cur ? rowEnd() : Math.max(start + 2, fx);
    ink.wash(band(i, start, end), { color: PURPLE, alpha: 0.34, bleed: 3 });
  }
}

export function strip(t) {
  const u = smooth(span(t, LIFT, LIFT + 0.45));
  const drift = span(t, LIFT + 0.45, 73.3);
  const fly = ease.in(span(t, 73.3, 73.95));
  return {
    on: t >= LIFT,
    height: 70 * u + 30 * drift,
    dx: 30 * u + 26 * drift + 160 * fly,
    dy: -150 * u - 30 * drift + 22 * Math.sin(drift * 3) - 900 * fly,
    rot: -0.035 * u - 0.015 * drift + 0.08 * fly,
    wobble: spring(t - LIFT - 0.45, 0.02, 9, 4),
  };
}

const PAPER = [244, 233, 211];

export function stripPaint(k, t) {
  const s = strip(t);
  const ctx = k.ctx;
  const x0 = TX1 - 24;
  const x1 = rowEnd() + 16;
  const y0 = base(SEL[0]) - 46;
  const y1 = base(SEL[1]) + 26;
  const cx = (x0 + x1) / 2;
  const cy = (y0 + y1) / 2;
  ctx.save();
  ctx.translate(cx + s.dx, cy + s.dy);
  ctx.rotate(s.rot + s.wobble);
  ctx.translate(-cx, -cy);
  const edge = (y, seed) => Array.from({ length: 41 }, (_, i) => [x0 + ((x1 - x0) * i) / 40, y + 4 * noise1(i * 0.9, seed)]);
  const top = edge(y0, 3);
  const bottom = edge(y1, 5).reverse();
  const outline = [...top, ...bottom];
  ctx.fillStyle = `rgb(${PAPER})`;
  ctx.beginPath();
  outline.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
  ctx.closePath();
  ctx.fill();
  k.wash([outline], { color: PURPLE, alpha: 0.14, bleed: 3 });
  k.line(top, { width: 2.2, overshoot: 6 });
  k.line(bottom, { width: 2.4, overshoot: 6 });
  k.line([top[top.length - 1], bottom[0]], { width: 2 });
  k.line([bottom[bottom.length - 1], top[0]], { width: 2 });
  for (let i = SEL[0]; i <= SEL[1]; i++) {
    k.write(titleList(i, 99), { size: SIZE, fill: 'solid', alpha: 0.9 });
    k.text(TITLES[i][1], { x: HX, y: base(i), size: 24, alpha: 0.6, color: [70, 58, 50] });
  }
  ctx.restore();
}
