// Resident engines: sign in, idle engines, an instant open.
import { kit } from '../lib/kit/index.js';
import { FPS } from '../lib/time.js';
import { build, IDX, T, even, COILS, ARROW, TIP, FILE_AT, CORNER, GAUGE, R, LABEL_END, arrowTip } from './c03-build.js';
import { measure, slack } from './c03-slack.js';
import { TAU, wobble } from './c03-shapes.js';

const S = 20;
const E = 30;
const { keys, span, ease, lerp } = kit;

build(S);

// Each slack pins the stroke after it, once drawn.
const SLACKS = [
  { at: IDX.slackV[0], next: IDX.field[0], from: 21.5 },
  { at: IDX.slackF[0], next: IDX.dots[0], from: even(21.9375) },
  { at: IDX.slackA[0], next: IDX.table[0], from: T.slack },
  { at: IDX.slackB[0], next: IDX.comb[0], from: even(24.625) },
  { at: IDX.slackW[0], next: IDX.gauge[0], from: T.dbl },
];
const anchors = [];

// Lengths without this slack: up to it, and to next.
function need(list, i) {
  const { at, next } = SLACKS[i];
  const m = measure(list, at, next);
  const marks = kit.marks(list).list;
  const shown = marks[next].end - marks[next].start > 0.5;
  return { ...m, shown, pre: marks[at].start + m.c0, post: m.c0 + (shown ? marks[next].start : marks[at].start + m.conn) };
}

// Hidden next: path length holds; shown: the next stroke holds.
function settle(t, upto) {
  const list = build(t);
  const made = [];
  for (let i = 0; i < upto && t >= SLACKS[i].from; i++) {
    const m = need(list, i);
    const a = anchor(i);
    made.push({ m, total: m.shown ? a.post - m.post : a.pre - m.pre });
    list[SLACKS[i].at] = slack(m, made[i].total);
  }
  return { list, made };
}

function anchor(i) {
  if (anchors[i] === undefined) {
    let pre = 0;
    let post = 0;
    let conn = 0;
    for (let f = Math.ceil(SLACKS[i].from * FPS - 1e-9); f <= E * FPS; f++) {
      const m = need(settle(f / FPS, i).list, i);
      if (m.shown) post = Math.max(post, m.post);
      else [pre, conn] = [Math.max(pre, m.pre), Math.max(conn, m.conn)];
    }
    anchors[i] = { pre: pre + 30, post: Math.max(post, pre + 30 + conn) + 30 };
  }
  return anchors[i];
}

// Short tails bend the exit tangent; correct twice more.
function strokes(t) {
  const { list, made } = settle(t, SLACKS.length);
  for (let pass = 0; pass < 2; pass++) {
    const m = kit.marks(list).list;
    let carried = 0;
    made.forEach((s, i) => {
      const { at, next } = SLACKS[i];
      if (!s.m.shown) return;
      const local = m[next].start - anchor(i).post - carried;
      carried += local;
      s.total -= local;
      list[at] = slack(s.m, s.total);
    });
  }
  return list;
}

const one = (name, k = 0) => [IDX[name][0] + k, IDX[name][0] + k];
const all = (name) => IDX[name];
const ITEMS = [
  [one('avatar', 0), 20.75, 21.1875, 'sine'],
  [one('avatar', 1), 21.1875, 21.3125, 'sine'],
  [one('avatar', 2), 21.3125, 21.4375, 'sine'],
  [all('slackV'), 21.5, 21.5],
  [all('field'), 21.5625, 21.875, 'linear'],
  [all('slackF'), even(21.9375), even(21.9375)],
  ...[0, 1, 2, 3, 4, 5].map((i) => [one('dots', i), 22.0625 + 0.125 * i, 22.09375 + 0.125 * i, 'out']),
  [all('arrow'), T.arrow, 22.8125, 'inOut'],
  [all('slackA'), T.slack, T.slack],
  [all('table'), 23.5, 23.75, 'inOut'],
  [all('show'), 23.75, 24.0, 'inOut'],
  ...COILS.map(([a, b], k) => [[IDX.engines[0] + 2 * k, IDX.engines[0] + 2 * k + 1], a, b, 'sine']),
  [all('slackB'), even(24.625), even(24.625)],
  [all('comb'), 24.6875, 25.0, 'linear'],
  [all('zero'), 25.0, 25.1875, 'inOut'],
  [all('file'), 25.25, 25.5, 'inOut'],
  [all('label'), 25.5, 25.75, 'inOut'],
  [[IDX.click1[0], IDX.slackW[0]], T.dbl, T.dbl],
  [[IDX.gauge[0], IDX.gauge[0]], T.gauge, 27.5, 'inOut'],
  [[IDX.gauge[0] + 1, IDX.gauge[1] - 1], 27.5, 27.75, 'inOut'],
  [[IDX.gauge[1], IDX.gauge[1]], 27.75, T.full, 'in'],
];

function target(t, list) {
  let item = null;
  for (const it of ITEMS) if (t >= it[1]) item = it;
  if (!item) return 0;
  const [[i0, i1], a, b, how] = item;
  const m = kit.marks(list).list;
  const u = b > a ? ease[how](span(t, a, b)) : 1;
  return lerp(i0 > 0 ? m[i0 - 1].end : 0, m[i1].end, u);
}

// The gate measures even frames; never pass their longest path.
let table = null;
function pen(t) {
  if (!table) {
    table = new Float64Array((E - S) * FPS + 1);
    let top = 0;
    let longest = 0;
    for (let f = 0; f < table.length; f++) {
      const tf = S + f / FPS;
      const list = strokes(tf);
      if ((S * FPS + f) % 2 === 0) longest = Math.max(longest, kit.marks(list).total);
      top = Math.max(top, target(tf, list));
      table[f] = Math.min(top, longest + 0.5);
    }
  }
  return table[Math.max(0, Math.min(table.length - 1, Math.round((t - S) * FPS)))];
}

const add = (p, d) => [p[0] + d[0], p[1] + d[1]];
const tap = (t, at) => [0, 3 * Math.max(0, Math.sin(Math.PI * span(t, at - 0.03125, at + 0.0625)))];
const USED_END = [GAUGE[0] + R * Math.cos(-Math.PI / 2 + TAU * 0.88), GAUGE[1] + R * Math.sin(-Math.PI / 2 + TAU * 0.88)];
const EXIT = [1890, 420];
const point = ([x, y]) => ({ x, y });

function cursor(t) {
  if (t >= T.slack && t < T.fly) {
    const p = t < T.click ? keys(t, [[T.slack, TIP], [22.984375, ARROW, 'out']])
      : keys(t, [[T.click, ARROW], [T.fly, arrowTip(t)]]);
    return point(add(p, tap(t, T.click)));
  }
  if (t >= 25.75 && t < T.gauge) {
    const p = keys(t, [[25.75, LABEL_END], [25.8125, add(LABEL_END, [10, 8]), 'out'], [25.9375, FILE_AT, 'back'],
      [26.1875, FILE_AT], [26.875, CORNER]]);
    return point(add(add(p, tap(t, T.dbl)), tap(t, 26.125)));
  }
  if (t >= T.full) return point(keys(t, [[29.0, USED_END], [29.25, add(USED_END, [-12, 10]), 'out'], [E, EXIT, 'in']]));
  return null;
}

function camera(t) {
  const [x, y, w] = keys(t, [[T.click, [960, 480, 1600]], [23.5, [960, 540, 1920]]]);
  return { x, y, w: w * (1 - wobble(t - T.open, 0.012, 12, 6)) };
}

export default {
  id: 'c03', start: S, end: E, app: 'suite',
  strokes: (t) => strokes(t),
  pen: (t) => pen(t),
  cursor: (t) => cursor(t),
  camera: (t) => camera(t),
};
