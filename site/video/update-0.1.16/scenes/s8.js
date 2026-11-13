// s8: a blank page; a text block leaves its marks.
import { keys, span, clamp, ease, spring } from '../lib/time.js';
import { REST } from '../lib/flame.js';
import { hop, first } from '../lib/act.js';
import { burst } from '../lib/embers.js';
import { glyphs } from '../lib/glyph.js';
import * as pdf from './s8-page.js';

const LINES = ['见字如面。', '院子里的杏花开了，', '我把去年的信誊了一遍，', '寄给你。'];
const SIZE = 36;
const BX = 735;
const BY = 270;
const LEAD = 56;
const TYPE = [76.6, 77.35];
const POPS = [77.5, 77.75, 78.0];
const MOVE = [78.65, 79.95];
const BEND = [210, 110];
const DEST = [60, 330];
const GRIP = [BX + 52, BY + 6];
const WATCH = [880, 650];
const MARKS = [[0, -0.3], [1, 7], [3, 4]].map(([line, j]) => [BX + j * SIZE, BY + line * LEAD + 4]);
const twos = (t) => Math.floor(t * 12 + 1e-6) / 12;

function offset(t) {
  const u = ease.inOut(span(t, MOVE[0], MOVE[1]));
  const v = 1 - u;
  return [2 * u * v * BEND[0] + u * u * DEST[0], 2 * u * v * BEND[1] + u * u * DEST[1]];
}

const height = (t) => keys(t, [[78.5, 0], [MOVE[0], 26, 'out'], [MOVE[1] - 0.05, 26], [80.0, 0, 'in']]);

function edge(t) {
  return [960, pdf.edgeY(t) + 4];
}

function folding(t) {
  if (t < 74.58) return { squash: 1 - spring(t - pdf.DROP[1], 0.18, 14, 7) };
  if (t < pdf.OPEN[0]) return hop(t, { t0: 74.75, t1: pdf.OPEN[0], from: [REST.x, REST.y], to: edge(pdf.OPEN[0]), height: 70, post: 0 });
  if (t < pdf.OPEN[1]) {
    const tq = twos(t);
    const s = Math.sin(pdf.theta(tq));
    const [x, y] = edge(tq);
    return { x, y, h: 96 + 46 * s, squash: 1 + 0.18 * s, lean: 0.1 * Math.sin(t * 9), energy: 0.9 };
  }
  const bottom = edge(pdf.OPEN[1]);
  const c = pdf.corner(pdf.LAND);
  return first(
    hop(t, { t0: 75.8, t1: pdf.LAND, from: bottom, to: [c[0] + 6, c[1] + 2], height: 90, pre: 0.14, post: 0 }),
    { x: bottom[0], y: bottom[1], squash: 1 - spring(t - pdf.OPEN[1], 0.32, 15, 7) },
  );
}

function after(t) {
  if (t < 76.62) {
    const [x, y] = pdf.corner(twos(t));
    return { x: x + 6, y: y + 2, squash: 1 - spring(t - pdf.LAND, 0.42, 15, 6), roar: keys(t, [[76.0, 0.6], [76.4, 0]]), energy: 0.9 };
  }
  const c = pdf.corner(76.62);
  if (t < 78.03) {
    return first(hop(t, { t0: 76.78, t1: 77.05, from: [c[0] + 6, c[1] + 2], to: WATCH, height: 120, pre: 0.16 }),
      { x: WATCH[0], y: WATCH[1], lean: 0.12 * Math.sin(Math.PI * span(t, 77.4, 78.0)) });
  }
  if (t < MOVE[0]) {
    const s = hop(t, { t0: 78.2, t1: 78.5, from: WATCH, to: GRIP, height: 110 }) ?? { x: GRIP[0], y: GRIP[1] };
    return { ...s, y: s.y - 26 * span(t, 78.5, MOVE[0]) };
  }
  if (t < 80.0) {
    const tq = twos(t);
    const [dx, dy] = offset(tq);
    const [ax, ay] = offset(t - 1 / 24);
    const [bx, by] = offset(t + 1 / 24);
    return { x: GRIP[0] + dx, y: GRIP[1] + dy - height(tq), lean: clamp((bx - ax) * 12 / 900, -0.5, 0.5),
      squash: clamp(1 + (by - ay) * 12 / 3000, 0.85, 1.2), energy: 0.85 };
  }
  const end = [GRIP[0] + DEST[0], GRIP[1] + DEST[1]];
  return first(
    hop(t, { t0: 80.7, t1: 81.0, from: end, to: [REST.x, REST.y], height: 110, post: 0.45 }),
    t < 80.7 && { x: end[0], y: end[1], squash: 1 - spring(t - 80.0, 0.36, 15, 7), roar: keys(t, [[80.0, 0.35], [80.3, 0]]) },
    {},
  );
}

function flame(t) {
  const pool = keys(t, [[74, REST.pool], [74.6, 760], [81.0, 760], [81.8, REST.pool]]);
  const h = keys(t, [[74.6, REST.h], [75.0, 96], [80.7, 96], [81.0, REST.h, 'out']]);
  return { h, pool, ...(t < pdf.LAND ? folding(t) : after(t)) };
}

function block(t, d) {
  const out = [];
  LINES.forEach((line, k) => out.push(...glyphs(line, { x: BX + d[0], y: BY + k * LEAD + d[1], size: SIZE })));
  return out;
}

function marks(ink, t) {
  MARKS.forEach((p, k) => {
    const s = ease.back(span(t, POPS[k] - 0.12, POPS[k] + 0.08));
    pdf.pin(ink, p, 1.3 * s, pdf.wobble(t, k));
  });
}

function draw(ctx, t, ink) {
  if (t < pdf.DROP[1] + 0.05) {
    ink.lift(keys(t, [[pdf.DROP[0], 140], [pdf.DROP[1], 0, 'in']]), [960, 350 + pdf.dropY(t)], (k) => pdf.sheet(k, t));
    return;
  }
  pdf.sheet(ink, t);
  pdf.label(ink, t);
  if (t < TYPE[0]) return;
  const o = { size: SIZE, fill: 'solid', alpha: 0.9, upto: span(t, TYPE[0], TYPE[1]) };
  const lift = height(t);
  if (lift <= 0 && t < MOVE[0]) {
    ink.write(block(t, [0, 0]), o);
    marks(ink, t);
    return;
  }
  marks(ink, t);
  const d = offset(t);
  if (lift > 0) ink.lift(lift, [BX + 180 + d[0], BY + 84 + d[1]], (k) => k.write(block(t, d), o));
  else ink.write(block(t, d), o);
}

function fx(ctx, t) {
  const [ex, ey] = edge(pdf.OPEN[1]);
  burst(ctx, t, { t0: pdf.OPEN[1], x: ex, y: ey - 30, count: 20, speed: 300, life: 0.5, spread: 1.2, gravity: 360, seed: 81, size: 1.8 });
  const [cx, cy] = pdf.corner(pdf.LAND);
  burst(ctx, t, { t0: pdf.LAND, x: cx, y: cy - 30, count: 40, speed: 460, life: 0.7, spread: 1.4, gravity: 300, seed: 82, size: 2.1 });
  burst(ctx, t, { t0: 80.0, x: GRIP[0] + DEST[0], y: GRIP[1] + DEST[1] - 30, count: 24, speed: 300, life: 0.55, spread: 1.3, gravity: 340, seed: 83, size: 1.8 });
}

export default { id: 's8', start: 74, end: 82, flame, draw, fx };
