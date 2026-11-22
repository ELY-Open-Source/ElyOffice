// Seven icons cinch into one package that opens three ways.
import { kit } from '../lib/kit/index.js';
import { SUITE, BOX, ROW, box, linux, macName, setup, row, version } from './c02-parts.js';
import { schedule } from './c02-pen.js';

const START = 8;
const END = 20;
const CINCH = 12.0;
const SWAP = 18.0;
const HOPS = [3, 2, 4, 1, 5, 0, 6];
const tickAt = (i) => 14 + 0.5 * i;
const hopAt = (i) => 12.125 + 0.0625 * HOPS.indexOf(i);

const home = (i) => [ROW.x + (i - 3) * ROW.pitch, ROW.y];
const base = SUITE.map((a, i) => kit.icon(a, ...home(i), ROW.size, { app: a, w: 1.05 }));
const wobble = (tau, amp, f = 18, d = 8) => (tau > 0 ? amp * Math.exp(-d * tau) * Math.sin(f * tau) : 0);

// Icon i at t: row, taut, cinched, hopping in.
function pose(i, t, k) {
  const off = i - 3;
  const taut = k.ease.out(k.span(t, 11.5, 11.875)) * (1 - k.span(t, 11.875, CINCH));
  const pull = k.ease.in(k.span(t, 11.875, CINCH));
  const x0 = off * (ROW.pitch + 9 * taut - 30 * pull) - wobble(t - CINCH, off * 4);
  const y0 = -5 * taut;
  const u = k.span(t, hopAt(i), hopAt(i) + 0.3125);
  const tx = BOX.dx / 2 + 6 * (HOPS.indexOf(i) - 3);
  const ty = BOX.rim + 96 - ROW.y;
  const x = k.lerp(x0, tx, u);
  const y = k.lerp(y0, ty, u * u) - 150 * u * (1 - u);
  const r = (i % 2 ? 0.55 : -0.55) * u * u;
  const bottom = ROW.y + y + ROW.size / 2;
  return { x, y, r, sink: k.clamp((bottom - (BOX.rim - 14)) / (ROW.size * 0.9)) };
}

function icons(t, k) {
  return base.flatMap((strokes, i) => {
    if (t < 11.5) return strokes;
    const p = pose(i, t, k);
    const [ox, oy] = home(i);
    return k.leave(k.move(strokes, { x: ROW.x - ox + p.x, y: p.y, r: p.r, ox, oy }), p.sink);
  });
}

function squash(t) {
  let s = 0;
  for (const i of HOPS) s += wobble(t - hopAt(i) - 0.25, 5, 26, 12);
  return s;
}

function flip(t, k) {
  const pre = 0.06 * k.ease.out(k.span(t, 17.625, 17.8125)) * (1 - k.span(t, 17.8125, 17.9375));
  if (t < 17.9375) return 1 + pre - 0.97 * k.ease.in(k.span(t, 17.8125, 17.9375));
  return 0.03 + 0.97 * k.ease.out(k.span(t, 17.9375, SWAP)) + wobble(t - SWAP, 0.09, 22, 9);
}

const rest = [macName(), linux(), setup()];

const ROWS = [0, 1, 2, 3, 4, 5, 6];

// The list first, then the pen ticks down it.
function rows(t, k) {
  const sx = flip(t, k);
  const all = ROWS.map((i) => row(i, sx));
  return [...all.flatMap((o) => [o.square, ...o.icon, ...o.name]), ...all.map((o, i) => ({ ...o.tick, link: i ? 0.35 : 0.12 }))];
}

function strokes(t, k) {
  const u = k.ease.inOut(k.span(t, 17.875, SWAP));
  return [...icons(t, k), ...box(squash(t)), ...rest.flat(), ...rows(t, k), ...version(u)];
}

// Pen windows, in stroke order.
const groups = [];
let at = 0;
const add = (n, a, b, ease) => {
  groups.push({ from: at, to: at + n - 1, a, b, ease });
  at += n;
};
base.forEach((s, i) => add(s.length, 8.75 + 0.375 * i, 9.125 + 0.375 * i, 'sine'));
add(box().length, 11.4375, 12.0625, 'linear');
add(rest.flat().length, 12.125, 13.25, 'sine');
add(ROWS.reduce((n, i) => n + 1 + row(i).icon.length + row(i).name.length, 0), 13.25, 13.875);
for (const i of ROWS) add(1, tickAt(i) - 0.125, tickAt(i), 'in');
add(version(0).length, 17.0, 17.4375);

const pen = schedule(groups, (t) => strokes(t, kit), START, END);

// Out under the version, toward the next panel.
function exit(k) {
  const y = (k.next ?? [0, 540])[1];
  return [1890, k.clamp(y, 700, 820)];
}

const last = version(1).at(-1).pts.at(-1);

export default {
  id: 'c02', start: START, end: END, app: 'suite',
  strokes,
  pen,
  cursor(t, k) {
    if (t < 19.0) return null;
    const [ex, ey] = exit(k);
    const u = k.ease.inOut(k.span(t, 19.0, END));
    return { x: k.lerp(last[0], ex, u), y: k.lerp(last[1], ey, u) + Math.sin(Math.PI * u) * 40 };
  },
  camera(t, k) {
    const near = { x: 500, y: 232, w: 1120 * (1 - 0.03 * k.span(t, 8.75, 11.875)) };
    const lean = k.ease.sine(k.span(t, 13.5, 17.5)) * (1 - k.ease.inOut(k.span(t, 18.25, 19.25)));
    const far = { x: k.lerp(960, 1010, lean), y: 540, w: k.lerp(1920, 1840, lean) };
    const out = k.ease.inOut(k.span(t, 12.0, 13.0));
    const w = Math.exp(k.lerp(Math.log(near.w), Math.log(far.w), out));
    return { x: k.lerp(near.x, far.x, out), y: k.lerp(near.y, far.y, out), w, r: 0 };
  },
};
