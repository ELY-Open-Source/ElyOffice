// c08 timeline: beats, slides, keys, pointer path, camera.
import { kit } from '../lib/kit/index.js';
import { SLIDE, KEY_H } from './c08-ui.js';

export const T = {
  insert: 69.4375, lit: 69.5625, glyph: 69.625, drag: [69.6875, 69.875], select: 70.0,
  swap: 70.625, script: 76.3125, text: 72.625, type: 72.6875, undo: 73.5, redo: 74.0, retype: 74.25, links: [75.0, 75.5],
};

// Pen-drawn stretches; everything else appears at once.
export const DRAW = [[68.75, 69.3125], [70.125, 70.375]];

export const SHOW = [[70.625, 72.25], [74.25, 75.875], [76.625, 78]];

const SLIDES = [[0, 1, 0], [71.0, 2, 1], [71.5, 1, -1], [72.0, 2, 1], [75.0, 3, 1], [75.5, 1, 1]];
const PUSH = 0.3125;
const SHIFT = 1120;

// Horizontal offset per visible slide; hidden slides are absent.
export function slides(t, te) {
  let i = 0;
  while (i + 1 < SLIDES.length && te >= SLIDES[i + 1][0] - 1e-9) i++;
  const [at, cur, dir] = SLIDES[i];
  const out = { [cur]: 0 };
  if (dir === 0 || i === 0) return out;
  const u = kit.ease.snap(kit.span(t, at, at + PUSH));
  if (u >= 1) return out;
  out[SLIDES[i - 1][1]] = -dir * SHIFT * u;
  out[cur] = dir * SHIFT * (1 - u);
  return out;
}

const RX = SLIDE.x + 1048;
export const KEY_Y = SLIDE.y + 520;
const W = { F5: 76, Esc: 76, Space: 200, '←': 64, '→': 64, Ctrl: 92, Shift: 110, Z: 64 };
const right = (...ws) => RX - ws.reduce((a, b) => a + b + 12, -12);

// Keys: when shown, where, tapped or held.
export const KEYS = [
  { label: 'F5', taps: [70.625, 74.25, 76.625], held: false,
    shows: [[70.4375, 70.6875], [74.125, 74.3125], [76.5, 76.6875]], x: () => right(W.F5) },
  { label: 'Esc', taps: [72.25, 75.875], held: false, shows: [[72.0625, 72.3125], [75.625, 75.9375]],
    x: (t) => (t < 74 ? right(W.Esc, W.Space, W['←'], W['→']) : right(W.Esc)) },
  { label: 'Space', taps: [72.0], held: false, shows: [[70.8125, 72.3125]], x: () => right(W.Space, W['←'], W['→']) },
  { label: '←', taps: [71.5], held: false, shows: [[70.84375, 72.3125]], x: () => right(W['←'], W['→']) },
  { label: '→', taps: [71.0], held: false, shows: [[70.875, 72.3125]], x: () => right(W['→']) },
  { label: 'Ctrl', taps: [], held: true, shows: [[73.3125, 74.125]],
    x: (t) => kit.keys(t, [[73.75, right(W.Ctrl, W.Z)], [73.875, right(W.Ctrl, W.Shift, W.Z), 'back']]) },
  { label: 'Shift', taps: [], held: true, shows: [[73.75, 74.3125]],
    x: (t) => kit.keys(t, [[74.125, right(W.Shift, W.Z)], [74.25, right(W.Shift, W.F5), 'back']]) },
  { label: 'Z', taps: [T.undo, T.redo], held: false, shows: [[73.3125, 74.125]], x: () => right(W.Z) },
].map((k) => ({ ...k, w: W[k.label] }));

export const LEAVE = 0.125;

export function pop(te, a) {
  return kit.keys(te - a, [[0, 0.55], [1 / 15, 1.1, 'out'], [2 / 15, 1, 'inOut']]);
}

export function press(key, t) {
  if (key.held) return 1;
  let p = 0;
  for (const T0 of key.taps) {
    const d = t - T0;
    if (d < -1 / 32 || d > 0.5) continue;
    if (d < 0) p = Math.max(p, (d + 1 / 32) * 32);
    else if (d < 1 / 16) p = 1;
    else p = Math.max(p, Math.exp(-(d - 1 / 16) * 22) * Math.cos((d - 1 / 16) * 24));
  }
  return p;
}

export function face(key, t) {
  return [key.x(t) + key.w / 2, KEY_Y + 4 + 6 + (KEY_H - 18) / 2];
}

// Pointer: stretches of keyframes, null between them.
export function pointer(spots) {
  const { canvasEnd, insert, glyph, dragFrom, dragTo, select, thumbsEnd, textClick, typedEnd, linkA, linkB, shape, bracesEnd, exit } = spots;
  const key = (label) => KEYS.find((k) => k.label === label);
  const out = [];
  const tap = (T0, label) => {
    const [x, y] = face(key(label), T0);
    out.push([T0 - 0.0625, [x, y - 34], 'inOut'], [T0, [x, y], 'in'], [T0 + 0.09375, [x, y - 26], 'out']);
  };
  const runs = [];
  const run = (a, b, build) => {
    out.length = 0;
    build();
    runs.push({ a, b, keys: out.slice() });
  };
  run(DRAW[0][1], DRAW[1][0], () => {
    out.push([DRAW[0][1], canvasEnd], [T.insert, insert, 'inOut'], [T.lit, glyph, 'inOut'], [T.glyph, glyph]);
    const back = [dragFrom[0] - 12, dragFrom[1] - 9];
    out.push([T.drag[0] - 0.03125, back, 'inOut'], [T.drag[0], dragFrom, 'in'], [T.drag[1], dragTo, 'out']);
    out.push([T.drag[1] + 0.03125, dragTo], [T.select - 0.0625, select, 'inOut'], [T.select, select]);
  });
  run(DRAW[1][1], T.type, () => {
    out.push([DRAW[1][1], thumbsEnd]);
    tap(70.625, 'F5');
    tap(71.0, '→');
    tap(71.5, '←');
    tap(72.0, 'Space');
    tap(72.25, 'Esc');
    out.push([T.text - 0.0625, textClick, 'inOut'], [T.text, textClick]);
  });
  run(T.type + 0.625, 78.01, () => {
    out.push([T.type + 0.625, typedEnd]);
    tap(T.undo, 'Z');
    tap(T.redo, 'Z');
    tap(74.25, 'F5');
    out.push([T.links[0] - 0.125, linkA, 'inOut'], [T.links[0], linkA]);
    out.push([T.links[1] - 0.0625, linkB, 'inOut'], [T.links[1], linkB]);
    tap(75.875, 'Esc');
    out.push([T.script - 0.0625, shape, 'inOut'], [T.script, shape]);
    tap(76.625, 'F5');
    out.push([77.0, bracesEnd, 'inOut'], [77.375, bracesEnd], [78, exit, 'in']);
  });
  return (t) => {
    const r = runs.find((s) => t >= s.a && t < s.b);
    if (!r) return null;
    const [x, y] = kit.keys(t, r.keys);
    return { x, y };
  };
}

const EDIT = [960, 540, Math.log(1640)];
const FULL = [SLIDE.x + SLIDE.w / 2, SLIDE.y + SLIDE.h / 2, Math.log(1056)];
const NEAR = [FULL[0], FULL[1], Math.log(1024)];

export function camera(t) {
  const frames = [[68.75, EDIT]];
  for (const [a, b] of SHOW) {
    frames.push([a, EDIT], [a + 0.375, FULL]);
    if (b < 78) frames.push([b, FULL], [b + 0.375, EDIT]);
  }
  frames.push([78, NEAR, 'sine']);
  const [x, y, lw] = kit.keys(t, frames);
  return { x, y, w: Math.exp(lw), r: 0 };
}
