// The cardboard box and the seven falls into it.
import { keys, span, lerp } from '../lib/time.js';
import { noise1 } from '../lib/rand.js';
import { ICONS, place, rect, solid } from './s4-icons.js';

export const HIT = 36.0;
export const fly = (i) => (i ? 0.72 : 0.5);
const S = 0.62;
export const landAt = (i) => 32.5 + 0.5 * i;

const FL = [250, 762];
const FR = [710, 762];
const BR = [672, 716];
const BL = [288, 716];
const FOOT = [[706, 950], [254, 950]];
export const FACE = [FL, FR, ...FOOT];
export const ENGINE = [480, 806];

export const HOLE = rect(432, 792, 96, 26, 13);
const CARD = [196, 150, 100];
const SHADE = [150, 104, 64];
const HOLLOW = [58, 36, 24];

const SLOTS = [
  [350, 730, -0.24], [610, 732, 0.2], [480, 716, 0.04], [406, 746, -0.1],
  [552, 750, 0.12], [318, 758, -0.36], [642, 760, 0.32],
];
const FROM = [
  [-150, 430, 250, -4.6], [1060, -190, 30, 3.7], [610, -220, 0, -2.7], [-170, 110, 170, 5.1],
  [2000, 520, 380, -4.2], [240, -210, 20, 3.1], [930, -230, 70, -5.3],
];

function shake(t, i) {
  let k = 0;
  for (let j = i + 1; j < 7; j++) {
    const tau = t - landAt(j);
    if (tau > 0) k += 0.05 * Math.exp(-9 * tau) * Math.sin(22 * tau + j);
  }
  const tau = t - HIT;
  if (tau > 0) k += 0.16 * Math.exp(-5 * tau) * Math.sin(17 * tau + i * 1.7);
  return k;
}

export function iconAt(i, t) {
  const t1 = landAt(i);
  const u = (t - (t1 - fly(i))) / fly(i);
  if (u < 0) return null;
  const [xs, ys, as] = SLOTS[i];
  const [x0, y0, hgt, spin] = FROM[i];
  if (u < 1) {
    const fall = 0.4 * u + 0.6 * u * u;
    return {
      x: lerp(x0, xs, u), y: lerp(y0, ys, fall) - 4 * hgt * u * (1 - u),
      a: as + spin * (1 - u) ** 1.4, s: S, inside: u > 0.9,
    };
  }
  const tau = t - t1;
  const hop = 16 * Math.exp(-8 * tau) * Math.abs(Math.sin(13 * tau));
  const out = Math.sign(xs - ENGINE[0]) * 8 * Math.exp(-5 * Math.max(0, t - HIT)) * (t > HIT ? 1 : 0);
  return {
    x: xs + out, y: ys - hop - 10 * span(t, HIT, HIT + 0.06) * Math.exp(-6 * Math.max(0, t - HIT - 0.06)),
    a: as + 0.2 * Math.sign(spin) * Math.exp(-6 * tau) * Math.sin(15 * tau) + shake(t, i), s: S, inside: true,
  };
}

export function velocity(i, t) {
  const a = iconAt(i, t - 1 / 48);
  const b = iconAt(i, t + 1 / 48);
  if (!a || !b) return [0, 0];
  return [(b.x - a.x) * 24, (b.y - a.y) * 24];
}

export function kick(t) {
  let k = 0;
  for (let i = 0; i < 7; i++) {
    const tau = t - landAt(i);
    if (tau > 0) k += 0.5 * Math.exp(-10 * tau) * Math.cos(20 * tau);
  }
  const tau = t - HIT;
  if (tau > 0) k += 2.4 * Math.exp(-4.5 * tau) * Math.cos(11 * tau);
  return k;
}

const sketch = (t) => span(t, 32.0, 32.42);

function flap(ink, hinge, tip, k, u) {
  const [a, b] = hinge;
  const far = tip.map(([x, y]) => [x, y - k * 26]);
  const shape = [a, b, far[1], far[0]];
  solid(ink, [shape], [214, 178, 130], u, 0.5);
  ink.wash([shape], { color: CARD, alpha: 0.85 * u, bleed: 2 });
  ink.hatch([shape], { gap: 7, angle: Math.atan2(b[1] - a[1], b[0] - a[0]) + 1.45, width: 1.2, alpha: 0.28 * u });
  ink.line([a, far[0], far[1], b], { width: 2.8, overshoot: 6, upto: u, head: u < 1 });
}

export function back(ink, t) {
  const u = sketch(t);
  if (u <= 0) return;
  flap(ink, [BL, BR], [[272, 636], [690, 630]], -kick(t) * 0.4, u);
  ink.wash([[FL, FR, BR, BL]], { color: HOLLOW, alpha: 0.9 * u, bleed: 3 });
  ink.hatch([[FL, FR, BR, BL]], { gap: 5, angle: -0.4, width: 1.4, alpha: 0.4 * u });
}

export function front(ink, t) {
  const u = sketch(t);
  if (u <= 0) return;
  const k = kick(t);
  const pen = { upto: u, head: u < 1 };
  solid(ink, [FACE], [214, 178, 130], u, 0.5);
  ink.wash([FACE], { color: CARD, alpha: 0.95 * u, bleed: 3 });
  ink.wash([FACE], { color: SHADE, alpha: 0.35 * u, bleed: 6 });
  ink.hatch([FACE], { gap: 6, angle: 1.52, width: 1.1, alpha: 0.22 * u, reach: 0.9 });
  ink.line([FL, FR], { width: 3.6, overshoot: 9, ...pen });
  ink.line([FR, FOOT[0], FOOT[1], FL], { width: 3.2, ...pen });
  ink.line([[480, 764], [480, 948]], { width: 1.4, alpha: 0.45, style: 'charcoal', upto: u });
  ink.wash([HOLE], { color: HOLLOW, alpha: 0.95 * u, bleed: 1 });
  ink.line(HOLE, { width: 2.6, closed: true, overshoot: 4, upto: u });
  flap(ink, [BL, FL], [[220, 668], [182, 736]], k, u);
  flap(ink, [FR, BR], [[778, 734], [740, 666]], k, u);
  ink.smudge(480, 958 + noise1(t * 3, 71), 260, { alpha: 0.12 * u, blots: 6 });
}

export function sitting(ink, t) {
  for (let i = 0; i < 7; i++) {
    const p = iconAt(i, t);
    if (p?.inside) place(ink, p, ICONS[i].paint);
  }
}

export function flying(ink, t) {
  const up = [];
  for (let i = 0; i < 7; i++) {
    const p = iconAt(i, t);
    if (p && !p.inside && p.y > -160) up.push([i, p]);
  }
  if (!up.length) return;
  const cx = up.reduce((s, [, p]) => s + p.x, 0) / up.length;
  const cy = up.reduce((s, [, p]) => s + p.y, 0) / up.length;
  ink.lift(70, [cx, cy], (k) => up.forEach(([i, p]) => place(k, p, ICONS[i].paint)));
}

export const glowOf = (t) => keys(t, [[35.6, 0], [HIT, 0.5, 'in'], [HIT + 0.04, 1.35, 'out'], [37.2, 0.85]]);
