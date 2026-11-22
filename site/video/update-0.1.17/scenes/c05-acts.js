// The four banners' fates, frame by frame.
import { kit } from '../lib/kit/index.js';
import * as P from './c05-parts.js';
import { foldAll, foldX, settle, shift, fade, box, byGlyph, tumble, rectFrom } from './c05-motion.js';

const { keys, span, ease, lerp } = kit;

export const T = {
  draw: [38.75, 39.375],
  grab: 39.5, dog: 39.625, off: 39.875, gone: 40.125, land: 40.0,
  press: 41.0, drag: 41.125, dialog: 42.0,
  click: 42.25, close: [42.3125, 42.5625],
  hold: 42.5625, blind: [42.625, 42.9375], error: 43.0,
  reopen: 43.5, roll: [43.5625, 43.8125], zip: [43.75, 43.9375],
  touch: 44.5, curl: 44.75, laid: 45.0625,
};

export const C0 = [P.STRIP.x, P.links.top + P.STRIP.h];
const DOG = [C0[0] + 22, C0[1] - 17];
export const PULL = [C0[0] + 780, C0[1] + 150];
export const pull = (t) => keys(t, [[T.grab, C0], [T.dog, DOG, 'out'], [T.off, PULL, 'inOut']]);
export const THROW = [PULL[0] + 366, PULL[1] - 200];
export const thrown = (t) => keys(t, [[T.off, PULL], [T.gone, THROW, 'out']]);

export const CF = [P.STRIP.x, P.fonts.top + P.STRIP.h];
const CURL = [0.8, -0.6];
const curlD = (t) => keys(t, [[T.touch, 0], [T.curl, 58, 'out'], [T.laid, 0, 'inOut']]);
export const curlAt = (t) => {
  const d = curlD(t);
  return [CF[0] + CURL[0] * d, CF[1] + CURL[1] * d];
};

export const rise = (t) => {
  const a = P.PITCH * ease.back(span(t, T.off, T.off + 0.3125));
  const b = P.PITCH * ease.back(span(t, 41.25, 41.5625));
  const c = P.PITCH * ease.inOut(span(t, 43.9375, 44.5));
  return [a, a + b, a + b + c];
};

export const dim = (t) => 1 - 0.72 * (ease.inOut(span(t, 41.1875, 41.8125)) - ease.inOut(span(t, T.close[0], 42.625)));

const FONT_STRIP = rectFrom(P.fonts.strip, CF);
export function fonts(t, a) {
  const g = P.fonts;
  const body = [FONT_STRIP, ...g.icon];
  const [strip, ...icon] = t > T.touch && t < T.laid ? foldAll(body, CF, curlAt(t)) : body;
  return fade(P.head([strip, ...g.text, ...icon], 0.25), a);
}

const TAPE = P.head([rectFrom(P.links.strip, C0), ...P.links.icon], 0.4);
export function tape(t) {
  if (t <= T.grab) return TAPE;
  const f = foldAll(TAPE, C0, pull(t));
  if (t <= T.off) return f;
  const u = span(t, T.off, T.gone);
  const hand = thrown(t);
  const m = kit.move(f, { x: hand[0] - PULL[0], y: hand[1] - PULL[1], s: 1 - 0.85 * ease.out(u), ox: PULL[0], oy: PULL[1] });
  return kit.leave(m, ease.out(u));
}

const WORDS = P.head(P.reversed(P.links.text), 0.3);
const GLYPHS = byGlyph(WORDS).map((idx) => {
  const b = box(idx.map((i) => WORDS[i]));
  let rel = T.off;
  for (let t = T.dog; t <= T.off; t += 1 / 480) {
    if (foldX(C0, pull(t)) >= b.x0) {
      rel = t;
      break;
    }
  }
  return { idx, c: [b.cx, b.cy], rel, spin: (kit.hash('c05', b.x0) - 0.5) * 1.1 };
});
const FLY = T.land - Math.max(...GLYPHS.map((g) => g.rel));
const TRAVEL = [P.linksAtFoot.dx, P.linksAtFoot.dy];
const FOOT = shift(WORDS, ...TRAVEL);
export const WORDS_END = WORDS.at(-1).pts.at(-1);

export function words(t, a) {
  if (t >= T.land + 0.5) return fade(FOOT, a);
  const out = WORDS.slice();
  for (const g of GLYPHS) {
    const u = span(t, g.rel, g.rel + FLY);
    if (u <= 0) continue;
    const part = g.idx.map((i) => WORDS[i]);
    const moved = u < 1 ? tumble(part, g.c, TRAVEL, u, g.spin) : shift(part, TRAVEL[0], TRAVEL[1] + settle(t - g.rel - FLY, 7));
    g.idx.forEach((i, j) => (out[i] = moved[j]));
  }
  return fade(out, a);
}

export function riskBox(t, lift) {
  const D = P.DIALOG;
  const press = 3 * Math.sin(Math.PI * span(t, T.press, T.drag));
  const mx = ease.inOut(span(t, T.drag, 41.6875));
  const my = ease.inOut(span(t, 41.1875, 41.75));
  const mh = ease.back(span(t, 41.3125, T.dialog));
  return {
    x: lerp(P.STRIP.x, D.x, mx), y: lerp(P.risk.top - lift, D.y, my) + press,
    w: lerp(P.STRIP.w, D.w, mx), h: lerp(P.STRIP.h, D.h, mh) - 2 * press, r: lerp(P.STRIP.r, D.r, mx), mx, mh,
  };
}

const RB = P.risk.button;
export function riskButton(b) {
  return [b.x + b.w - RB.w - lerp(8, 32, b.mx), b.y + b.h - P.BTN - lerp(8, 32, b.mh)];
}

export function risk(t, lift) {
  const g = P.risk;
  const b = riskBox(t, lift);
  const at = (x0, y0, x1, y1, list) => shift(list, b.x + lerp(x0, x1, b.mx) - P.STRIP.x - x0, b.y + lerp(y0, y1, b.mh) - g.top - y0);
  const icon = at(32, 32, 48, 74, g.icon);
  const text = at(64, 42, 84, 84, g.text);
  const [bx, by] = riskButton(b);
  const btn = shift([RB.rect, ...RB.text], bx - RB.rect.pts[0][0] + 8, by - g.top - 8);
  const edge = rectFrom(kit.rect(b.x, b.y, b.w, b.h, { r: b.r, w: P.SHAPE }), [b.x + b.w, b.y + b.h]);
  let out = [...icon, ...text, ...btn, edge];
  if (t > T.close[0]) {
    const u = span(t, ...T.close);
    const c = [P.DIALOG.x + P.DIALOG.w / 2, P.DIALOG.y + P.DIALOG.h / 2];
    out = kit.leave(kit.move(out, { s: 1 - 0.08 * ease.in(u), ox: c[0], oy: c[1] }), ease.in(u));
  }
  return P.head(out, 0.4);
}

const FULL = P.PAGE.foot - P.rowTop(1);
export const blind = (t) => keys(t, [[T.blind[0], P.STRIP.h], [T.blind[1], FULL, 'in'], [T.roll[0], FULL], [T.roll[1], 22, 'inOut']])
  - settle(t - T.blind[1], 18);
export const PULL_F = 0.17;
export const zip = (t) => P.STRIP.w * (1 - ease.inOut(span(t, ...T.zip)));

const SG = P.stopped;
const SB = SG.button;
const TITLE_W = kit.measure('The document engine stopped', { size: P.SIZE }).w;
const ERR = { icon: [960, 470], s: 3, title: [960 - (TITLE_W * 34) / 28 / 2, 566], ts: 34 / 28, button: [960 - SB.w / 2, 606] };
export const REOPEN = [960, 606 + P.BTN / 2];

export function stopped(t, lift) {
  const top = SG.top - lift;
  const dy = top - SG.top;
  const h = blind(t);
  const w = zip(t);
  const box0 = kit.rect(P.STRIP.x, top, Math.max(w, 1), h, { r: Math.min(P.STRIP.r, w / 2), w: P.SHAPE });
  const strip = { ...rectFrom(box0, [P.STRIP.x + PULL_F * w, top + h]), show: w > 4 ? [0, 1] : [0, 0] };
  const go = (i, from, to, pad) => {
    const a = T.blind[0] + 0.0625 * i;
    const u = ease.inOut(span(t, a, a + 0.25));
    const room = (top + h - pad - from) / (to - from);
    return { u, v: Math.min(Math.max(u, ease.out(span(t, a, a + 0.1875))), Math.max(0, room)), b: settle(t - a - 0.25, 6) };
  };
  const m0 = go(0, top + 32, ERR.icon[1], 196);
  const m1 = go(1, top + 42, ERR.title[1], 100);
  const m2 = go(2, top + 8, ERR.button[1], 60);
  const ic = [P.STRIP.x + 32, top + 32];
  const icon = kit.move(shift(SG.icon, 0, dy), { s: lerp(1, ERR.s, Math.min(m0.u, m0.v)), ox: ic[0], oy: ic[1],
    x: (ERR.icon[0] - ic[0]) * Math.min(m0.u, m0.v), y: (ERR.icon[1] - ic[1]) * m0.v + m0.b });
  const tx = [P.STRIP.x + 64, top + 42];
  const text = kit.move(shift(SG.text, 0, dy), { s: lerp(1, ERR.ts, Math.min(m1.u, m1.v)), ox: tx[0], oy: tx[1],
    x: (ERR.title[0] - tx[0]) * Math.min(m1.u, m1.v), y: (ERR.title[1] - tx[1]) * m1.v + m1.b });
  const b0 = [SB.rect.pts[0][0] - 8, SB.rect.pts[0][1] + dy];
  const btn = shift([SB.rect, ...SB.text], (ERR.button[0] - b0[0]) * Math.min(m2.u, m2.v), dy + (ERR.button[1] - b0[1]) * m2.v + m2.b);
  const inner = kit.leave([...icon, ...text, ...btn], ease.in(span(t, T.reopen, 43.6875)));
  return P.head([...inner, strip], 0.4);
}

export const blindEdge = (t, lift) => (t > T.blind[0] - 0.0625 && t < T.zip[1] ? SG.top - lift + blind(t) + 10 : -Infinity);

const off = (x, y) => ({ ...kit.line(x, y, x, y - 1), show: [0, 0] });
const vis = (v) => (v > 1e-4 ? [0, Math.min(1, v)] : [0, 0]);

export function page(t, lift, yc, a) {
  const G = P.PAGE;
  const top = G.top - lift;
  const y0 = Math.max(top, yc);
  const L = G.x;
  const R = G.x + G.w;
  const right = y0 < G.foot - 1 ? kit.line(R, G.foot, R, y0, { w: P.SHAPE }) : off(R, G.foot);
  const edge = { ...kit.line(R, top, L, top, { w: P.SHAPE }), show: vis((top - yc) / 24) };
  const left = y0 < G.foot - 1 ? kit.line(L, y0, L, G.foot, { w: P.SHAPE }) : off(L, G.foot);
  const lines = P.LINES.map(([d, len, w = 1.05], i) => {
    const y = top + d;
    const [x0, x1] = i % 2 ? [L + 64 + len, L + 64] : [L + 64, L + 64 + len];
    return { ...kit.line(x0, y, x1, y, { w }), show: vis(Math.min((G.clip - y) / 24, (y - yc) / 24)) };
  });
  return fade(P.head([right, edge, left, ...P.head(lines, 0.2)], 0.12), a);
}

export const status = (a) => fade([{ ...kit.line(P.WIN.x, P.RULE, P.WIN.x + P.WIN.w, P.RULE, { w: P.SHAPE }), link: 0.12 }, ...P.head(P.pageNo, 0.3)], a);
