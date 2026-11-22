// Launch: traced, then still; a tab closes at once.
import { kit as k } from '../lib/kit/index.js';
import * as P from './c04-parts.js';
import { penTable } from './c04-pen.js';

const hide = (list) => list.map((s) => ({ ...s, show: [0, 0] }));
const backward = (list) => list.slice().reverse().map((s) => ({ ...s, pts: [...s.pts].reverse() }));
const settle = (tau, a, f, d) => (tau < 0 ? 0 : a * Math.exp(-d * tau) * Math.sin(f * tau));
const NAME = P.name();
const FRAME = P.frame();
const REDUCED = P.segment(1);
const FULL = P.segment(2);
const PICK = [REDUCED.x + REDUCED.w / 2, P.SEG.y + P.SEG.h / 2];
const NOTES = P.notesClose();
const PICKED = 32.75;

function sweep(t) {
  const v = Math.max(0, t - 31.5625) / 0.5;
  const leg = Math.floor(v);
  const f = k.ease.inOut(v - leg);
  return 0.7 * (leg % 2 ? 1 - f : f);
}

function launch(t) {
  const grow = k.ease.out(k.span(t, 31.125, 31.375));
  const dip = k.keys(t, [[31.8125, 0], [31.875, 1, 'out'], [32.0, 0]]);
  const s = k.span(t, 31.875, 32.0);
  const u = 0.55 * s + 0.45 * k.ease.out(s);
  const content = k.move(P.pageFrom(0.92 + 0.08 * grow - 0.035 * dip, u), { y: settle(t - 32.0, -7, 15, 8) });
  const start = k.leave([...NAME, ...P.track(sweep(t))], k.ease.in(k.span(t, 31.875, 31.9375)));
  const fill = t < 36.0 ? k.span(t, 31.5625, 31.875) : k.ease.sine(k.span(t, 36.0, 37.5));
  const shut = k.ease.in(k.span(t, 33.25, 33.4375));
  const ring = t < 36.0 ? k.leave([P.ring(fill)], shut) : [P.ring(fill)];
  return [...k.leave([FRAME.rect, FRAME.bar, ...FRAME.controls, ...content, ...start], shut), ...ring];
}

function still(t) {
  const on = t >= 33.5;
  const doc = t >= 34.0;
  const start = [...NAME, ...P.track(0.35)];
  const slide = t < 36.0 ? 0 : P.SHUT - k.spring(t - 36.0, P.SHUT, 12, 8);
  const show = (list, yes) => (yes ? list : hide(list));
  return [
    ...show(start, on && !doc),
    ...show(backward(P.pageFrom(1, doc ? 1 : 0)), on),
    ...show([FRAME.rect], on),
    ...show([FRAME.bar], on && !doc),
    ...show(P.tabs(slide, t >= 36.0), doc),
    ...clicks(t),
    ...show(FRAME.controls, on),
  ];
}

function engine(t) {
  const turn = k.keys(t, [[31.125, 0], [31.9375, 1.8, 'out'], [36.0, 1.8], [37.5, 1.8 + Math.PI / 2, 'sine']]);
  return P.engine(turn);
}

function settings(t) {
  const list = P.settings(k.keys(t, [[PICKED, 0], [PICKED + 0.1875, 1, 'back']]), k.span(t, PICKED, PICKED + 0.4));
  if (t < 32.0) return hide(list);
  return k.leave(list, k.ease.in(k.span(t, 33.25, 33.4375)));
}

function clicks(t) {
  return [[...P.CLOSE, 33.25], [...NOTES, 36.0]].map(([x, y, at]) => k.ripple(x, y, k.span(t, at, at + 0.4)));
}

function groups(t) {
  return [engine(t), launch(t), settings(t), still(t)];
}

const sizes = groups(30).map((g) => g.length);
const first = (g) => sizes.slice(0, g).reduce((s, x) => s + x, 0);
const A = first(1);
const B = A + 2 + FRAME.controls.length;

const scene = {
  id: 'c04', start: 30, end: 38, app: 'doc',
  strokes: (t) => groups(t).flat(),
  pen: () => 0,
  cursor(t) {
    if (t < 32.5) return null;
    const below = [FULL.x + FULL.w - 20, P.SEG.y + P.SEG.h + 46];
    const near = (p, dx, dy) => [p[0] + dx, p[1] + dy];
    const [x, y] = k.keys(t, [
      [32.5, below], [PICKED - 0.0625, near(PICK, 10, 14)], [PICKED, PICK, 'in'], [33.0, PICK],
      [33.25, P.CLOSE], [33.375, near(P.CLOSE, 5, 6), 'out'],
      [34.875, near(P.CLOSE, 5, 6)], [35.625, near(NOTES, 16, 20)], [35.875, near(NOTES, 4, 5), 'out'],
      [36.0, NOTES, 'in'], [36.375, near(NOTES, 6, 8), 'out'],
      [37.125, [1420, 560]], [38.0, [1470, 1078], 'in'],
    ]);
    return { x, y };
  },
  camera(t) {
    const rest = [960, 540, 1920];
    const near = [760, 300, 1280];
    const [x, y, w] = k.keys(t, [[34.5, rest], [35.5, near], [36.125, near], [37.0, rest]]);
    return { x, y, w, r: 0 };
  },
};

scene.pen = penTable(scene, [
  { t0: 30.75, t1: 30.9375, from: -1, to: A - 1, ease: 'linear' },
  { t0: 30.9375, t1: 31.125, from: A - 1, to: B - 1, ease: 'linear' },
  { t0: 31.125, t1: 31.375, from: B - 1, to: B + 6, ease: 'linear' },
  { t0: 31.375, t1: 31.5, from: B + 6, to: B + 11 + NAME.length, ease: 'linear' },
  { t0: 31.5, t1: 31.5625, from: B + 11 + NAME.length, to: first(2) - 1, ease: 'linear' },
  { t0: 32.0, t1: 32.5, from: first(2) - 1, to: first(3) - 1, ease: 'linear' },
]);

export default scene;
