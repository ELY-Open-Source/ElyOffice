// What appears while dragging: ripple, guides, spacing marks, Alt.
import { kit } from '../lib/kit/index.js';
import { A, B, C, D, SLIDE, CENTRE, roundRect } from './c07-stage.js';
import { HIT, GAPS, SNAPS, grip, free } from './c07-motion.js';

const { ease, span, spring } = kit;
const IN = 0.15;
const OUT = 0.1875;
const GUIDE = { w: 0.9 };
const quiet = (st) => ({ ...st, link: 0 });

function life(t, a, back) {
  const grow = ease.out(span(t, a, a + IN));
  const gone = back === null ? 0 : ease.in(span(t, back, back + OUT));
  return grow * (1 - gone);
}

const shown = (s, b) => ({ ...s, show: b > 1e-4 ? [0, b] : [0, 0] });

// Grows from the middle out, and back.
const bloom = (s, b) => ({ ...s, show: b > 1e-4 ? [0.5 - b / 2, 0.5 + b / 2] : [0, 0] });

// Reveals strokes in order, as one pen would.
export function drawOn(strokes, u) {
  const lens = strokes.map((s) => kit.length(s));
  const total = lens.reduce((a, b) => a + b, 0);
  let run = 0;
  return strokes.map((s, i) => {
    const b = kit.clamp((u * total - run) / lens[i]);
    run += lens[i];
    return shown(s, b);
  });
}

const [edge, centres, slide, gaps] = SNAPS;
const press = grip(free(HIT.grab));
const g2 = free(centres.ta);
const KEY = { x: 90, y: 760, w: 150, h: 140, from: 64.5, to: 64.9375 };

function ripple(t) {
  return [kit.ripple(press[0], press[1], span(t, HIT.grab, HIT.grab + 0.375), { r: 14, w: 0.7 })];
}

function edgeGuide(t) {
  const b = life(t, edge.ta, edge.tb);
  return [shown(kit.line(A.x, A.y - 24, A.x, C.y + 28, GUIDE), b)];
}

function centreGuide(t) {
  const b = life(t, centres.ta, centres.tb);
  return [shown(kit.line(B.x, B.y - B.r - 24, B.x, g2[1] + D.h / 2 + 24, GUIDE), b)];
}

function slideGuides(t) {
  const b = life(t, slide.ta, slide.tb);
  return [
    bloom(kit.line(CENTRE[0], SLIDE.y, CENTRE[0], SLIDE.y + SLIDE.h, GUIDE), b),
    bloom(quiet(kit.line(SLIDE.x, CENTRE[1], SLIDE.x + SLIDE.w, CENTRE[1], GUIDE)), b),
  ];
}

// Equal gaps: an I-beam each, drawn outward.
function spacing(t) {
  const b = life(t, gaps.ta, gaps.tb);
  const y = GAPS.y - 6;
  const tick = 14;
  const left = GAPS.x - D.w / 2;
  const right = GAPS.x + D.w / 2;
  const rim = B.x - Math.sqrt(B.r * B.r - (y - B.y) ** 2);
  const beam = (x0, x1) => drawOn([
    kit.line(x0, y - tick, x0, y + tick, GUIDE),
    quiet(kit.line(x0, y, x1, y, GUIDE)),
    quiet(kit.line(x1, y - tick, x1, y + tick, GUIDE)),
  ], b);
  return [...beam(left - 8, A.x + A.w + 8), ...beam(right + 8, rim - 8).map((st, i) => (i ? st : quiet(st)))];
}

function keyCap(t) {
  const down = t < HIT.alt ? 0 : t < HIT.keyUp ? ease.snap(span(t, HIT.alt, HIT.alt + 0.0625))
    : 1 - span(t, HIT.keyUp, HIT.keyUp + 0.0625) + spring(t - HIT.keyUp - 0.0625, -0.25, 40, 16);
  const dy = 15 * down;
  const cx = KEY.x + KEY.w / 2;
  const face = roundRect(cx, KEY.y + 61 + dy * 0.7, KEY.w - 28 + dy * 0.4, 102 - dy * 0.6, 12, { w: 1.05 });
  const word = kit.text('Alt', { x: cx, y: KEY.y + 61 + 15.7 + dy * 0.7, size: 40, align: 'center' });
  const cap = roundRect(cx, KEY.y + KEY.h / 2, KEY.w, KEY.h, 18, { w: 1.2 });
  return drawOn([cap, face, ...word], ease.inOut(span(t, KEY.from, KEY.to)));
}

// Each event: when it joins the line, and its strokes.
export function events(t) {
  return [
    { start: HIT.grab, strokes: ripple(t) },
    { start: edge.ta, strokes: edgeGuide(t) },
    { start: centres.ta, strokes: centreGuide(t) },
    { start: slide.ta, strokes: slideGuides(t) },
    { start: gaps.ta, strokes: spacing(t) },
    { start: KEY.from, strokes: keyCap(t) },
  ];
}

export const shape = (c) => roundRect(c[0], c[1], D.w, D.h, D.r, { w: 1.45 });
