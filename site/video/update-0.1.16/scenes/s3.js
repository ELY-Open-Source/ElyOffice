// s3: the gap snaps shut; tofu burns into 思源宋体.
import { keys, span, clamp, smooth, spring } from '../lib/time.js';
import { REST, rested } from '../lib/flame.js';
import { hop, first } from '../lib/act.js';
import { burst } from '../lib/embers.js';
import { BASE, STAMPS, SNAP, gapX, seamX, lineOne } from './s3-gap.js';
import { CATCH, MARK, LEAP, travel, plan, lineTwo, fxTwo, lightsTwo } from './s3-tofu.js';

const START = 20;
const END = 32;
const STAND = [470, 800];
const SHOW = [240, 123, 42];
const SHEET = [[252, 152], [1668, 146], [1672, 934], [248, 938]];
const IN_GAP = { t0: 21.5, t1: 22.0, height: 110, pre: 0.17, post: 0 };
const POP = { t0: SNAP, t1: 24.5, height: 380, pre: 0, post: 0.45 };
const BOUNCE = { t0: 28.2, t1: 28.4, from: STAND, to: STAND, height: 24, pre: 0.12, post: 0.3 };
const HOME = { t0: 29.75, t1: 30.0, from: STAND, to: [REST.x, REST.y], height: 120, pre: 0.17, post: 0.45 };

const onTwos = (t) => Math.floor(t * 12 + 1e-6) / 12;

function stamp(t, ts) {
  const d = t - ts;
  if (d < -0.12) return { sq: 0, y: 0 };
  if (d < 0) {
    const u = smooth(span(d, -0.12, 0));
    return { sq: 0.14 * u, y: -14 * u };
  }
  return { sq: -spring(d, 0.36, 16, 8), y: 0 };
}

function wedged(t) {
  const land = hop(t, { ...IN_GAP, from: [REST.x, REST.y], to: [gapX(IN_GAP.t1), BASE] });
  if (t < IN_GAP.t1) return land;
  let sq = keys(t, [[22.0, 1.04], [23.6, 1.34]]) - spring(t - 22.0, 0.38, 15, 7);
  let y = 0;
  let lean = spring(t - 22.0, 0.16, 9, 6);
  STAMPS.forEach((ts, i) => {
    const s = stamp(t, ts);
    sq += s.sq;
    y += s.y;
    lean += (i % 2 ? -0.13 : 0.13) * Math.exp(-6 * Math.max(0, t - ts)) * (t > ts - 0.12 ? 1 : 0);
  });
  return {
    x: gapX(onTwos(t)), y: BASE + y, squash: sq, lean,
    h: keys(t, [[22.0, 140], [22.3, 118]]),
    ember: keys(t, [[23.62, 0], [23.9, 0.86, 'out']]),
    energy: keys(t, [[22.0, 0.85], [23.62, 0.85], [23.9, 0.3]]),
  };
}

function breathing(t) {
  let e = 0;
  let i = 0;
  CATCH.forEach((tc, k) => {
    const t0 = tc - travel(k) - 0.05;
    i += smooth(span(t, t0 - 0.2, t0 - 0.03)) * (1 - smooth(span(t, t0 - 0.03, t0 + 0.03)));
    e += smooth(span(t, t0 - 0.02, t0 + 0.05)) * (1 - smooth(span(t, tc + 0.02, tc + 0.2)));
  });
  return { e: clamp(e), i: clamp(i) };
}

function standing(t) {
  const s = first(hop(t, { ...POP, from: [seamX(), BASE], to: STAND }), hop(t, BOUNCE)) ?? { x: STAND[0], y: STAND[1], squash: 1, lean: 0 };
  const { e, i } = breathing(t);
  const think = keys(t, [[24.95, 0], [25.15, 0.3], [25.42, 0.3], [25.58, 0, 'snap']]);
  const look = keys(t, [[24.6, 0], [25.0, 0.18], [25.6, 0.1], [27.6, 0.12], [27.8, 0.3], [28.1, 0.06], [29.4, 0]]);
  return {
    x: s.x - 6 * i + 16 * e,
    y: s.y,
    squash: s.squash - 0.18 * i + 0.22 * e,
    lean: s.lean + look + 0.38 * e - 0.06 * i,
    ember: Math.max(think, 0.12 * i, keys(t, [[SNAP, 0.86], [SNAP + 0.05, 0, 'out']])),
    roar: Math.max(0.75 * e, keys(t, [[SNAP, 0.9], [24.3, 0.35], [24.6, 0.1], [24.8, 0]]),
      keys(t, [[27.98, 0], [28.04, 0.6], [28.7, 0]])),
    energy: 0.7 + 0.3 * e,
  };
}

function flame(t) {
  if (t < 20.95) return {};
  if (t < IN_GAP.t0 - IN_GAP.pre) {
    return {
      curl: keys(t, [[20.95, 0], [21.1, 0.65, 'back'], [21.2, 0.65], [21.3, 0, 'snap']]),
      lean: keys(t, [[20.95, 0], [21.08, 0.22], [21.3, 0]]),
    };
  }
  const pool = keys(t, [[21.5, REST.pool], [22.0, 570], [24.3, 570], [24.6, 640], [29.6, 640], [30.4, REST.pool]]);
  if (t < SNAP) return { ...wedged(t), pool };
  if (t < HOME.t0 - HOME.pre) return { ...standing(t), pool };
  return { ...(hop(t, HOME) ?? {}), pool, energy: keys(t, [[29.6, 0.7], [30.4, REST.energy]]) };
}

function sheet(ink, t) {
  const u = span(t, START, START + 0.35);
  if (u <= 0) return;
  ink.lift(5, [960, 540], (k) => {
    k.wash([SHEET], { color: SHOW, alpha: 0.09 * u, bleed: 6 });
    for (let i = 0; i < 4; i++) {
      const a = SHEET[i];
      const b = SHEET[(i + 1) % 4];
      k.line([a, b], { width: i === 2 ? 3.2 : 2.4, upto: clamp(u * 1.6 - i * 0.18), overshoot: 12, alpha: 0.85 });
    }
  });
}

function draw(ctx, t, ink) {
  sheet(ink, t);
  lineOne(ink, t);
  lineTwo(ink, t);
}

function tipOf(t) {
  const s = rested(flame(t));
  const hh = s.h * (1 + 0.6 * s.roar) * (1 - 0.86 * s.ember) * s.squash;
  return [s.x + Math.sin(s.lean) * hh * 0.7, s.y - Math.cos(s.lean) * hh * 0.7];
}

function fx(ctx, t) {
  const x = seamX();
  burst(ctx, t, { t0: SNAP, x, y: BASE - 60, count: 34, speed: 520, life: 0.6, spread: 0.8, angle: -Math.PI / 2, gravity: 420, seed: 31, size: 2.1 });
  STAMPS.forEach((ts, i) => burst(ctx, t, { t0: ts, x: gapX(ts), y: BASE, count: 8, speed: 200, life: 0.35, spread: 1.2, angle: -Math.PI / 2, gravity: 500, seed: 40 + i, size: 1.6 }));
  burst(ctx, t, { t0: 28.04, x: STAND[0], y: STAND[1] - 120, count: 22, speed: 260, life: 0.8, gravity: -60, seed: 50, size: 1.8 });
  fxTwo(ctx, t, t > 25.6 && t < 27.8 ? tipOf(t) : null);
}

function lights(t) {
  const out = lightsTwo(t);
  const flash = t >= SNAP ? Math.exp(-(t - SNAP) * 12) : 0;
  if (flash > 0.01) out.push({ x: seamX(), y: BASE - 50, r: 380, i: 0.5 * flash });
  for (const [k, tc] of CATCH.entries()) {
    const u = span(t, tc - travel(k), tc);
    const [x, y] = plan().cells[k].at;
    if (u > 0 && u < 1) out.push({ x: STAND[0] + (x - STAND[0]) * u, y: y - 30, r: 230, i: 0.35 * Math.sin(Math.PI * u) });
  }
  if (t > LEAP && t < MARK) out.push({ x: 1280, y: 700, r: 160, i: 0.3 });
  return out;
}

export default { id: 's3', start: START, end: END, flame, draw, fx, lights };
