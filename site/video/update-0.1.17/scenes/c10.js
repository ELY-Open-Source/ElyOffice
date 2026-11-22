// Links and safety: encoded, refused, locked, checked.
import { kit } from '../lib/kit/index.js';
import * as A from './c10-art.js';
import { route } from './c10-route.js';
import { tables, penOf, cursorOf } from './c10-time.js';

const START = 88;
const END = 98;
const TAP = 0.125;
const fast = (s) => (s.route ? 0.35 : 1);
const wobble = (tau, amp, f, d) => (tau < 0 ? 0 : amp * Math.exp(-d * tau) * Math.sin(f * tau));

const PHASES = [
  { id: 'doc', from: 'jump', keys: [[88.75, 'win'], [89.1875, 'win', 'in'], [89.4375, 'link1', 'out']] },
  { id: 'out', from: 'jump', keys: [[89.625, 'pill'], [89.75, 'pill', 'in'], [90.0, 'url2', 'out']] },
  { id: 'broken', from: 'jump', keys: [[90.3125, 'link2'], [90.6875, 'link2', 'linear']] },
  { id: 'says', from: 'jump', keys: [[91.125, 'rule'], [91.1875, 'rule', 'in'], [91.5, 'says', 'out']] },
  { id: 'mailto', from: 'jump', keys: [[91.75, 'link3'], [92.25, 'link3', 'linear']] },
  { id: 'strike', from: 'jump', keys: [[92.6875, 'strike'], [92.9375, 'strike', 'snap']] },
  { id: 'engine', from: 'jump', keys: [[93.1875, 'icon'], [93.4375, 'icon'], [93.625, 'channel', 'linear'],
    [93.75, 'gear', 'linear'], [93.875, 'lock', 'out']] },
  { id: 'versions', from: 'end', keys: [[94.0625, 'r:v1'], [94.3125, 'v2']] },
  { id: 'paste', from: 'jump', keys: [[95.375, 'snippet'], [95.75, 'snippet'], [95.9375, 'lens'], [96.25, 'picture'],
    [96.5, 'tick', 'out']] },
].map((p) => ({ weight: p.id === 'out' ? (s) => (A.isCode(s) ? 2.2 : fast(s)) : fast, ...p }));

const CLICKS = {
  click1: [89.5, [A.xAt(A.URL_IN, 37), A.ROW[0] - 10]],
  click2: [91.0, [A.xAt(A.BROKEN, 17), A.ROW[1] - 10]],
  click3: [92.5, [A.xAt(A.MAILTO, 35), A.ROW[2] - 10]],
};

const ORDER = ['win', 'link1', 'click1', 'r:pill', 'pill', 'url2', 'r:link2', 'link2', 'click2', 'r:rule', 'rule', 'says',
  'r:link3', 'link3', 'click3', 'r:strike', 'strike', 'r:icon', 'icon', 'channel', 'gear', 'r:lock', 'lock',
  'r:v1', 'v1', 'r:v2', 'v2', 'r:snippet', 'snippet', 'r:lens', 'lens', 'r:picture', 'picture', 'r:tick', 'tick'];

const statics = {
  win: A.windowArt(), link1: A.link1(), pill: A.pill(), url2: A.url2(), link2: A.link2(), rule: A.rule(), says: A.says(),
  link3: A.link3(), strike: A.strike(), icon: A.icon(), channel: A.channel(), snippet: A.snippet(),
  picture: A.picture(), tick: A.tick(), v1: A.version(A.ICON[0]), v2: A.version(A.GEAR.x),
};

const lift = (t) => kit.keys(t, [[93.875, 26], [93.9375, 32, 'out'], [94.0, 0, 'snap']]);
const swing = (t) => wobble(t - 94.0, 0.16, 9, 3.2);
const turn = (t) => kit.keys(t, [[96.5, 0], [96.8125, (Math.PI * 2) / 10, 'back']]);
const pulse = (t) => wobble(t - 95.9375, 0.14, 20, 7);
const shake = (t) => wobble(t - 93.0, 16, 48, 8);
const glide = (t, d) => kit.keys(t, [[94.3125, 0], [94.5, d, 'back'], [94.75, d], [95.0, 0, 'back']]);

function group(name, t) {
  if (name in CLICKS) {
    const [at, [x, y]] = CLICKS[name];
    return A.click(x, y, kit.span(t, at, at + TAP));
  }
  if (name === 'gear') return A.gear(turn(t));
  if (name === 'lock') return A.lock(lift(t), swing(t));
  if (name === 'lens') return A.lens(pulse(t));
  const base = statics[name];
  if (name === 'link3' || name === 'strike') return kit.move(base, { x: shake(t) });
  if (name === 'v1') return kit.move(base, { x: glide(t, A.MEET - A.ICON[0]) });
  if (name === 'v2') return kit.move(base, { x: glide(t, A.MEET - A.GEAR.x) });
  return base;
}

const OPEN = {};
for (const p of PHASES) {
  const names = p.keys.map(([, n]) => ORDER.indexOf(n));
  const lo = Math.min(...names);
  for (let i = ORDER[lo - 1]?.startsWith('r:') ? lo - 1 : lo; i <= Math.max(...names); i++) {
    if (!(ORDER[i] in OPEN)) OPEN[ORDER[i]] = p.keys[0][0];
  }
}
const OWNER = Object.fromEntries(PHASES.map((p) => [ORDER[ORDER.indexOf(p.keys[0][1]) - 1], p.id]));

const BOW = { 'r:pill': 0.3, 'r:link2': 0.02, 'r:icon': -0.3, 'r:v2': 0, 'r:snippet': -0.15 };

const visible = (s) => s.show[1] - s.show[0] > 1e-6 && s.pts.length > 1;

function build(t, curls) {
  const out = [];
  let last = null;
  ORDER.forEach((name, i) => {
    const open = name in CLICKS || t >= OPEN[name] - 1e-9;
    let g;
    if (name.startsWith('r:')) {
      const next = group(ORDER[i + 1], t)[0].pts[0];
      g = [route(last ?? next, next, { bow: BOW[name] ?? 0.12, extra: curls[OWNER[name]] ?? 0 })];
    } else g = group(name, t);
    if (!open) g = g.map((s) => ({ ...s, show: [0, 0] }));
    for (const s of g) if (visible(s)) last = s.pts.at(-1);
    out.push(...g);
  });
  return out;
}

const ends = {};
{
  let i = 0;
  for (const s of ORDER) {
    const n = s.startsWith('r:') ? 1 : group(s, START).length;
    ends[s] = [i, i + n];
    i += n;
  }
}

const v2end = (t) => group('v2', t).at(-1).pts.at(-1);
const exit = [1830, 540];
const way = [
  [89.4375, 'end:link1'], [89.46875, [CLICKS.click1[1][0] + 10, CLICKS.click1[1][1] - 22], 'out'],
  [89.5, CLICKS.click1[1], 'in'], [89.625, 'start:pill'],
  [90.0, 'end:url2'], [90.0625, [A.PILL.x + A.PILL.w + 40, A.ROW[0] + 50], 'out'],
  [90.1875, [1260, 470], 'sine'], [90.3125, 'start:link2'],
  [90.6875, 'end:link2'], [91.0, CLICKS.click2[1], 'out'], [91.125, 'start:rule'],
  [91.5, 'end:says'], [91.75, 'start:link3'],
  [92.25, 'end:link3'], [92.5, CLICKS.click3[1], 'out'], [92.5625, CLICKS.click3[1]], [92.6875, 'start:strike'],
  [92.9375, 'end:strike'], [93.1875, 'start:icon'],
  [93.875, 'end:lock'], [94.0625, 'end:lock'],
  [94.3125, v2end], [95.0, v2end, 'linear'], [95.375, 'start:snippet'],
  [96.5, 'end:tick'], [96.75, 'end:tick'], [98.0, exit, 'inOut'],
];

const get = tables({ start: START, end: END, build, ends, phases: PHASES, way });

const VIEW = [[88.75, [960, 360, 1800]], [93.0625, [960, 360, 1800]], [93.5625, [955, 570, 1900]],
  [96.75, [955, 570, 1900]], [97.75, [960, 540, 1920]]];

export default {
  id: 'c10', start: START, end: END, app: 'doc',
  strokes: (t) => build(t, get().curls),
  pen: (t) => penOf(get, t),
  cursor: (t) => (t < 88.75 ? null : cursorOf(get, t)),
  camera(t, k) {
    const [x, y, w] = k.keys(t, VIEW);
    return { x, y, w, r: 0 };
  },
};
