// s2: files find their Ely windows; the photo lands.
import { keys, span, clamp, lerp, ease } from '../lib/time.js';
import { REST, rested } from '../lib/flame.js';
import { hop, first } from '../lib/act.js';
import { burst } from '../lib/embers.js';
import { BLUE, GREEN, DOC, TAB, frame, sheet, photo, unfold, bloom } from './s2-desk.js';
import { page, sheetView, splash } from './s2-page.js';

const KICK = 10.0;
const DOC_OPEN = 10.5;
const FLING = 12.0;
const BUMP = 12.5;
const TAB_OPEN = 13.0;
const LET_GO = 15.5;
const LAND = 16.0;

const HOME = [REST.x, REST.y];
const KICKER = [382, 900];
const FLINGER = [730, 945];
const WATCH = [1060, 650];
const PICKER = [1240, 950];
const DROPPER = [608, 700];
const DOCX = [440, 860];
const XLSX = [660, 905];
const DESK = [1240, 880];
const SPOT = [608, 382];
const PAGE = [560, 350];
const BOUNCE = [790, 300];
const CELL = [1390, 430];
const RIDE = 318;
const LIFT = 150;

const H1 = { t0: 9.0, t1: 9.5, from: HOME, to: KICKER, height: 120, post: 0.25 };
const H2 = { t0: 11.0, t1: 11.5, from: KICKER, to: FLINGER, height: 90, post: 0.25 };
const H3 = { t0: 12.3, t1: 12.6, from: [FLINGER[0] - 10, FLINGER[1]], to: WATCH, height: 70, pre: 0.1, post: 0.3 };
const H4 = { t0: 13.6, t1: 14.0, from: WATCH, to: PICKER, height: 80, post: 0.2 };
const H5 = { t0: 14.6, t1: 15.0, from: PICKER, to: DROPPER, height: 70, pre: 0.1, post: 0.25 };
const H6 = { t0: 16.75, t1: 17.0, from: DROPPER, to: DROPPER, height: 45, pre: 0.12, post: 0.3 };
const H7 = { t0: 18.0, t1: 18.5, from: DROPPER, to: HOME, height: 110, post: 0.6 };

const at = (p, o) => ({ x: p[0], y: p[1], ...o });

function pose(t) {
  return first(
    t < 8.83 && { lean: keys(t, [[8.0, 0], [8.3, -0.3], [8.65, -0.24], [8.83, 0]]) },
    hop(t, H1),
    t < KICK && at(KICKER, { lean: keys(t, [[9.75, 0], [9.97, -0.45, 'out']]), squash: keys(t, [[9.75, 1], [9.97, 0.72, 'out']]) }),
    t < 10.83 && at(KICKER, {
      x: KICKER[0] + keys(t, [[KICK, 0], [10.07, 28, 'out'], [10.45, 6], [10.83, 0]]),
      lean: keys(t, [[KICK, -0.45], [10.07, 0.8, 'snap'], [10.3, 0.2], [10.6, 0.12], [10.83, 0]]),
      squash: keys(t, [[KICK, 0.72], [10.07, 1.38, 'snap'], [10.3, 0.92], [10.45, 1.04], [10.6, 1]]),
    }),
    hop(t, H2),
    t < FLING && at(FLINGER, { lean: keys(t, [[11.75, 0], [11.97, 0.45, 'out']]), squash: keys(t, [[11.75, 1], [11.97, 0.7, 'out']]) }),
    t < 12.2 && at(FLINGER, {
      x: FLINGER[0] + keys(t, [[FLING, 0], [12.06, -24, 'out'], [12.2, -10]]),
      lean: keys(t, [[FLING, 0.45], [12.06, -0.8, 'snap'], [12.2, 0]]),
      squash: keys(t, [[FLING, 0.7], [12.06, 1.4, 'snap'], [12.2, 1]]),
    }),
    hop(t, H3),
    t < 13.43 && at(WATCH, { lean: keys(t, [[12.9, 0], [13.05, 0.2], [13.43, 0]]) }),
    hop(t, H4),
    t < 14.5 && at(PICKER, { squash: keys(t, [[14.2, 1], [14.3, 0.78, 'out'], [14.42, 1.22, 'snap'], [14.5, 1]]) }),
    hop(t, H5),
    t < 16.63 && at(DROPPER, { squash: keys(t, [[LAND, 1], [16.04, 1.18, 'snap'], [16.3, 0.93], [16.5, 1]]) }),
    hop(t, H6),
    t < 17.83 && at(DROPPER, { lean: keys(t, [[17.3, 0], [17.5, 0.3], [17.65, 0.3], [17.83, 0]]) }),
    hop(t, H7),
    {},
  );
}

function flame(t) {
  return {
    ...pose(t),
    pool: keys(t, [[8.0, REST.pool], [8.75, 760, 'out'], [18.5, 760], [19.5, REST.pool]]),
    energy: keys(t, [[8.0, REST.energy], [9.0, 0.6], [KICK, 0.9], [10.4, 0.65], [FLING, 0.9], [12.4, 0.6], [TAB_OPEN, 0.85],
      [13.5, 0.6], [15.25, 0.6], [15.45, 0.3], [LAND, 1], [16.8, 0.7], [19.5, REST.energy]]),
    roar: keys(t, [[KICK, 0], [10.04, 0.35, 'out'], [10.3, 0], [DOC_OPEN, 0], [10.56, 0.25, 'out'], [10.9, 0],
      [FLING, 0], [12.04, 0.4, 'out'], [12.3, 0], [TAB_OPEN, 0], [13.06, 0.6, 'out'], [13.6, 0],
      [LAND, 0], [16.05, 0.65, 'out'], [16.7, 0.1], [17.0, 0]]),
    curl: keys(t, [[12.6, 0], [12.74, 0.72, 'back'], [12.93, 0.72], [TAB_OPEN, 0, 'snap']]),
    ember: keys(t, [[15.25, 0], [15.42, 0.62, 'out'], [LET_GO, 0.62], [15.96, 0.12, 'in'], [LAND, 0]]),
  };
}

function docxAt(t) {
  const u = span(t, KICK, DOC_OPEN);
  const e = ease.out(u);
  const lift = 160 * Math.sin(Math.PI * u);
  return { x: lerp(DOCX[0], PAGE[0], e), y: lerp(DOCX[1], PAGE[1], e) - 90 * Math.sin(Math.PI * e),
    lift, rot: -Math.PI * 2 * (1 - e), s: 1 + 0.0028 * lift };
}

function bez(a, c, b, u) {
  const v = 1 - u;
  return [0, 1].map((k) => v * v * a[k] + 2 * v * u * c[k] + u * u * b[k]);
}

function xlsxAt(t) {
  if (t < 12.45) {
    const e = ease.out(span(t, FLING, 12.45));
    const [x, y] = bez(XLSX, [560, 520], BOUNCE, e);
    const lift = 70 * e + 80 * Math.sin(Math.PI * e);
    return { x, y, lift, rot: 0.3 - Math.PI * 2 * (1 - e), s: 1 + 0.0028 * lift };
  }
  if (t < 12.55) {
    const w = Math.sin(((t - 12.45) / 0.1) * Math.PI * 2);
    return { x: BOUNCE[0] + 5 * w, y: BOUNCE[1], lift: 70, rot: 0.3 + 0.18 * w, s: 1 + 0.0028 * 70 };
  }
  const e = ease.inOut(span(t, 12.55, TAB_OPEN));
  const [x, y] = bez(BOUNCE, [1150, 120], CELL, e);
  const lift = 70 * (1 - e) + 110 * Math.sin(Math.PI * e);
  return { x, y, lift, rot: 0.3 + (Math.PI * 2 - 0.3) * e, s: 1 + 0.0028 * lift };
}

function carried(s) {
  const f = rested(pose(s));
  return [f.x, f.y - RIDE];
}

function photoAt(t) {
  if (t < 14.2) return { x: DESK[0], y: DESK[1], lift: 0, rot: -0.04, s: 1 };
  if (t >= LET_GO) {
    const lift = LIFT * (1 - ease.in(span(t, LET_GO, LAND)));
    return { x: SPOT[0], y: SPOT[1], lift, rot: 0, s: 1 + 0.0016 * lift };
  }
  const u = ease.out(span(t, 14.2, 14.5));
  const [cx, cy] = carried(t - 0.08);
  const [px] = carried(t - 0.14);
  const bob = 4 * Math.sin(t * 11) * (1 - span(t, 15.2, 15.45));
  const lift = LIFT * u;
  return { x: lerp(DESK[0], cx, u), y: lerp(DESK[1], cy, u) + bob * u, lift,
    rot: lerp(-0.04, clamp((cx - px) / 300, -0.35, 0.35), u), s: 1 + 0.0016 * lift };
}

function trail(ink, t, path, t0, t1) {
  const pts = [];
  for (let k = 0; k <= 10; k++) {
    const s = clamp(t - 0.16 + k * 0.016, t0, t1);
    const p = path(s);
    pts.push([p.x, p.y]);
  }
  ink.curve(pts, { style: 'charcoal', width: 4, alpha: 0.4, taper: [0.9, 0.05], overshoot: 0, boil: 1.2 });
}

function updraft(ink, t) {
  const fade = span(t, 14.3, 14.5) * (1 - span(t, 15.25, 15.45));
  if (fade <= 0) return;
  const f = rested(pose(t));
  const p = photoAt(t);
  const top = f.y - f.h * f.squash - 6;
  const bottom = p.y + 64 * p.s + 8;
  if (top - bottom < 20) return;
  for (const dx of [-20, 0, 20]) {
    const pts = [];
    for (let k = 0; k <= 6; k++) {
      const y = lerp(top, bottom, k / 6);
      pts.push([lerp(f.x, p.x, k / 6) + dx * (1 - 0.3 * k / 6) + 5 * Math.sin(k * 1.4 - t * 14 + dx), y]);
    }
    ink.curve(pts, { style: 'charcoal', width: 1.8, alpha: 0.5 * fade, taper: [0.4, 0.4], overshoot: 0, boil: 1.2 });
  }
}

function flying(ink, p, paint) {
  ink.lift(p.lift, [p.x, p.y], (k) => {
    k.ctx.save();
    k.ctx.translate(p.x, p.y);
    k.ctx.rotate(p.rot);
    k.ctx.scale(p.s, p.s);
    paint(k);
    k.ctx.restore();
  });
}

function lying(ink, t, [x, y], gone, name, paint) {
  if (t >= gone) {
    ink.smudge(x, y + 30, 46, { id: `left-${name}`, alpha: 0.07 });
    return;
  }
  const ctx = ink.ctx;
  ctx.save();
  ctx.translate(x, y);
  paint(ink);
  ctx.restore();
  ink.text(name, { x, y: y + (name.endsWith('jpg') ? 94 : 80), size: 22, align: 'center', alpha: 0.8 });
}

function draw(ctx, t, ink) {
  bloom(ink, t, DOC_OPEN, PAGE, DOC, BLUE);
  bloom(ink, t, TAB_OPEN, CELL, TAB, GREEN);
  const shiver = t >= BUMP ? 3.5 * Math.exp(-(t - BUMP) * 6) : 0;
  frame(ink, t, DOC, { id: 'doc', t0: 8.0, color: BLUE, tint: t < DOC_OPEN ? 0.16 : lerp(0.16, 0.42, span(t, DOC_OPEN, DOC_OPEN + 0.3)),
    shiver, titles: [['ElyDoc', 8.45, DOC_OPEN], ['notes.docx', DOC_OPEN, 99]] });
  frame(ink, t, TAB, { id: 'tab', t0: 8.25, color: GREEN, tint: t < TAB_OPEN ? 0.16 : lerp(0.16, 0.42, span(t, TAB_OPEN, TAB_OPEN + 0.3)),
    titles: [['ElyTable', 8.7, TAB_OPEN], ['budget.xlsx', TAB_OPEN, 99]] });
  page(ink, t, { open: DOC_OPEN, land: LAND, hit: BUMP, bump: BOUNCE, spot: SPOT });
  sheetView(ink, t, TAB_OPEN);
  lying(ink, t, DOCX, KICK, 'notes.docx', (k) => sheet(k, 'doc'));
  lying(ink, t, XLSX, FLING, 'budget.xlsx', (k) => sheet(k, 'table'));
  lying(ink, t, DESK, 14.2, 'hills.jpg', photo);
  if (t >= LAND) {
    ctx.save();
    ctx.translate(SPOT[0], SPOT[1]);
    photo(ink);
    ctx.restore();
  }
  splash(ink, t, LAND, SPOT);
  unfold(ink, t, DOC_OPEN, PAGE, DOC);
  unfold(ink, t, TAB_OPEN, CELL, TAB);
  if (t >= KICK && t < DOC_OPEN) {
    trail(ink, t, docxAt, KICK, DOC_OPEN);
    flying(ink, docxAt(t), (k) => sheet(k, 'doc'));
  }
  if (t >= FLING && t < TAB_OPEN) {
    trail(ink, t, xlsxAt, FLING, TAB_OPEN);
    flying(ink, xlsxAt(t), (k) => sheet(k, 'table'));
  }
  updraft(ink, t);
  if (t >= 14.2 && t < LAND) flying(ink, photoAt(t), photo);
}

function fx(ctx, t) {
  burst(ctx, t, { t0: KICK, x: DOCX[0] - 20, y: DOCX[1] + 20, count: 26, speed: 420, angle: -0.9, spread: 0.8, life: 0.55, gravity: 380, seed: 201, size: 2 });
  burst(ctx, t, { t0: DOC_OPEN, x: PAGE[0], y: PAGE[1], count: 44, speed: 460, life: 0.75, gravity: 160, seed: 202, size: 2.1 });
  burst(ctx, t, { t0: FLING, x: XLSX[0] + 30, y: XLSX[1] + 10, count: 24, speed: 420, angle: -2.0, spread: 0.7, life: 0.55, gravity: 380, seed: 203, size: 2 });
  burst(ctx, t, { t0: BUMP - 0.05, x: BOUNCE[0], y: BOUNCE[1], count: 14, speed: 260, life: 0.4, gravity: 300, seed: 204, size: 1.6 });
  burst(ctx, t, { t0: TAB_OPEN, x: CELL[0], y: CELL[1], count: 44, speed: 460, life: 0.75, gravity: 160, seed: 205, size: 2.1 });
  burst(ctx, t, { t0: LAND, x: DROPPER[0], y: DROPPER[1] - 110, count: 50, speed: 520, angle: -Math.PI / 2, spread: 0.9, life: 0.8, gravity: 220, seed: 206, size: 2.2 });
}

function lights(t) {
  const out = [];
  for (const [t0, [x, y]] of [[DOC_OPEN, PAGE], [TAB_OPEN, CELL]]) {
    if (t >= t0) out.push({ x, y, r: 400, i: 0.5 * Math.exp(-(t - t0) * 5), c: [1.06, 1, 0.92] });
  }
  return out;
}

export default { id: 's2', start: 8, end: 20, flame, draw, fx, lights };
