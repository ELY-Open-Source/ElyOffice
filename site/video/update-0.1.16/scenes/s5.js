// s5: ElyMark; dollar hooks burn, the flame writes Euler.
import { keys, span, clamp, spring, smooth } from '../lib/time.js';
import { noise1 } from '../lib/rand.js';
import { REST, drawFlame } from '../lib/flame.js';
import { hop, first } from '../lib/act.js';
import { burst } from '../lib/embers.js';
import { lay, route, along, drawn, finishes, moved } from './s5-math.js';
import { page, frame, codeLine, source, altLine, picture, COL } from './s5-page.js';
import { drawHooks, fireHooks, hookLights, scorchAt, pull, hookAt } from './s5-hook.js';

const BURNT = [62, 30, 18];
const WRITE = [47.0, 48.0];
const WATCH = [1046, 640];
const FUSE = [46.5, 46.95];
const SPREAD = 900;

let made = null;
function plan() {
  if (made) return made;
  const p = page();
  const bar = (g) => () => g.strokes[1][g.strokes[1].length - 1];
  const under = (g) => [(g.box.x0 + g.box.x1) / 2, g.box.y1 + 2];
  const half = (g) => (g.box.x1 - g.box.x0) / 2;
  const tall = (g) => g.box.y1 - g.box.y0;
  const f1 = lay(['a', ['2', 1], '+', 'b', ['2', 1], '=', 'c', ['2', 1]], { x: 960, y: 424, size: 92, align: 'center' });
  const f2 = lay(['e', ['i', 1], ['p', 1], '+', '1', '=', '0'], { x: 960, y: 846, size: 118, align: 'center' });
  const ab = (g) => ({ ...g, box: g.at });
  const code = [
    { seed: 51, root: bar(p.dollars[0]), target: under(p.f1[0]), hw: half(p.f1[0]), gh: tall(p.f1[0]), side: 1,
      via: [[1012, 318], [930, 352], [800, 338], [672, 316], [566, 338]], grow: [44.0, 45.0] },
    { seed: 57, root: bar(p.dollars[1]), target: under(p.f1[1]), hw: half(p.f1[1]), gh: tall(p.f1[1]), side: 1,
      via: [[1046, 326], [956, 368], [818, 358], [692, 334], [594, 352]], grow: [44.12, 45.0] },
  ].map((h) => ({ ...h, snag: 45.0, land: 46.0, burn: 0.5, u0: 0.43, yank: [34, -52] }));
  const e = ab(f2[0]);
  const i = ab(f2[1]);
  const alt = [
    { seed: 61, root: bar(p.alt[1][0]), target: under(e), hw: half(e), gh: tall(e), side: -1,
      via: [[690, 556], [850, 574], [906, 652], [884, 738], [836, 776]], grow: [50.0, 50.75] },
    { seed: 67, root: bar(p.alt[1][1]), target: under(i), hw: half(i) + 3, gh: tall(i), side: -1,
      via: [[716, 552], [884, 562], [944, 640], [912, 694], [858, 716]], grow: [50.06, 50.75] },
  ].map((h) => ({ ...h, snag: 50.75, land: 51.5, burn: 0.5, u0: 0.5, yank: [-30, -38] }));
  const mid = (hs, t) => {
    const [a, b] = hs.map((h) => hookAt(h, t, h.u0));
    return [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2 + 4];
  };
  const legs = route(f2);
  made = { p, f1, f2, code, alt, legs, ends: finishes(legs), L1: mid(code, 46.0), L2: mid(alt, 51.5) };
  made.W0 = [along(legs, 0).x, along(legs, 0).y + 6];
  made.W1 = [along(legs, 1).x, along(legs, 1).y + 6];
  return made;
}

const writeU = (t) => {
  const s = span(t, WRITE[0], WRITE[1]);
  return 0.72 * s + 0.28 * s * s;
};
const timeOfU = (u) => WRITE[0] + (WRITE[1] - WRITE[0]) * ((-0.72 + Math.sqrt(0.5184 + 1.12 * u)) / 0.56);

function writing(t) {
  const { legs, f2 } = plan();
  const u = writeU(t);
  const a = along(legs, writeU(t - 1 / 48));
  const b = along(legs, writeU(t + 1 / 48));
  const vx = (b.x - a.x) * 24;
  const vy = (b.y - a.y) * 24;
  const here = along(legs, u);
  let sup = 0;
  for (const d of [-0.03, 0, 0.03]) {
    const l = along(legs, u + d).leg;
    const g = l.ink ? l.gi : legs[legs.indexOf(l) + 1]?.gi;
    sup += f2[g ?? 0].sup ? 1 / 3 : 0;
  }
  return { x: here.x, y: here.y + 6, h: 104 - 40 * sup, energy: 1, roar: 0.12,
    lean: clamp(vx / 1500, -0.7, 0.7), squash: clamp(1 + Math.abs(vy) / 5000, 1, 1.25) };
}

function place(t) {
  const { L1, L2, W0, W1 } = plan();
  const rest = [REST.x, REST.y];
  if (t >= WRITE[0] && t < WRITE[1]) return writing(t);
  return first(
    t < 45.45 && { lean: keys(t, [[42.4, 0], [42.7, 0.12], [43.1, -0.1], [43.5, 0], [44.2, 0], [44.4, -0.12]]),
      squash: 1 + keys(t, [[44.9, 0], [45.0, 0.22, 'snap'], [45.2, 0]]) },
    hop(t, { t0: 45.6, t1: 46.0, from: rest, to: L1, height: 90, pre: 0.15, post: 0.4 }),
    t < 46.62 && { x: L1[0], y: L1[1] },
    hop(t, { t0: 46.75, t1: WRITE[0], from: L1, to: W0, height: 40, pre: 0.13, post: 0 }),
    t < 48.2 && { x: W1[0], y: W1[1], squash: 1 - spring(t - WRITE[1], 0.32) },
    hop(t, { t0: 48.35, t1: 48.7, from: W1, to: WATCH, height: 70, pre: 0.15, post: 0.4 }),
    t < 51.08 && { x: WATCH[0], y: WATCH[1], lean: keys(t, [[49.65, 0], [49.9, -0.1], [50.15, -0.3, 'out'], [50.6, -0.18]]),
      squash: keys(t, [[50.3, 1], [50.55, 1.22, 'back'], [50.75, 1.1], [50.85, 1.28, 'snap'], [51.08, 1]]) },
    hop(t, { t0: 51.25, t1: 51.5, from: WATCH, to: L2, height: 60, post: 0.4 }),
    t < 52.4 && { x: L2[0], y: L2[1] },
    hop(t, { t0: 52.55, t1: 52.9, from: L2, to: rest, height: 80, pre: 0.15, post: 0.4 }),
  ) ?? {};
}

function flame(t) {
  if (t >= 53.3) return {};
  const pos = place(t);
  return {
    ...pos,
    curl: keys(t, [[44.5, 0], [44.62, 0.7, 'back'], [44.85, 0.7], [44.95, 0, 'snap']]),
    ember: keys(t, [[45.08, 0], [45.2, 0.72, 'out'], [45.4, 0.72], [45.48, 0, 'snap']]),
    roar: Math.max(pos.roar ?? 0, keys(t, [[45.4, 0], [45.5, 0.4], [45.6, 0.1], [46.0, 0.1], [46.04, 0.6, 'out'], [46.5, 0.35],
      [46.6, 0], [48.0, 0], [48.02, 0.95, 'out'], [48.6, 0.15], [48.8, 0], [50.5, 0], [50.6, 0.35], [51.0, 0.2], [51.5, 0.2],
      [51.54, 0.55, 'out'], [52.0, 0.4], [52.03, 0.9, 'out'], [52.5, 0.1], [52.7, 0]])),
    energy: pos.energy ?? keys(t, [[44.9, REST.energy], [45.0, 0.8], [45.6, 0.7], [46.0, 1], [48.8, 1], [49.3, 0.6],
      [50.0, 0.6], [50.4, 0.9], [52.2, 1], [53.3, REST.energy]]),
    pool: keys(t, [[42.0, REST.pool], [43.0, 660], [51.5, 660], [52.0, 720], [53.3, REST.pool]]),
  };
}

function formula1(ink, t) {
  const { f1, p, code } = plan();
  const off = (i) => {
    const pa = pull(code[0], t);
    const pb = pull(code[1], t);
    const k = i === 0 ? pa : i === 1 ? pb : 0.8 * (pa + pb) / 2 * 0.86 ** (i - 1);
    return [code[0].yank[0] * k, code[0].yank[1] * k];
  };
  const fuse = (g) => FUSE[0] + (FUSE[1] - FUSE[0]) * clamp((g.box.x0 - COL) / 380);
  source(ink, p.f1, { t, t0: 43.0, offset: off, burnAt: fuse, life: 0.25 });
  const prog = span(t, 46.95, 47.4);
  if (prog <= 0) return;
  const strokes = f1.flatMap((g) => g.strokes);
  strokes.forEach((st, j) => {
    const u = clamp(prog * strokes.length - j);
    if (u <= 0) return;
    ink.line(st, { id: `f1-${j}`, width: 5.6, upto: u, head: u < 1, taper: [0.06, 0.2], overshoot: 5, pressure: 0.45 });
    ink.line(st, { id: `f1d-${j}`, style: 'charcoal', width: 8, alpha: 0.35, upto: u, taper: [0.1, 0.3], overshoot: 6 });
  });
}

function altPull(t) {
  const { alt } = plan();
  const pa = pull(alt[0], t);
  const pb = pull(alt[1], t);
  return (gi) => {
    const k = gi === 0 ? pa : gi === 1 ? pb : 0.75 * pb * 0.78 ** (gi - 1);
    return [alt[0].yank[0] * k, alt[0].yank[1] * k];
  };
}

function formula2(ink, t) {
  const { f2, p, legs } = plan();
  const front = (g) => WRITE[0] + Math.abs((g.box.x0 + g.box.x1) / 2 - plan().W0[0]) / SPREAD;
  source(ink, p.f2, { t, t0: 43.5, burnAt: front });
  const u = writeU(t);
  if (u <= 0) return;
  const done = drawn(legs, u);
  const off = altPull(t);
  f2.forEach((g, gi) => moved(g, off(gi)).strokes.forEach((st, si) => {
    const f = done.get(`${gi}.${si}`);
    if (!f) return;
    const w = g.sup ? 0.7 : 1;
    ink.line(st, { id: `f2c-${gi}-${si}`, width: 9 * w, color: BURNT, alpha: 0.85, upto: f, head: f < 1, taper: [0.04, 0.08], overshoot: 4, boil: 1.3 });
    ink.line(st, { id: `f2s-${gi}-${si}`, style: 'charcoal', width: 15 * w, color: [30, 20, 16], alpha: 0.85, upto: f, taper: [0.05, 0.1], overshoot: 7, boil: 1.7 });
  }));
}

const wriggle = (t, t0, t1, t2) => keys(t, [[t0, 0], [t0 + 0.15, 3.2], [t0 + 0.35, 1.4], [t1, 1], [t2, 0, 'out']]);

function draw(ctx, t, ink) {
  const { code, alt } = plan();
  frame(ink, t, 42.0);
  picture(ink, t, 43.3);
  codeLine(ink, t, 42.5, wriggle(t, 43.85, 45.0, 46.5));
  for (const h of code) scorchAt(ink, h, t);
  formula1(ink, t);
  altLine(ink, t, 43.25, wriggle(t, 49.9, 50.75, 52.0));
  for (const h of alt) scorchAt(ink, h, t);
  formula2(ink, t);
  drawHooks(ink, code, t);
  drawHooks(ink, alt, t);
}

function hot(ctx, t, ink) {
  const { f2, legs } = plan();
  const u = writeU(t);
  if (u <= 0) return;
  const cool = clamp(1 - (t - WRITE[1]) / 0.8);
  if (cool <= 0) return;
  const flare = t >= WRITE[1] ? 1.4 * cool : 0;
  const done = drawn(legs, u);
  const at = along(legs, u).leg;
  ctx.globalCompositeOperation = 'lighter';
  f2.forEach((g, gi) => g.strokes.forEach((st, si) => {
    const f = done.get(`${gi}.${si}`);
    if (!f) return;
    const live = at.gi === gi && at.si === si;
    const a = Math.max(flare * 0.6, live ? 0.8 : 0.22 * cool);
    if (a < 0.02) return;
    const from = live ? Math.max(0, f - 0.5) : 0;
    ink.line(st, { id: `f2h-${gi}-${si}`, width: g.sup ? 9 : 12, color: [255, 120, 36], alpha: a, from, upto: f, taper: [0.4, 0.05], overshoot: 0, boil: 1.2 });
    ink.line(st, { id: `f2w-${gi}-${si}`, width: 4, color: [255, 226, 160], alpha: a, from: live ? Math.max(0, f - 0.2) : 0, upto: f, taper: [0.5, 0.05], overshoot: 0, boil: 1 });
  }));
  ctx.globalCompositeOperation = 'source-over';
}

function fuse(ctx, t, n) {
  const runs = [[FUSE, COL - 4, 384, 416, -0.35]];
  const [w0] = plan().W0;
  for (const dir of [-1, 1]) {
    const reach = dir < 0 ? w0 - COL + 8 : COL + 360 - w0;
    runs.push([[WRITE[0], WRITE[0] + reach / SPREAD], w0, dir * reach, 840, -0.35 * dir]);
  }
  for (const [[a, b], x0, len, y, lean] of runs) {
    if (t < a || t > b + 0.04) continue;
    drawFlame(ctx, { x: x0 + len * span(t, a, b), y, h: 32, energy: 1, lean, glow: 0.8, pool: 0 }, t + 7, n + 7);
  }
}

function fx(ctx, t, ink) {
  const { code, alt, ends, W1, L1, L2 } = plan();
  fireHooks(ctx, code, t, ink.drawing);
  fireHooks(ctx, alt, t, ink.drawing);
  fuse(ctx, t, ink.drawing);
  hot(ctx, t, ink);
  ends.forEach((u, k) => {
    const t0 = timeOfU(u);
    const at = along(plan().legs, u);
    burst(ctx, t, { t0, x: at.x, y: at.y - 10, count: 12, speed: 220, life: 0.5, gravity: 120, seed: 500 + k, size: 1.7 });
  });
  burst(ctx, t, { t0: WRITE[1], x: W1[0], y: W1[1] - 40, count: 60, speed: 560, life: 1, spread: Math.PI, gravity: 220, seed: 520, size: 2.3 });
  burst(ctx, t, { t0: 46.0, x: L1[0], y: L1[1], count: 34, speed: 380, life: 0.7, spread: 1.4, gravity: 300, seed: 530, size: 2 });
  burst(ctx, t, { t0: 51.5, x: L2[0], y: L2[1], count: 34, speed: 380, life: 0.7, spread: 1.4, gravity: 300, seed: 540, size: 2 });
  burst(ctx, t, { t0: 52.0, x: 790, y: 800, count: 40, speed: 420, life: 0.8, spread: Math.PI, gravity: 200, seed: 550, size: 2 });
  for (const [k, h] of code.entries()) {
    const [x, y] = h.target;
    burst(ctx, t, { t0: 46.5, x, y, count: 14, speed: 260, life: 0.5, gravity: 200, seed: 560 + k, size: 1.8 });
  }
}

function lights(t) {
  const out = [...hookLights(plan().code, t), ...hookLights(plan().alt, t)];
  if (t >= FUSE[0] && t <= FUSE[1]) out.push({ x: COL + 384 * span(t, FUSE[0], FUSE[1]), y: 400, r: 220, i: 0.45 });
  const f = clamp(1 - (t - WRITE[1]) / 0.8);
  if (t >= WRITE[1] && f > 0) out.push({ x: 960, y: 800, r: 420, i: 0.55 * smooth(f) * (1 + 0.1 * noise1(t * 20, 3)) });
  return out;
}

export default { id: 's5', start: 42, end: 54, flame, draw, fx, lights };
