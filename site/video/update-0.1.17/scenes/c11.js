// Final fixes: save, GPU, Linux engines, Git, mail sidebar.
import { kit } from '../lib/kit/index.js';
import { join, marks } from '../lib/line.js';
import { tether } from '../lib/tether.js';
import { FPS } from '../lib/time.js';
import * as art from './c11-art.js';
import { backOut, press, shake, route, tetherCurve, scaleAbout, hide } from './c11-motion.js';

const { span, lerp, clamp, ease } = kit;
const START = 98;
const END = 106;
const F0 = START * FPS;
const F1 = END * FPS;

// Drawing phases in frames; leads start on even frames.
const PHASES = [
  { id: 'A', s: 5925, e: 5974 },
  { id: 'B1', s: 6020, e: 6048 },
  { id: 'B2', s: 6068, e: 6090 },
  { id: 'C', s: 6112, e: 6140 },
  { id: 'D', s: 6180, e: 6224 },
  { id: 'E', s: 6240, e: 6292 },
];

const DLG = art.dialog();
const CARD = art.card();
const FILE = art.fileIcon();
const WIN = art.docWindow();
const SRC = art.sources();
const PROC = art.processes();
const MAIL = art.mailFrame();
const EXIT_LEN = 420;

const at = (list) => list[0].pts[0];
const firstOf = { B1: at(CARD), C: at(SRC.image), D: at(PROC.keep) };
const bold = (list, u, by = 0.9) => list.map((st) => ({ ...st, w: st.w * (1 + by * u) }));

function engines(t) {
  const fade = lerp(1, 0.4, ease.inOut(span(t, 102.625, 103.0)));
  const image = SRC.image.map((s) => ({ ...s, alpha: fade }));
  const lockOpen = backOut(span(t, 102.5, 102.625), 2);
  const tap = press(t, 102.5);
  const lock = bold(scaleAbout(art.lock(lockOpen), 1 - 0.08 * tap, art.LOCK_AT), tap);
  const rig = art.GEARS.flatMap(([gx, gy, app], i) => {
    const off = 102.5 + 0.0625 * i;
    const land = off + 0.375;
    const top = [gx, gy - 32];
    const [px, py] = art.SOURCES.pkg;
    const old = art.cable(top, art.SOURCES.image, 120);
    const fresh = art.cable(top, [px, py + shake(t - land, 7, 46, 10)], 70);
    old.show = [0, 1 - ease.in(span(t, off, off + 0.1875))];
    fresh.show = [0, ease.out(span(t, off + 0.125, land))];
    const spin = ease.out(span(t, land - 0.0625, land + 0.4375)) * (Math.PI / 4);
    return [art.gear(gx, gy, 26, spin, app), old, fresh];
  });
  return [...image, ...lock, ...SRC.pkg, ...rig];
}

function divAt(t, k) {
  if (t < 104.9375) return art.DIVIDER.from;
  if (t >= 105.5) return art.DIVIDER.to;
  return scene.cursor(t, k).x;
}

function mail(t, k) {
  const div = divAt(t, k);
  const held = ease.out(span(t, 104.875, 104.9375)) * (1 - ease.out(span(t, 105.5, 105.6875)));
  return [...MAIL, ...art.sidebar(div), ...art.list(div), ...bold([art.divider(div)], held, 1.2)];
}

function opening(t) {
  const o = backOut(span(t, 101.1333, 101.4333), 1.5);
  const [cx, cy] = art.WIN_AT;
  const [fx, fy] = art.FILE_AT;
  return scaleAbout(WIN, lerp(0.14, 1, o), art.WIN_AT, [lerp(fx - cx, 0, clamp(o)), lerp(fy - cy, 0, clamp(o))]);
}

const leads = new Map();
const leadW = new Map();
let index = null;

function leadStroke(id, t, k) {
  const p = PHASES.find((q) => q.id === id);
  if (Math.round(t * FPS) < p.s) return { pts: [[0, 0], [1, 0]], w: 1, alpha: 1, show: [0, 0], app: null };
  if (!leads.has(id)) leads.set(id, leadPts(p, k));
  const dry = ease.inOut(span(t, p.s / FPS, p.s / FPS + 0.5));
  return { pts: leads.get(id), w: (leadW.get(id) ?? 1) * lerp(1, 0.4, dry), alpha: lerp(1, 0.55, dry), show: [0, 1], app: null };
}

function tipOf(list) {
  const J = join(list, 'c11');
  const n = J.x.length - 1;
  const a = [J.x[n - 1], J.y[n - 1]];
  const b = [J.x[n], J.y[n]];
  const d = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
  return { at: b, tan: [(b[0] - a[0]) / d, (b[1] - a[1]) / d] };
}

function leadPts(p, k) {
  const f = p.s - 1;
  const c = scene.cursor(f / FPS, k);
  return tetherCurve(tipOf(raw(f / FPS, k)), [c.x, c.y], tether(scene, k, f));
}

function raw(t, k) {
  const f = Math.round(t * FPS);
  const gate = (id, list) => (f >= PHASES.find((q) => q.id === id).s ? list : hide(list));
  const push = press(t, 100.0);
  const save = bold(scaleAbout(DLG.save, 1 - 0.07 * push, DLG.saveAt), push);
  const click = Math.max(press(t, 100.9375, 0.0625, 0.0625), press(t, 101.0625, 0.0625, 0.0625));
  const file = bold(scaleAbout(FILE, 1 - 0.08 * click, art.FILE_AT), click, 0.7);
  const stop = bold(PROC.end.slice(2, 4), press(t, 104.0));
  const parts = [
    ['A', kit.move([...DLG.body, ...save], { x: shake(t - 99.5667, 9) })],
    ['B1', [leadStroke('B1', t, k), ...CARD, ...file]],
    ['B2', [leadStroke('B2', t, k), ...opening(t)]],
    ['C', [leadStroke('C', t, k), ...engines(t)]],
    ['D', [leadStroke('D', t, k), ...PROC.keep, ...PROC.end.slice(0, 2), ...stop, ...PROC.end.slice(4)]],
    ['E', [leadStroke('E', t, k), ...mail(t, k)]],
  ];
  const out = [];
  const idx = {};
  for (const [id, list] of parts) {
    idx[id] = [out.length, out.length + list.length];
    out.push(...gate(id, list).map((st) => ({ ...st, link: (st.link ?? 1) * 0.4 })));
  }
  index ??= idx;
  const from = idx.D[1] - PROC.end.length;
  const gone = kit.leave(out.slice(from, idx.E[0] + 1), ease.inOut(span(t, 104.0, 104.5)));
  out.splice(from, gone.length, ...gone);
  return out;
}

function target(f, m) {
  const p = PHASES.find((q) => f >= q.s && f < q.e);
  if (f < PHASES[0].s) return 0;
  if (!p) return m.total;
  const [a, b] = index[p.id];
  const base = p.id === 'A' ? 0 : m.list[a].end;
  const end = Math.max(...m.list.slice(0, b).map((q) => q.end));
  return base + ease.sine((f - p.s) / (p.e - p.s)) * (end - base);
}

let table = null;
function ensure(k) {
  if (table) return table;
  const P = new Float64Array(F1 - F0 + 1);
  let me = 0;
  let top = 0;
  for (let f = F0; f <= F1; f++) {
    const m = marks(raw(f / FPS, k));
    if (f % 2 === 0 || f === F1) me = Math.max(me, m.total);
    top = Math.max(top, Math.min(target(f, m), me));
    P[f - F0] = top;
  }
  for (const p of PHASES.slice(1)) {
    const speed = ((P[p.s + 3 - F0] - P[p.s - 4 - F0]) * FPS) / 7;
    leadW.set(p.id, 0.75 / clamp(1.35 - speed / 2600, 0.55, 1.35));
  }
  table = P;
  return P;
}

const tips = new Map();
function tipAt(f, k) {
  if (!tips.has(f)) tips.set(f, tipOf(raw(f / FPS, k)).at);
  return tips.get(f);
}

function moves(id, k) {
  const tip = (f) => tipAt(f, k);
  const io = ease.inOut;
  switch (id) {
    case 'A': return [
      { t0: 99.5667, t1: 99.75, to: [tip(5974)[0] + 28, tip(5974)[1] - 66], ease: ease.out },
      { t0: 99.75, t1: 99.9375, to: DLG.saveAt, ease: io, bow: 0.1 },
      { t0: 100.0625, t1: 100.2813, to: firstOf.B1, bow: 0.14 },
    ];
    case 'B1': {
      const [x, y] = tip(6048);
      return [
        { t0: 100.8, t1: 100.875, to: [x + 5, y - 13], ease: ease.out },
        { t0: 100.875, t1: 100.9375, to: [x, y], ease: ease.in },
        { t0: 100.9375, t1: 101.0, to: [x + 4, y - 10], ease: ease.out },
        { t0: 101.0, t1: 101.0625, to: [x, y], ease: ease.in },
      ];
    }
    case 'B2': return [{ t0: 101.5, t1: 101.8125, to: firstOf.C, bow: -0.16 }];
    case 'C': return [
      { t0: 102.3333, t1: 102.4375, to: art.LOCK_AT, ease: io },
      { t0: 102.625, t1: 102.9375, to: firstOf.D, bow: 0.1 },
    ];
    case 'D': return [{ t0: 103.7333, t1: 103.9375, to: art.STOP_AT, bow: -0.2 }];
    default: return exitMoves(k);
  }
}

function exitMoves(k) {
  const [nx, ny] = k.next ?? [3400, 540];
  const from = [art.DIVIDER.to, 905];
  const low = [art.DIVIDER.to + 260, 1018];
  const d = Math.hypot(nx - low[0], ny - low[1]);
  const out = [low[0] + ((nx - low[0]) / d) * EXIT_LEN, low[1] + ((ny - low[1]) / d) * EXIT_LEN];
  return [
    { t0: 104.8667, t1: 104.9375, to: [art.DIVIDER.from, 905], ease: ease.out },
    { t0: 104.9375, t1: 105.0, to: [art.DIVIDER.from + 12, 905], ease: ease.out },
    { t0: 105.0, t1: 105.4375, to: [art.DIVIDER.to - 9, 905], ease: ease.inOut },
    { t0: 105.4375, t1: 105.5, to: from, ease: ease.out },
    { t0: 105.5625, t1: 105.8125, to: low, ease: ease.in, bow: -0.2 },
    { t0: 105.8125, t1: 106, to: out, ease: ease.linear },
  ];
}

const VIEW = {
  A0: [470, 300, 1360], A: [452, 288, 1290],
  B: [462, 806, 1260], B1: [455, 812, 1190],
  C: [1430, 288, 1300], C1: [1425, 292, 1250],
  DE: [1420, 806, 1270], E: [1690, 800, 1120],
};

// Camera shots: t0, t1, from, to, hop, ease.
const SHOTS = [
  [98.75, 100.0625, VIEW.A0, VIEW.A, 0, 'sine'], [100.0625, 100.4375, VIEW.A, VIEW.B, 0.12],
  [100.4375, 101.5625, VIEW.B, VIEW.B1, 0, 'sine'], [101.5625, 102.0, VIEW.B1, VIEW.C, 0.14],
  [102.0, 103.0625, VIEW.C, VIEW.C1, 0, 'sine'], [103.0625, 103.5, VIEW.C1, VIEW.DE, 0.1],
  [103.5, 104.625, VIEW.DE, VIEW.DE, 0], [104.625, 105.375, VIEW.DE, VIEW.E, 0, 'sine'],
  [105.375, END, VIEW.E, VIEW.E, 0],
];

const scene = {
  id: 'c11', start: START, end: END, app: 'suite',
  strokes(t, k) {
    ensure(k);
    return raw(t, k);
  },
  pen(t, k) {
    return ensure(k)[clamp(Math.round(t * FPS), F0, F1) - F0];
  },
  cursor(t, k) {
    const f = Math.round(t * FPS);
    const drawing = PHASES.find((p) => f >= p.s && f < p.e);
    if (drawing) {
      const [x, y] = tipAt(drawing.e, k);
      return { x, y };
    }
    const i = PHASES.findIndex((p, j) => f >= p.e && f < (PHASES[j + 1]?.s ?? F1 + 1));
    if (i < 0) return null;
    const [x, y] = route(t, tipAt(PHASES[i].e, k), moves(PHASES[i].id, k));
    return { x, y };
  },
  camera(t) {
    const m = SHOTS.find(([, t1]) => t <= t1) ?? SHOTS.at(-1);
    const [t0, t1, a, b, hop, how] = m;
    const u = ease[how ?? 'inOut'](span(t, t0, t1));
    const w = Math.exp(lerp(Math.log(a[2]), Math.log(b[2]), u)) * (1 + hop * Math.sin(Math.PI * u));
    return { x: lerp(a[0], b[0], u), y: lerp(a[1], b[1], u), w, r: 0 };
  },
};

export default scene;
