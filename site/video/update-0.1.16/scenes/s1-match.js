// The match and its box, for s1.
import { keys, clamp, span, smooth } from '../lib/time.js';
import { noise1 } from '../lib/rand.js';

const WOOD = [222, 186, 132];
const HEAD = [182, 50, 38];
const CHAR = [42, 30, 26];
const SAND = [118, 64, 46];

export const STRIKE = 2.0;
export const LEAVE = 3.5;

const PATH = [
  [0, [760, 800, 0.32]],
  [0.75, [690, 826, 0.3]],
  [0.9, [702, 833, 0.3], 'in'],
  [1.0, [598, 843, 0.28], 'out'],
  [1.25, [692, 825, 0.3]],
  [1.4, [702, 832, 0.3], 'in'],
  [1.5, [540, 846, 0.27], 'out'],
  [1.75, [696, 824, 0.3]],
  [1.86, [706, 831, 0.31], 'in'],
  [2.0, [420, 852, 0.26], 'linear'],
  [2.2, [452, 712, 0.72], 'out'],
  [2.5, [556, 604, 1.02], 'out'],
  [3.3, [566, 598, 1.0]],
  [3.5, [570, 606, 0.98]],
  [3.75, [548, 560, 0.86], 'out'],
  [4.7, [330, -160, 0.7], 'in'],
];

export function head(t) {
  const [x, y, a] = keys(t, PATH);
  const hold = span(t, 2.4, 2.6) * (1 - span(t, 3.4, 3.6));
  return [x + 5 * noise1(t * 1.3, 3) * hold, y + 4 * noise1(t * 1.1, 4) * hold, a];
}

export function box(ink) {
  const top = [[300, 806], [650, 784], [712, 822], [338, 850]];
  const side = [[338, 850], [712, 822], [714, 856], [342, 888]];
  ink.wash([top], { color: [214, 196, 168], alpha: 0.45, bleed: 4 });
  ink.wash([side], { color: SAND, alpha: 0.6, bleed: 3 });
  ink.hatch([side], { gap: 5, angle: 0.15, width: 1.4, alpha: 0.45 });
  for (const [a, b] of [[0, 1], [1, 2], [2, 3], [3, 0]]) ink.line([top[a], top[b]], { width: 2.4 });
  ink.line([side[1], side[2]], { width: 2.2 });
  ink.line([side[2], side[3]], { width: 2.6 });
  ink.line([side[3], side[0]], { width: 2.2 });
  const s = ink.seed();
  for (let i = 0; i < 160; i++) {
    const u = ((i * 0.618) % 1);
    const v = ((i * 0.381 + 0.2) % 1);
    const x = 338 + u * 374 + 2 * noise1(i, s);
    const y = 850 - u * 28 + v * 34;
    ink.ctx.fillStyle = 'rgba(40,24,18,0.55)';
    ink.ctx.fillRect(x, y, 1.6, 1.6);
  }
}

export function scorch(ink, t) {
  const u = span(t, 1.86, 2.0);
  if (u <= 0) return;
  const x = 706 + (420 - 706) * u;
  ink.line([[706, 840], [x, 846 + 6 * u]], { style: 'charcoal', width: 9, alpha: 0.55 * u, color: CHAR });
}

export function match(ink, t) {
  const [hx, hy, a] = head(t);
  if (Math.max(hy, hy + Math.sin(a) * 340) < -30) return;
  stick(ink, [hx, hy, a], t > STRIKE ? smooth(span(t, STRIKE, LEAVE + 0.6)) : 0, t >= STRIKE);
}

export function stick(ink, [hx, hy, a], burnt, lit) {
  const c = Math.cos(a);
  const s = Math.sin(a);
  const len = 340;
  const w = 7;
  const along = (d, off) => [hx + c * d - s * off, hy + s * d + c * off];
  const stick = [[along(8, -w), along(len, -w * 0.9), along(len, w * 0.9), along(8, w)]];
  ink.wash(stick, { color: WOOD, alpha: 0.75, bleed: 1.5 });
  ink.line([along(10, -w), along(len, -w * 0.9)], { width: 2.2 });
  ink.line([along(10, w), along(len, w * 0.9)], { width: 2.6 });
  ink.line([along(len, -w * 0.9), along(len + 2, w * 0.9)], { width: 2 });
  ink.line([along(60, 1), along(220, 2)], { width: 1, alpha: 0.4, style: 'charcoal' });
  const char = 46 * burnt;
  if (char > 1) {
    const black = [[along(10, -w), along(10 + char, -w * 0.95), along(16 + char, 0), along(10 + char, w * 0.95), along(10, w)]];
    ink.wash(black, { color: CHAR, alpha: 0.85, bleed: 2 });
    ink.hatch(black, { gap: 3, width: 1.2, angle: a + 1.2, alpha: 0.6 });
  }
  const blob = [];
  for (let i = 0; i < 18; i++) {
    const q = (i / 18) * Math.PI * 2;
    const r = 1 + 0.06 * Math.sin(q * 3);
    blob.push(along(Math.cos(q) * 19 * r + 6, Math.sin(q) * 13 * r));
  }
  ink.wash([blob], { color: lit ? CHAR : HEAD, alpha: lit ? 0.9 : 0.85, bleed: 2 });
  ink.hatch([blob], { gap: 3.2, width: 1.2, angle: a - 0.8, alpha: lit ? 0.7 : 0.4, reach: 0.6 });
  ink.line([...blob, blob[0], blob[1]], { width: 2.4, overshoot: 6 });
}

export function smoke(ink, t) {
  const age = t - LEAVE;
  if (age < 0 || age > 1.6) return;
  const [hx, hy] = head(t);
  for (let k = 0; k < 2; k++) {
    const rise = 40 + age * 160;
    const pts = [];
    for (let i = 0; i <= 8; i++) {
      const u = i / 8;
      pts.push([hx + 14 * Math.sin(u * 5 + age * 3 + k * 2) * u + k * 8, hy - 10 - u * rise]);
    }
    const fade = clamp(1 - age / 1.6);
    ink.curve(pts, { style: 'charcoal', width: 3.2 - k, alpha: 0.4 * fade, from: clamp(age * 0.35), taper: [0.3, 0.5] });
  }
}
