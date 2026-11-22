// c03 strokes at time t, in pen order.
import { kit } from '../lib/kit/index.js';
import { TAU, COIL, coil, pill, comb, cornerRect, pair, blend, scaleAbout, wobble } from './c03-shapes.js';
import { SLACK } from './c03-slack.js';
import { slice } from '../lib/geom.js';

const { span, ease, lerp, keys } = kit;

const APPS = ['doc', 'table', 'show'];
const ROW = [300, 540, 780];
const EX = 470;
// The gate samples even frames; instant jumps land on them.
export const even = (t) => Math.ceil(t * 30 - 1e-9) / 30;
export const T = {
  arrow: 22.75, slack: even(22.875), click: 23.0, fly: 23.0625,
  dbl: 26.0, open: 26.25, land: 26.5, recoil: 26.75, gauge: 27.25, full: 28.0,
};
// Pen windows for each engine, bottom row first.
export const COILS = [[24.0, 24.1875], [24.1875, 24.375], [24.375, 24.5625]];

// Every other stroke reversed, so the pen zigzags.
const zig = (list, from = 0) => list.map((st, i) => (i >= from && (i - from) % 2 ? { ...st, pts: [...st.pts].reverse() } : st));
const faint = (list, link = 0.3) => list.map((st, i) => (i ? { ...st, link } : st));

const DOC = faint(zig(kit.icon('doc', 290, ROW[0], 150, { app: 'doc' }), 2));
const TABLE = faint(kit.icon('table', 290, ROW[1], 150, { app: 'table' }));
const SHOW = faint(kit.icon('show', 290, ROW[2], 150, { app: 'show' }));

const start = DOC[0].pts[0];
const a0 = Math.atan2(start[1] - ROW[0], start[0] - 290);
const RING = kit.circle(960, 372, 92, { from: a0, to: a0 + TAU }).pts;
const HEAD = kit.circle(960, 346, 31).pts;
const SHOULDERS = kit.arc(960, 478, 68, Math.PI + 0.66, TAU - 0.66).pts;
const AVATAR = [pair(RING, DOC[0].pts), pair(HEAD, DOC[1].pts, 72), pair(SHOULDERS, DOC[2].pts, 72)];

export const ARROW = [1142, 540];
export const TIP = [1160, 540];
const ARROW_PTS = [[1124, 540], TIP, [1146, 526], TIP, [1146, 554], TIP];
const DOTS = [0, 1, 2, 3, 4, 5].map((i) => [776 + 30 * i, 540]);

export const FILE_AT = [1640, 760];
const FILE = faint(zig(kit.icon('doc', FILE_AT[0], FILE_AT[1], 130, { app: 'doc' }), 2));
const LABEL = kit.text('Report.docx', { x: FILE_AT[0], y: 892, size: 40, align: 'center' });
export const LABEL_END = LABEL[LABEL.length - 1].pts.at(-1);
const ZERO = kit.text('0%', { x: 1416, y: 318, size: 48 });

const WIN = { x: 640, y: 430, w: 820, h: 470 };
export const CORNER = [WIN.x + WIN.w, WIN.y + 8];
const MID = [WIN.x + WIN.w / 2, WIN.y + WIN.h / 2];
const CHROME = kit.window(WIN.x, WIN.y, WIN.w, WIN.h, { chrome: 'windows', title: 'Report.docx', titleSize: 38, bar: 64, app: 'doc' });
const BAR = { ...CHROME[1], pts: [...CHROME[1].pts].reverse() };
const PARTS = faint([...CHROME.slice(6), ...CHROME.slice(2, 6), BAR], 0.4).map((st, i) => (i ? st : { ...st, link: 0.3 }));
const CONTENT = faint(zig([
  ...[470, 668, 640, 660].map((len, i) => kit.line(720, 814 - 56 * i, 720 + len, 814 - 56 * i, { app: 'doc' })),
  kit.line(720, 570, 1030, 570, { w: 1.5, app: 'doc' }),
]), 0.4).map((st, i) => (i ? st : { ...st, link: 0.4 }));
const OUTLINE = cornerRect(WIN.x, WIN.y, WIN.w, WIN.h, 8);

export const GAUGE = [1640, 300];
export const R = 66;
const TRACK = kit.circle(GAUGE[0], GAUGE[1], R, { w: 0.5 });
const GB = kit.text('4 GB', { x: GAUGE[0], y: GAUGE[1] + 15, size: 40, align: 'center' });
const USED = kit.arc(GAUGE[0], GAUGE[1], R, -Math.PI / 2, -Math.PI / 2 + TAU * 0.88, { w: 1.5 });

const hidden = (st) => ({ ...st, show: [0, 0] });
const SLACK_STUB = { pts: [[0, 0], [1, 0]], show: [0, 0], app: null, ...SLACK };
const shown = (st, u) => (u > 1e-4 ? { ...st, show: [0, Math.min(1, u)] } : hidden(st));
const at = (st, t, from) => (t < from ? hidden(st) : st);
const line = (pts, o = {}) => ({ pts, w: 1, alpha: 1, show: [0, 1], app: null, link: 1, ...o });

function field(t) {
  const fly = ease.inOut(span(t, T.fly, 23.4375));
  const press = ease.out(span(t, T.click, T.fly)) * (1 - fly);
  const land = wobble(t - 23.4375, -9, 15, 7);
  const cy = lerp(540, ROW[0], fly) + land;
  return pill(lerp(960, 1010, fly), cy, lerp(460, 740, fly) + 18 * press, Math.max(0, lerp(70, 0, fly) - 16 * press));
}

function breath(e, t) {
  return Math.sin((TAU * (t - 24)) / 4 + (e * TAU) / 3);
}

function settleOf(e, t) {
  if (e === 0 && t >= T.recoil) return t < 27.25 ? 0.8 : 1 - 0.2 * Math.exp(-5 * (t - 27.25)) * Math.cos(11 * (t - 27.25));
  const tau = t - COILS[2 - e][1];
  return tau < 0 ? 0.75 : 1 - 0.25 * Math.exp(-5 * tau) * Math.cos(11 * tau);
}

function half(e, h, t) {
  const pitch = COIL.pitch * (1 + 0.05 * breath(e, t)) * settleOf(e, t);
  const ry = COIL.ry * (1 + 0.025 * breath(e, t));
  return coil(EX, ROW[e], { pitch, ry, from: h * 0.5, to: h * 0.5 + 0.5 });
}

// Typed dots ride the field up into the line.
function dot(i, t) {
  const u = ease.inOut(span(t, T.fly, 23.4375));
  const rel = (DOTS[i][0] - 960) / 460;
  const c = [lerp(DOTS[i][0], 1010 + rel * 740, u), lerp(DOTS[i][1], ROW[0], u)];
  const k = T.fly + 0.03125 * (5 - i);
  const r = 6.5 * (1 - ease.in(span(t, k, k + 0.25)));
  const st = line(kit.circle(c[0], c[1], Math.max(r, 1)).pts, { app: APPS[Math.floor(i / 2)] });
  return r < 0.4 ? hidden(st) : st;
}

// Engines in pen order: ElyShow, ElyTable, then ElyDoc.
function engines(t) {
  const list = [2, 1, 0].flatMap((e) => [0, 1].map((h) => at(line(half(e, h, t), { app: APPS[e] }), t, COILS[2 - e][0])));
  if (t >= T.open && t < T.recoil) return list.map((st, i) => (i >= 4 ? hidden(st) : st));
  if (t >= T.recoil) {
    list[4] = shown(list[4], ease.out(span(t, T.recoil, 27.0)));
    list[5] = shown(list[5], ease.out(span(t, 27.0, 27.25)));
  }
  const show = kit.leave(list.slice(0, 2), ease.inOut(span(t, T.full, 28.375)));
  const table = kit.leave(list.slice(2, 4), ease.inOut(span(t, 28.125, 28.5)));
  return [...show, ...table, list[4], list[5]];
}

// The clicked arrow slides back into the last dot.
function arrow(t) {
  const u = ease.in(span(t, T.click, 23.1875));
  const d = [DOTS[5][0] - 1124, -6.5];
  const pts = ARROW_PTS.map(([x, y]) => [x + d[0] * u, y + d[1] * u]);
  return kit.leave([line(pts)], u);
}

export function arrowTip(t) {
  const st = arrow(t)[0];
  return slice(st.pts, st.show[0], st.show[1]).at(-1);
}

const DOC_COIL = [...half(0, 0, T.open), ...half(0, 1, T.open).slice(1)];
const OPENING = pair(DOC_COIL, OUTLINE, 320);

function windowParts(t) {
  const s = lerp(0.94, 1, ease.out(span(t, T.open, 26.75)));
  const fit = (st) => ({ ...st, pts: scaleAbout(st.pts, MID[0], MID[1], s) });
  const list = [...PARTS, ...CONTENT];
  const wipe = span(t, T.land, T.land + 0.125) * list.length;
  const parts = list.map((st, k) => fit(shown(st, wipe - k)));
  const morph = blend(OPENING, ease.inOut(span(t, T.open, T.land)));
  return [...parts, at(line(scaleAbout(morph, MID[0], MID[1], s), { app: 'doc' }), t, T.open)];
}

function avatar(t) {
  return AVATAR.map((p, i) => line(blend(p, ease.back(span(t, T.fly + 0.03125 * i, T.fly + 0.03125 * i + 0.4375)))));
}

function gauge(t) {
  const back = keys(t, [[28.125, 1], [28.625, 0.72 / 0.88]]);
  return [at(TRACK, t, T.gauge), ...GB.map((st) => at(st, t, 27.5)), t < 27.75 ? hidden(USED) : shown(USED, back)];
}

export const IDX = {};

function add(list, name, strokes) {
  if (!(name in IDX)) IDX[name] = [list.length, list.length + strokes.length - 1];
  list.push(...strokes);
}

export function build(t) {
  const list = [];
  add(list, 'avatar', avatar(t));
  add(list, 'bars', DOC.slice(3).map((st, i) => shown(st, ease.out(span(t, 23.3125 + 0.0625 * i, 23.4375 + 0.0625 * i)))));
  add(list, 'slackV', [SLACK_STUB]);
  add(list, 'tap', [kit.ripple(ARROW[0], ARROW[1], span(t, T.click, 23.25), { r: 16 })]);
  add(list, 'field', [line(field(t), { w: lerp(1, 1.3, ease.inOut(span(t, T.fly, 23.4375))) })]);
  add(list, 'slackF', [SLACK_STUB]);
  add(list, 'dots', DOTS.map((_, i) => dot(i, t)));
  add(list, 'arrow', t < T.arrow ? [hidden(arrow(t)[0])] : arrow(t));
  add(list, 'slackA', [SLACK_STUB]);
  add(list, 'table', TABLE.map((st) => at(st, t, 23.5)));
  add(list, 'show', SHOW.map((st) => at(st, t, 23.75)));
  add(list, 'engines', engines(t));
  add(list, 'slackB', [SLACK_STUB]);
  add(list, 'comb', [at(kit.stroke(comb(640, 356, 740, 20, 16, 74 * (t - 24.6875)), { w: 0.5, alpha: 0.6 }), t, 24.6875)]);
  add(list, 'zero', ZERO.map((st) => at(st, t, 25.0)));
  add(list, 'file', FILE.map((st) => at(st, t, 25.25)));
  add(list, 'label', LABEL.map((st) => at(st, t, 25.5)));
  add(list, 'click1', [{ ...kit.ripple(FILE_AT[0], FILE_AT[1], span(t, T.dbl, 26.3), { r: 18 }), app: 'doc' }]);
  add(list, 'window', windowParts(t));
  add(list, 'click2', [{ ...kit.ripple(FILE_AT[0], FILE_AT[1], span(t, 26.125, 26.5625), { r: 22 }), app: 'doc' }]);
  add(list, 'slackW', [SLACK_STUB]);
  add(list, 'gauge', gauge(t));
  return list.map((st) => ({ ...st, link: (st.link ?? 1) * 0.7 }));
}
