// Banners leave: status bar, dialog, error page; fonts stay.
import { kit } from '../lib/kit/index.js';
import { visible } from '../lib/line.js';
import * as P from './c05-parts.js';
import * as A from './c05-acts.js';
import { coil, tipOf, unit, TOOTH } from './c05-motion.js';

const { keys, span } = kit;
const T = A.T;
const START = 38;
const END = 46;
const FPS = 60;

const TAP = 0.3125;
const TAPS = [
  [T.grab, A.C0], [T.press, null], [T.click, null], [T.hold, null], [T.reopen, A.REOPEN], [T.touch, A.CF],
];

const GRAB = [0.88, 0.85];
const riskHold = (t) => {
  const b = A.riskBox(t, P.PITCH);
  return [b.x + GRAB[0] * b.w, b.y + GRAB[1] * b.h];
};
const SAVE = (() => {
  const [x, y] = A.riskButton(A.riskBox(T.dialog + 1, P.PITCH));
  return [x + P.risk.button.w / 2, y + P.BTN / 2];
})();
const PULL_X = P.STRIP.x + A.PULL_F * P.STRIP.w;
const BLIND_TOP = P.rowTop(1);
const LETGO = [A.CF[0] - 32, A.CF[1] - 52];

function exitPoint(k) {
  const [nx, ny] = k.next ?? [2880, 540];
  const dx = nx - 960;
  const dy = ny - 540;
  const u = Math.min(1020 / Math.abs(dx || 1e-9), 600 / Math.abs(dy || 1e-9));
  return [960 + dx * u, 540 + dy * u];
}

function hand(t, k) {
  if (t <= T.grab) return keys(t, [[T.draw[1], A.WORDS_END], [T.grab, A.C0, 'inOut']]);
  if (t <= T.off) return A.pull(t);
  if (t <= T.gone) return A.thrown(t);
  if (t <= T.drag) return keys(t, [[T.gone, A.THROW], [40.875, riskHold(40.875), 'inOut']]).map((v, i) => v + (i ? 3 * Math.sin(Math.PI * span(t, T.press, T.drag)) : 0));
  if (t <= T.dialog) return riskHold(t);
  if (t <= T.close[0]) return keys(t, [[T.dialog, riskHold(T.dialog)], [42.1875, SAVE, 'inOut']]);
  if (t <= T.blind[0]) return keys(t, [[T.close[0], SAVE], [T.hold, [PULL_X, BLIND_TOP + P.STRIP.h], 'inOut']]);
  if (t <= T.blind[1]) return [PULL_X, BLIND_TOP + A.blind(t)];
  if (t <= T.roll[0]) {
    const end = [PULL_X, BLIND_TOP + A.blind(T.blind[1])];
    return keys(t, [[T.blind[1], end], [T.error, [PULL_X + 6, end[1] + 10], 'out'], [43.375, A.REOPEN, 'inOut']]);
  }
  if (t <= T.touch) return keys(t, [[T.roll[0], A.REOPEN], [T.touch, A.CF, 'inOut']]);
  if (t <= T.curl) return A.curlAt(t);
  return keys(t, [[T.curl, A.curlAt(T.curl)], [45.125, LETGO, 'out'], [END, exitPoint(k), 'in']]);
}

function taps(t) {
  return TAPS.map(([t0, at], i) => {
    const p = at ?? hand(t0 + 0.0001, { next: null });
    return { ...kit.ripple(p[0], p[1], span(t, t0, t0 + TAP), { r: i === 5 ? 9 : 12 }), link: 0 };
  });
}

function content(t) {
  const [s1, s2, s3] = A.rise(t);
  const a = A.dim(t);
  return [
    ...P.chrome,
    ...A.status(a),
    ...A.page(t, s3, A.blindEdge(t, s2), a),
    ...A.words(t, a),
    ...A.fonts(t, a),
    ...A.stopped(t, s2),
    ...A.risk(t, s1),
    ...A.tape(t),
    ...taps(t),
  ];
}

function lastVisible(list) {
  for (let i = list.length - 1; i >= 0; i--) if (visible(list[i])) return list[i];
  return null;
}

function reserve(list, t, k) {
  const P0 = tipOf(lastVisible(list.slice(0, list.length - TAPS.length)));
  const c = hand(t, k);
  const dir = t < T.draw[1] ? [1, 0] : unit(c[0] - P0[0], c[1] - P0[1]);
  const total = kit.marks(list).total;
  const prev = lastVisible(list);
  const stub = coil(P0, dir, 2 * TOOTH);
  const both = kit.marks([prev, { pts: stub, show: [0, 1] }]);
  const link = both.total - (both.list[0].end - both.list[0].start) - (both.list[1].end - both.list[1].start);
  return { P0, dir, need: total + link };
}

const reserveStroke = (pts) => ({ pts, w: 0.5, alpha: 0.002, show: [0, 1], app: null, link: 0.004 });

let TOP = null;
function top(k) {
  if (TOP === null) {
    let m = 0;
    for (let f = START * FPS; f <= END * FPS; f++) m = Math.max(m, reserve(content(f / FPS), f / FPS, k).need);
    TOP = m + 2 * TOOTH + 1;
  }
  return TOP;
}

export default {
  id: 'c05', start: START, end: END, app: 'doc',
  strokes(t, k) {
    const list = content(t);
    const r = reserve(list, t, k);
    return [...list, reserveStroke(coil(r.P0, r.dir, top(k) - r.need))];
  },
  pen(t, k) {
    if (t <= T.draw[0]) return 0;
    return keys(t, [[T.draw[0], 0], [T.draw[1], top(k), 'inOut']]);
  },
  cursor(t, k) {
    if (t < T.draw[1]) return null;
    const [x, y] = hand(t, k);
    return { x, y };
  },
  camera(t) {
    const [x, y, w] = keys(t, [[44.25, [960, 540, 1920]], [45.25, [948, 524, 1800], 'inOut']]);
    return { x, y, w, r: 0 };
  },
};
