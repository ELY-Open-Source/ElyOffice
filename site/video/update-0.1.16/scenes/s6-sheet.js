// s6: the sheet, its drawing, the new chart.
import { span, clamp, lerp, ease, keys, spring } from '../lib/time.js';
import { signed } from '../lib/rand.js';

export const GREEN = [46, 158, 91];
const PALE = [150, 196, 160];
const BLUE = [48, 112, 214];
const PAPER = 'rgba(244,234,214,0.97)';

export const col = (i) => 480 + i * 136;
export const row = (r) => 236 + (r - 1) * 56;

const DATA = [
  ['月份', '收入', '支出'], ['1月', 120, 86], ['2月', 148, 92], ['3月', 132, 110],
  ['4月', 176, 128], ['5月', 190, 141], ['6月', 214, 160],
];
export const VALUE = { text: '1284', x: col(3) - 16, y: row(10) + 40, size: 32 };
export const CELL = [col(2), row(10), col(3), row(11)];

export const CHART = { w: 500, h: 260 };
const BIRTH = [684, 470];
const BUMP = [1294, 112];
const BEND = [1800, 330];
export const HOME = [1294, 540];
const CLOUD = { x: 1152, y: 318, rx: 128, ry: 64 };

const draws = (t, t0, d) => ease.out(span(t, t0, t0 + d));

export function grid(ink, t) {
  const lines = [[[420, 194], [1800, 194], 2.2, 0.55]];
  for (let r = 1; r <= 14; r++) lines.push([[420, row(r)], [1720 + 70 * signed(61, r), row(r)], 1.5, r === 1 ? 0.5 : 0.3]);
  for (let i = 0; i <= 10; i++) lines.push([[col(i), 200], [col(i), 960 + 40 * signed(62, i)], 1.4, 0.28]);
  lines.forEach(([a, b, width, alpha], k) => {
    const u = draws(t, 54.0 + 0.022 * k, 0.26);
    if (u > 0) ink.line([a, b], { id: 600 + k, style: 'charcoal', width, alpha, upto: u, taper: [0.02, 0.3] });
  });
  const u = draws(t, 54.0, 0.2);
  if (u > 0) ink.line([[500, 148], [500, 190]], { id: 640, width: 1.6, alpha: 0.4 * u });
  for (let i = 0; i < 10; i++) {
    const k = draws(t, 54.1 + 0.02 * i, 0.12);
    if (k > 0) ink.text('ABCDEFGHIJ'[i], { x: col(i) + 64, y: 226, size: 26, align: 'center', color: ink.CHARCOAL, alpha: 0.6 * k });
  }
  for (let r = 1; r <= 13; r++) {
    const k = draws(t, 54.1 + 0.02 * r, 0.12);
    if (k > 0) ink.text(String(r), { x: 450, y: row(r) + 37, size: 24, align: 'center', color: ink.CHARCOAL, alpha: 0.55 * k });
  }
}

export function data(ink, t) {
  DATA.forEach((cells, r) => cells.forEach((v, c) => {
    const k = draws(t, 54.2 + 0.03 * (r * 3 + c), 0.14);
    if (k <= 0) return;
    const num = typeof v === 'number';
    ink.text(String(v), { x: num ? col(c + 1) - 16 : col(c) + 16, y: row(r + 1) + 40, size: 32,
      align: num ? 'right' : 'left', alpha: (r ? 0.92 : 0.75) * k, upto: k });
  }));
  const k = draws(t, 54.85, 0.14);
  if (k > 0) ink.text(VALUE.text, { ...VALUE, align: 'right', alpha: 0.92 * k, upto: k });
}

function cloudShape(t) {
  const sq = spring(t - 55.5, 0.16, 22, 8) + spring(t - 56.0, 0.05, 26, 10);
  const floor = CLOUD.y + CLOUD.ry * 1.1;
  const pts = [];
  for (let i = 0; i < 96; i++) {
    const a = (i / 96) * Math.PI * 2;
    const lobe = 1 + 0.13 * Math.abs(Math.sin(a * 3.5 + 0.4));
    const x = CLOUD.x + Math.cos(a) * CLOUD.rx * lobe * (1 + 0.6 * sq);
    const y = CLOUD.y + Math.sin(a) * CLOUD.ry * lobe;
    pts.push([x, floor - (floor - y) * (1 - sq)]);
  }
  return pts;
}

export function cloud(ink, t) {
  const u = draws(t, 54.3, 0.3);
  if (u <= 0) return;
  const shape = cloudShape(t);
  ink.lift(4, [CLOUD.x, CLOUD.y], (k) => {
    k.wash([shape], { id: 650, color: BLUE, alpha: 0.3 * u, bleed: 6 });
    k.hatch([shape], { id: 651, gap: 8, angle: 0.7, width: 1.4, color: [52, 74, 110], alpha: 0.4 * u, reach: 0.75 });
    k.line([...shape, shape[0], shape[1]], { width: 3, upto: u, overshoot: 10, pressure: 0.5 });
  });
}

function bezier(a, b, c, u) {
  const v = 1 - u;
  return [0, 1].map((i) => v * v * a[i] + 2 * v * u * b[i] + u * u * c[i]);
}

export function chartAt(t) {
  if (t < 55.0) return null;
  const s = keys(t, [[55.0, 0.32], [55.32, 1, 'out']]);
  const born = clamp((t - 55.0) / 0.09);
  if (t < 55.5) {
    const u = span(t, 55.0, 55.5);
    return { x: lerp(BIRTH[0], BUMP[0], u), y: lerp(BIRTH[1], BUMP[1], u) - 4 * 230 * u * (1 - u),
      s, rot: -0.16 * Math.sin(Math.PI * u), lift: keys(t, [[55.0, 8], [55.25, 120], [55.5, 90]]), sq: 1, born };
  }
  if (t < 56.0) {
    const u = ease.inOut(span(t, 55.5, 56.0));
    const [x, y] = bezier(BUMP, BEND, HOME, u);
    const bump = spring(t - 55.5, 0.08, 24, 10);
    return { x, y: y - 20 * bump, s, rot: 0.11 * Math.sin(Math.PI * u), lift: lerp(90, 10, u * u), sq: 1 - bump, born };
  }
  return { x: HOME[0], y: HOME[1], s: 1, rot: 0, lift: 10, sq: 1 - spring(t - 56.0, 0.1, 20, 8), born: 1 };
}

export function chartTop(c) {
  const up = (CHART.h / 2) * c.s * c.sq;
  return [c.x + Math.sin(c.rot) * up, c.y + (CHART.h / 2) * c.s * (1 - c.sq) - Math.cos(c.rot) * up];
}

const VALUES = DATA.slice(1).map((r) => [r[1], r[2]]);

function body(k, t) {
  const { w, h } = CHART;
  const card = [[-w / 2, -h / 2], [w / 2, -h / 2], [w / 2, h / 2], [-w / 2, h / 2]];
  k.ctx.fillStyle = PAPER;
  k.ctx.beginPath();
  card.forEach(([x, y], i) => (i ? k.ctx.lineTo(x, y) : k.ctx.moveTo(x, y)));
  k.ctx.fill();
  k.shape([card], { width: 2.4, overshoot: 9 });
  const x0 = -w / 2 + 44;
  const base = h / 2 - 30;
  for (let g = 1; g <= 3; g++) k.line([[x0, base - g * 60], [w / 2 - 22, base - g * 60]], { id: 660 + g, style: 'charcoal', width: 1.1, alpha: 0.25 });
  k.line([[x0, -h / 2 + 22], [x0, base]], { id: 665, width: 2.2 });
  k.line([[x0 - 6, base], [w / 2 - 18, base]], { id: 666, width: 2.4 });
  const gw = (w / 2 - 30 - x0) / 6;
  VALUES.forEach((pair, g) => {
    const bloom = span(t, 56.0 + g * 0.125, 56.22 + g * 0.125);
    pair.forEach((v, j) => {
      const bx = x0 + 12 + g * gw + j * 26;
      const top = base - (v / 240) * 190;
      const grow = bloom >= 1 ? 1 : ease.back(bloom);
      const rect = [[bx, base], [bx + 22, base], [bx + 22, base - (base - top) * grow], [bx, base - (base - top) * grow]];
      if (bloom > 0) k.wash([rect], { id: 670 + g * 2 + j, color: j ? PALE : GREEN, alpha: 0.85 * Math.min(1, bloom * 2), bleed: 2.5 });
      const full = [[bx, base], [bx, top], [bx + 22, top], [bx + 22, base]];
      k.line(full, { id: 690 + g * 2 + j, width: bloom > 0 ? 1.8 : 1.2, alpha: bloom > 0 ? 0.85 : 0.4, overshoot: 4 });
    });
  });
}

export function chart(ink, t) {
  const c = chartAt(t);
  if (!c) return;
  ink.lift(c.lift, [c.x, c.y], (k) => {
    const ctx = k.ctx;
    ctx.save();
    ctx.globalAlpha = c.born;
    ctx.translate(c.x, c.y + (CHART.h / 2) * c.s * (1 - c.sq));
    ctx.rotate(c.rot);
    ctx.scale(c.s * (2 - c.sq), c.s * c.sq);
    body(k, t);
    ctx.restore();
  });
}

export function barTops() {
  const { w, h } = CHART;
  const x0 = -w / 2 + 44;
  const gw = (w / 2 - 30 - x0) / 6;
  return VALUES.map((pair, g) => [HOME[0] + x0 + 12 + g * gw + 24, HOME[1] + h / 2 - 30 - (pair[0] / 240) * 190]);
}

export function smear(ink, t) {
  if (t < 55.05 || t > 56.12) return;
  const fade = 1 - span(t, 56.0, 56.12);
  const trail = [];
  for (let k = 9; k >= 0; k--) {
    const c = chartAt(Math.max(55.0, Math.min(t, 56.0) - k * 0.022));
    trail.push([c.x, c.y]);
  }
  const [a, b] = [trail[0], trail[trail.length - 1]];
  const run = Math.hypot(b[0] - a[0], b[1] - a[1]);
  if (run < 40) return;
  const [nx, ny] = [-(b[1] - a[1]) / run, (b[0] - a[0]) / run];
  [-0.8, -0.3, 0.25, 0.75].forEach((o, i) => {
    const off = o * CHART.h * 0.5;
    ink.curve(trail.map(([x, y]) => [x + nx * off, y + ny * off]), { id: 900 + i, style: 'charcoal', width: 3.4,
      alpha: 0.7 * fade, upto: 0.82, taper: [0.6, 0.1], overshoot: 0 });
  });
}

export function impact(ink, t) {
  const hits = [[56.0, HOME[0] - CHART.w / 2, HOME[1] + CHART.h / 2, Math.PI], [56.0, HOME[0] + CHART.w / 2, HOME[1] + CHART.h / 2, 0],
    [55.5, CLOUD.x - 60, CLOUD.y - CLOUD.ry - 6, Math.PI], [55.5, CLOUD.x + 60, CLOUD.y - CLOUD.ry - 6, 0]];
  hits.forEach(([t0, x, y, side], h) => {
    const u = span(t, t0, t0 + 0.25);
    if (u <= 0 || u >= 1) return;
    for (let i = 0; i < 3; i++) {
      const a = side + (side ? 1 : -1) * (0.25 + i * 0.32);
      const r0 = 18 + 50 * ease.out(u);
      const r1 = r0 + 30 * (1 - u);
      ink.line([[x + Math.cos(a) * r0, y + Math.sin(a) * r0], [x + Math.cos(a) * r1, y + Math.sin(a) * r1]],
        { id: 920 + h * 3 + i, width: 2.6, overshoot: 0 });
    }
  });
}
