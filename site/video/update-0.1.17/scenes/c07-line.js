// Hidden pads pin drawn strokes; new strokes draw wet.
import { kit } from '../lib/kit/index.js';
import { cumulative, at, slice, densify, startTangent } from '../lib/geom.js';
import { stage } from './c07-stage.js';
import { events, shape } from './c07-events.js';
import { centre, grip, EXIT_DIR, MINI, HIT } from './c07-motion.js';

const FPS = 60;
const F0 = 58 * FPS;
const F1 = 68 * FPS;
const LEAD = 24;
const COIL = 3;
const SIDES = 12;
const HIDE = { alpha: 0.002, w: 0.05 };
const NONE = { show: [0, 0] };

// A pad ends at p heading t; spare length coils.
function pad(p, t, len) {
  const a = [p[0] - 12 * t[0], p[1] - 12 * t[1]];
  const pts = [[p[0] - LEAD * t[0], p[1] - LEAD * t[1]], a];
  const coil = len - LEAD;
  if (coil > 1e-6) {
    const turns = Math.max(1, Math.ceil(coil / (2 * Math.PI * COIL)));
    const rho = coil / (turns * SIDES * 2 * Math.sin(Math.PI / SIDES));
    const n = [-t[1], t[0]];
    const c = [a[0] + rho * n[0], a[1] + rho * n[1]];
    for (let i = 1; i <= turns * SIDES; i++) {
      const th = (2 * Math.PI * i) / SIDES;
      pts.push([c[0] + rho * (-n[0] * Math.cos(th) + t[0] * Math.sin(th)), c[1] + rho * (-n[1] * Math.cos(th) + t[1] * Math.sin(th))]);
    }
  }
  pts.push(p);
  return { ...kit.stroke(pts, HIDE), link: 0 };
}

const on = (s) => s.show[1] - s.show[0] > 1e-6 && kit.length(s) > 0.5;

function head(strokes) {
  const s = strokes.find(on) ?? strokes[0];
  const part = densify(slice(s.pts, s.show[0], Math.max(s.show[1], s.show[0] + 1e-3)), 10);
  return { p: part[0], t: startTangent(part) };
}

function tail(s) {
  const c = cumulative(s.pts);
  return at(s.pts, c, s.show[1] * c[c.length - 1]);
}

function mini(t) {
  const u = kit.ease.out(kit.span(t, HIT.keyUp, HIT.keyUp + 0.1875));
  return MINI.from.map((v, i) => v + (MINI.to[i] - v) * u);
}

// The scene's strokes with every pad at its entry length.
function raw(t) {
  const c = centre(t);
  const body = shape(c);
  const list = [...stage(mini(t))];
  const pads = [];
  const put = (p, dir, live, link = 0) => {
    pads.push({ i: list.length, p, dir, live, link });
    list.push({ ...pad(p, dir, LEAD), link, ...(live ? {} : NONE) });
  };
  put(body.pts[0], [-1, 0], true, 0.4);
  list.push(body);
  for (const e of events(t)) {
    const h = head(e.strokes);
    put(h.p, h.t, t >= e.start - 1e-9);
    list.push(...e.strokes);
  }
  const last = list.filter(on).at(-1);
  const g = grip(c);
  const d = EXIT_DIR;
  list.push({ ...kit.stroke([tail(last), [g[0] - LEAD * d[0], g[1] - LEAD * d[1]]], HIDE), link: 0 });
  put(g, d, true);
  return { list, pads };
}

function measure(t) {
  const r = raw(t);
  const m = kit.marks(r.list);
  return { ...r, s: r.pads.map((q) => m.list[q.i].start) };
}

let plan = null;

// Pins pads past the line; the last never shrinks.
function prepare() {
  if (plan) return plan;
  const frames = [];
  for (let f = F0; f <= F1; f++) {
    const r = measure(f / FPS);
    frames.push({ s: r.s, live: r.pads.map((q) => q.live) });
  }
  const last = frames[0].s.length - 1;
  const pins = [];
  const shift = (fr, j) => (j < 0 ? 0 : pins[j] - fr.s[j] - LEAD);
  const before = (fr, j) => shift(fr, fr.live.slice(0, j).lastIndexOf(true));
  const end = (fr) => fr.s[last] + before(fr, last) + LEAD;
  pins[0] = Math.max(...frames.map((fr) => fr.s[0])) + LEAD + 0.5;
  for (let j = 1; j < last; j++) {
    let need = 0;
    for (const fr of frames) need = Math.max(need, fr.live[j] ? fr.s[j] + before(fr, j) + LEAD : end(fr));
    pins[j] = need + 0.5;
  }
  let top = 0;
  const reach = frames.map((fr) => (top = Math.max(top, end(fr))));
  plan = { pins, reach, frames };
  return plan;
}

const index = (t) => Math.min(F1, Math.max(F0, Math.round(t * FPS))) - F0;

export function build(t) {
  const { pins, reach } = prepare();
  const r = measure(t);
  let before = 0;
  r.pads.forEach((q, j) => {
    if (!q.live) return;
    const pin = j === r.pads.length - 1 ? reach[index(t)] : pins[j];
    const len = pin - r.s[j] - before;
    if (len < LEAD - 1e-6) throw new Error(`c07: pad ${j} needs ${len.toFixed(1)} at ${t}`);
    r.list[q.i] = { ...pad(q.p, q.dir, len), link: q.link };
    before = pin - r.s[j] - LEAD;
  });
  return r.list;
}

// Drawn length: the hand first, then whatever the line holds.
export function penAt(t) {
  if (t <= 58.75) return 0;
  const { reach, frames } = prepare();
  const hold = reach[index(t)];
  const d = frames[Math.round(59.4 * FPS) - F0];
  const doneD = plan.pins[0] + kit.length(shape(centre(59.4)));
  const grabbed = reach[index(HIT.grab)];
  const hand = kit.keys(t, [
    [58.75, 0],
    [59.2083, d.s[0], 'sine'],
    [59.25, plan.pins[0], 'linear'],
    [59.4583, doneD, 'linear'],
    [HIT.grab, grabbed, 'linear'],
    [68, grabbed + 1e6, 'linear'],
  ]);
  return Math.min(hand, hold);
}
