// s6: ElyTable's chart dodges; the link frays.
import { keys, clamp, spring, span } from '../lib/time.js';
import { noise1 } from '../lib/rand.js';
import { REST } from '../lib/flame.js';
import { hop } from '../lib/act.js';
import { burst } from '../lib/embers.js';
import { grid, data, cloud, chart, smear, impact, chartAt, chartTop, barTops, col, row, VALUE, HOME, CHART } from './s6-sheet.js';
import { book, thread, formula, circle, breakAt, strainAt, SNAP, TEAR } from './s6-link.js';

const START = [REST.x, REST.y];
const PERCH = [col(2) + 68, row(10)];
const GUARD = [col(3) + 46, row(11)];
const SEAT = chartTop(chartAt(57));
const BORN = chartTop(chartAt(55.0));

const POOL = [[54.0, 520], [54.6, 760], [55.0, 860], [59.9, 860], [60.0, 300, 'out'], [60.6, 330], [61.0, 860],
  [62.6, 860], [63.5, REST.pool]];
const ENERGY = [[54.0, 0.5], [54.6, 0.7], [56.0, 0.95], [57.0, 0.75], [59.0, 0.9], [59.95, 1], [60.06, 0.25],
  [60.6, 0.6], [61.0, 0.85], [62.6, 0.7], [63.5, REST.energy]];
const ROAR = [[55.0, 0], [55.06, 0.5, 'out'], [55.4, 0.08], [56.0, 0], [56.06, 0.95, 'out'], [56.9, 0.12],
  [57.3, 0], [58.0, 0], [58.06, 0.35, 'out'], [58.5, 0], [60.75, 0], [60.82, 0.45, 'out'], [61.4, 0.08], [61.8, 0]];

function riding(t) {
  const at = (s) => chartTop(chartAt(clamp(s, 55.0, 57.0)));
  const [x, y] = at(t);
  const vx = (at(t + 1 / 48)[0] - at(t - 1 / 48)[0]) * 24;
  const c = chartAt(t);
  const jolt = spring(t - 55.0, 0.34) + spring(t - 55.5, 0.3, 16, 7) + spring(t - 56.0, 0.42, 15, 7);
  return { x, y, lean: clamp(c.rot - vx / 2600, -0.7, 0.7) + 0.05 * noise1(t * 3, 31), squash: 1 - jolt };
}

function pulled(t) {
  if (t < TEAR[0]) return { x: PERCH[0], y: PERCH[1], lean: keys(t, [[58.45, 0], [58.75, 0.22]]) };
  const brace = span(t, TEAR[1], SNAP - 0.05);
  const startle = keys(t, [[TEAR[0], 0], [TEAR[0] + 0.08, 0.32, 'out'], [TEAR[1], 0.06]]);
  const strain = strainAt(t);
  return { x: PERCH[0] + 10 * brace + 16 * strain, y: PERCH[1], lean: 0.22 - 0.3 * startle * (1 - brace) + 0.25 * brace + 0.25 * strain,
    squash: 1 + startle - 0.22 * brace, curl: 0.4 * span(t, 59.15, 59.3) * (1 - span(t, 59.4, 59.55)) };
}

function guarding(t) {
  return { x: GUARD[0], y: GUARD[1], squash: 0.94 - spring(t - 62.0, 0.2, 14, 7),
    lean: keys(t, [[60.75, -0.3], [61.0, -0.3], [61.3, 0.2], [61.9, 0.2], [62.15, -0.12], [62.45, 0]]),
    curl: keys(t, [[61.2, 0], [61.4, 0.25], [61.8, 0.25], [62.0, 0]]) };
}

function act(t) {
  if (t < 54.5) return { lean: keys(t, [[54.0, 0], [54.2, -0.28], [54.45, -0.12]]) };
  if (t < 55.0) return hop(t, { t0: 54.625, t1: 55.0, from: START, to: BORN, height: 170, pre: 0.125 });
  if (t < 57.5) return riding(t);
  if (t < 58.4) return hop(t, { t0: 57.6, t1: 58.0, from: SEAT, to: PERCH, height: 130, pre: 0.1 });
  if (t < SNAP) return pulled(t);
  if (t < 60.38) return { x: PERCH[0], y: PERCH[1], lean: -0.35 * Math.exp(-6 * (t - SNAP)) * Math.cos(14 * (t - SNAP)) };
  if (t < 61.15) return hop(t, { t0: 60.5, t1: 60.75, from: PERCH, to: GUARD, height: 60, pre: 0.12, post: 0.4 });
  if (t < 62.48) return guarding(t);
  return hop(t, { t0: 62.6, t1: 63.0, from: GUARD, to: START, height: 140, pre: 0.12, post: 0.5 });
}

function flame(t) {
  if (t >= 63.5) return {};
  return {
    ...act(t),
    pool: keys(t, POOL),
    energy: keys(t, ENERGY),
    roar: keys(t, ROAR),
    ember: keys(t, [[SNAP, 0], [SNAP + 0.06, 0.72, 'out'], [60.42, 0.55], [60.62, 0]]),
  };
}

function draw(ctx, t, ink) {
  grid(ink, t);
  data(ink, t);
  formula(ink, t);
  cloud(ink, t);
  smear(ink, t);
  chart(ink, t);
  impact(ink, t);
  book(ink, t);
  thread(ink, t);
  circle(ink, t);
}

const valueGlow = (t) => keys(t, [[58.0, 0], [58.12, 0.3], [59.95, 0.3], [SNAP + 0.04, 1, 'out'], [61.0, 0.8], [64, 0.6]]);

function fx(ctx, t, ink) {
  burst(ctx, t, { t0: 55.0, x: BORN[0], y: BORN[1], count: 34, speed: 420, life: 0.6, spread: 1.4, angle: -1.9, gravity: 420, seed: 61, size: 2 });
  const bottom = HOME[1] + CHART.h / 2;
  for (const [x, a, seed] of [[HOME[0] - 240, -2.7, 62], [HOME[0] + 240, -0.45, 63]]) {
    burst(ctx, t, { t0: 56.0, x, y: bottom, count: 16, speed: 360, life: 0.5, spread: 0.5, angle: a, gravity: 600, seed, size: 1.8 });
  }
  barTops().forEach(([x, y], g) => burst(ctx, t, { t0: 56.1 + g * 0.125, x, y, count: 7, speed: 170, life: 0.45, gravity: -60, seed: 70 + g, size: 1.5 }));
  const [bx, by] = breakAt(SNAP);
  burst(ctx, t, { t0: SNAP, x: bx, y: by, count: 22, speed: 300, life: 0.55, gravity: 300, seed: 80, size: 1.4 });
  const g = valueGlow(t);
  if (g <= 0) return;
  const list = ink.glyphs(VALUE.text, { ...VALUE, align: 'right' });
  const flick = g * (0.88 + 0.12 * noise1(t * 9, 87));
  ctx.save();
  for (const [blur, color, a, op] of [[12, '255,110,36', 0.5, 'lighter'], [0, '214,84,22', 0.95, 'source-over'],
    [1.2, '255,200,110', 0.55, 'lighter']]) {
    ctx.globalCompositeOperation = op;
    ctx.filter = blur ? `blur(${blur}px)` : 'none';
    ctx.fillStyle = `rgba(${color},${a * flick})`;
    ctx.beginPath();
    for (const gl of list) for (const c of gl.contours) {
      c.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
      ctx.closePath();
    }
    ctx.fill('nonzero');
  }
  ctx.restore();
}

function lights(t) {
  const out = [];
  if (t >= 56.0) out.push({ x: HOME[0], y: HOME[1], r: 520, i: 0.6 * Math.exp(-(t - 56.0) * 5), c: [1.05, 1, 0.9] });
  const g = valueGlow(t);
  if (g > 0) out.push({ x: VALUE.x - 34, y: VALUE.y - 12, r: 260, i: 0.4 * g, c: [1.15, 0.82, 0.55] });
  if (t >= SNAP) {
    const [bx, by] = breakAt(SNAP);
    out.push({ x: bx, y: by, r: 220, i: 0.5 * Math.exp(-(t - SNAP) * 9), c: [1.1, 0.9, 0.7] });
  }
  return out;
}

export default { id: 's6', start: 54, end: 64, flame, draw, fx, lights };
