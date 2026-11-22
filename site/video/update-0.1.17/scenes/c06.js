// c06: four small features, one quarter of the panel each.
import { kit } from '../lib/kit/index.js';
import { hide, nearest, pad } from './c06-util.js';
import { table } from './c06-table.js';
import { pdf } from './c06-pdf.js';
import { settings } from './c06-settings.js';
import { search } from './c06-search.js';

const START = 46;
const END = 58;
const FPS = 60;
const RIPPLE = 0.375;
// Each lead retraces drawn ink toward the next part.
const PARTS = [
  { ...table, app: 'table' },
  { ...pdf, app: 'pdf', lead: [[1180, 214], [1580, 214], [1580, 500], [1660, 500]] },
  { ...settings, app: 'mark', lead: [[1840, 960], [1840, 996], [1826, 1010], [1254, 1010]] },
  { ...search, app: 'mark', lead: [[880, 628], [80, 628], [80, 594]] },
];
const WHOLE = { x: 960, y: 540, w: 1920 };

const LINK = { into: 0.6, within: 0.18 };

function piece(p, t) {
  const rings = p.clicks.map(([at, x, y]) => kit.ripple(x, y, (t - at) / RIPPLE, { r: 12, w: 0.7 }));
  const lead = p.lead ? [kit.stroke(p.lead, { w: 0.45 })] : [];
  const list = [...lead, ...p.strokes(t), ...rings].map((s, i) => ({ ...s, app: p.app, link: i ? Math.min(s.link ?? 1, LINK.within) : LINK.into }));
  if (p.lead) list[1] = nearest(list[1], p.lead[p.lead.length - 1]);
  return list.flatMap((s) => (s.g === undefined ? [s] : [pad(s, 0.45 * LINK.within), s]));
}

const blank = PARTS.map((p) => piece(p, p.t1).map(hide));
const first = blank.map((_, i) => blank.slice(0, i).reduce((n, b) => n + b.length, 0));

// Geometry steps on even frames, where the gate measures it.
function strokes(time) {
  const t = Math.floor(time * (FPS / 2) + 1e-6) / (FPS / 2);
  return PARTS.flatMap((p, i) => (i === 0 || t >= p.t0 ? piece(p, t) : blank[i]));
}

function goal(t) {
  if (t < PARTS[0].t0) return 0;
  const m = kit.marks(strokes(t));
  const i = PARTS.findIndex((p) => t >= p.t0 && t < p.t1);
  if (i < 0) return m.total;
  const base = first[i] ? m.list[first[i] - 1].end : 0;
  return base + kit.ease.inOut(kit.span(t, PARTS[i].t0, PARTS[i].t1)) * (m.total - base);
}

let pens = null;
function penTable() {
  if (!pens) {
    pens = new Float64Array((END - START) * FPS + 1);
    let top = 0;
    for (let f = 0; f < pens.length; f++) pens[f] = top = Math.max(top, goal(START + f / FPS));
  }
  return pens;
}

function end(t) {
  const list = strokes(t).filter((s) => s.show[1] - s.show[0] > 1e-6 && kit.length(s) * (s.show[1] - s.show[0]) > 0.5);
  const s = list[list.length - 1];
  const target = kit.length(s) * s.show[1];
  let run = 0;
  for (let i = 1; i < s.pts.length; i++) {
    const d = Math.hypot(s.pts[i][0] - s.pts[i - 1][0], s.pts[i][1] - s.pts[i - 1][1]);
    if (run + d >= target) {
      const u = d > 0 ? (target - run) / d : 0;
      return [s.pts[i - 1][0] + (s.pts[i][0] - s.pts[i - 1][0]) * u, s.pts[i - 1][1] + (s.pts[i][1] - s.pts[i - 1][1]) * u];
    }
    run += d;
  }
  return s.pts[s.pts.length - 1];
}

const tips = PARTS.map((p, i) => [end(p.t1), i + 1 < PARTS.length ? end(PARTS[i + 1].t0 - 1 / FPS) : null]);

function exit(next) {
  const [dx, dy] = next ? [next[0] - 960, next[1] - 540] : [-1, 0];
  const u = Math.min(1040 / Math.abs(dx || 1e-9), 600 / Math.abs(dy || 1e-9));
  return [960 + dx * u, 540 + dy * u];
}

// Camera: slow push-in on each part, a small hop between.
const PUSH = 0.94;
const HOP = 0.12;
const STOPS = PARTS.flatMap((p, i) => {
  const arrive = i ? p.t0 + 0.5 : 46.75;
  const leave = i + 1 < PARTS.length ? PARTS[i + 1].t0 : 56.75;
  return [[arrive, [p.cam.x, p.cam.y, p.cam.w], 'inOut'], [leave, [p.cam.x, p.cam.y, p.cam.w * PUSH], 'sine']];
}).concat([[57.25, [WHOLE.x, WHOLE.y, WHOLE.w], 'inOut']]);

function camera(t) {
  const [x, y, w] = kit.keys(t, STOPS);
  const j = STOPS.findIndex(([at]) => at > t);
  const travel = j > 0 && j % 2 === 0 && j < STOPS.length - 1;
  const hop = travel ? HOP * Math.sin(Math.PI * kit.span(t, STOPS[j - 1][0], STOPS[j][0])) : 0;
  return { x, y, w: w * (1 + hop) };
}

export default {
  id: 'c06', start: START, end: END, app: 'suite',
  strokes: (t) => strokes(t),
  pen: (t) => penTable()[Math.min(penTable().length - 1, Math.max(0, Math.round((t - START) * FPS)))],
  cursor(t, k) {
    const i = PARTS.findLastIndex((p) => t >= p.t1);
    if (i < 0) return null;
    // While the next part draws, wait where it ends.
    if (i + 1 < PARTS.length && t >= PARTS[i + 1].t0) return { x: tips[i + 1][0][0], y: tips[i + 1][0][1] };
    const [tip, to] = tips[i];
    const at = PARTS[i].cursor(t, tip, to ?? exit(k.next));
    return at ? { x: at[0], y: at[1] } : null;
  },
  camera,
};
