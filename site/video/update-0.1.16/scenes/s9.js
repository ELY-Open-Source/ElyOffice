// s9: flashback, the title in fire, down to the match.
import { keys, span, smooth, spring } from '../lib/time.js';
import { noise1 } from '../lib/rand.js';
import { REST } from '../lib/flame.js';
import { hop, first } from '../lib/act.js';
import { burst } from '../lib/embers.js';
import { glyphs } from '../lib/glyph.js';
import { fireText, emberPoints, emberText } from '../lib/fire.js';
import { spent } from './s9-match.js';
import { moment, FLASH, s8 } from './s9-flash.js';

const HIT = 86.0;
const LEAP = [85.1, 85.4];
const DOWN = [87.75, 88.0];
const WINK = 88.5;
const OUT = 89.3;
const TITLE = { x: 960, y: 500, size: 220, align: 'center' };
const VERSION = { x: 960, y: 690, size: 120, align: 'center' };
const HEAD = [790, 884, -0.1];
const BURNT = [52, 30, 20];
const ASH = [70, 62, 58];

let layout = null;
function plan() {
  if (layout) return layout;
  const title = glyphs('Ely Office', TITLE);
  const version = glyphs('0.1.16', VERSION);
  const reach = Math.max(...title.filter((g) => g.box).map((g) => Math.abs((g.box.x0 + g.box.x1) / 2 - 960)));
  layout = { title, version, embers: emberPoints(version, VERSION.size), reach };
  return layout;
}

function heat(i, t) {
  const g = plan().title[i];
  if (!g.box) return 0;
  const d = Math.abs((g.box.x0 + g.box.x1) / 2 - 960) / plan().reach;
  const on = smooth(span(t, LEAP[1] + d * 0.45, LEAP[1] + d * 0.45 + 0.16));
  const off = smooth(span(t, 87.2 + (1 - d) * 0.45, 87.2 + (1 - d) * 0.45 + 0.3));
  return on * (1 - off);
}

const swell = (t) => 1 + 0.6 * Math.exp(-Math.max(0, t - HIT) * 4) * (t >= HIT ? 1 : 0);
const emberHeat = (t) => keys(t, [[85.4, 1], [87.4, 1], [88.2, 0.25, 'out'], [88.8, 0]]);
const charred = (t) => smooth(span(t, 86.2, 87.8));
const tip = [HEAD[0] + 4, HEAD[1] - 8];

function flame(t) {
  const m = moment(t);
  if (m) return m.scene.flame(m.at);
  if (t < FLASH[0]) return { ember: keys(t, [[82.0, 0], [82.42, 0.55]]), energy: keys(t, [[82.0, REST.energy], [82.42, 0.2]]) };
  if (t < LEAP[0] - 0.17) {
    return {
      roar: keys(t, [[FLASH[1], 1], [84.6, 1], [85.0, 0.6]]), energy: 1.3, h: keys(t, [[FLASH[1], REST.h], [84.8, 190]]),
      squash: 1 + spring(t - FLASH[1], 0.25, 10, 5), pool: keys(t, [[FLASH[1], REST.pool], [84.9, 680]]),
    };
  }
  if (t < LEAP[1] + 0.12) {
    return { ...hop(t, { t0: LEAP[0], t1: LEAP[1], from: [REST.x, REST.y], to: [960, 520], height: 110, post: 0.2 }),
      h: 190, roar: 0.8, energy: 1.3, pool: 680, alpha: keys(t, [[LEAP[1], 1], [LEAP[1] + 0.12, 0]]) };
  }
  if (t < 87.6) return { alpha: 0, pool: 0 };
  const glowing = keys(t, [[88.36, 1], [WINK - 0.04, 1.1], [WINK, 2.6, 'snap'], [WINK + 0.14, 0.7, 'out'], [OUT, 0]]);
  return {
    ...first(hop(t, { t0: DOWN[0], t1: DOWN[1], from: [960, 520], to: tip, height: 60, pre: 0.12, post: 0.3 }),
      t < DOWN[0] ? { x: 960, y: 520 } : { x: tip[0], y: tip[1] }),
    alpha: keys(t, [[87.6, 0], [87.72, 1], [WINK + 0.14, 1], [OUT, 0, 'in']]),
    h: 100, energy: keys(t, [[87.6, 0.8], [88.2, 0]]), ember: keys(t, [[88.0, 0], [88.36, 1]]), sparks: keys(t, [[88.0, 1], [88.4, 0]]),
    glow: glowing, pool: keys(t, [[87.6, 520], [88.2, 470], [WINK, 540], [OUT, 140]]),
  };
}

function draw(ctx, t, ink) {
  const m = moment(t);
  if (m) {
    m.scene.draw(ctx, m.at, ink);
    return;
  }
  if (t < FLASH[0]) {
    s8.draw(ctx, s8.end - 1 / 12, ink);
    return;
  }
  if (t < FLASH[1]) return;
  spent(ink, HEAD);
  const { title, version, embers } = plan();
  const c = charred(t);
  if (c > 0) {
    ink.write(title, { size: TITLE.size, style: 'charcoal', fill: 'hatch', color: BURNT, gap: 5, width: 3.4, alpha: 0.9 * c });
    const ash = smooth(span(t, 87.4, 88.4));
    for (const [i, [x, y]] of embers.entries()) {
      if (i % 2 || ash <= 0) continue;
      ink.ctx.fillStyle = `rgba(${ASH},${(0.75 * ash).toFixed(3)})`;
      ink.ctx.fillRect(x - 1.6, y - 1.6, 3.2, 3.2);
    }
    const line = smooth(span(t, 87.6, 88.6));
    if (line > 0) ink.write(version, { size: VERSION.size, style: 'charcoal', fill: 'none', color: ASH, width: 1.6, alpha: 0.5 * line, outline: true });
  }
}

function fx(ctx, t, ink) {
  const m = moment(t);
  if (m) {
    m.scene.fx?.(ctx, m.at, ink);
    return;
  }
  const { title, embers } = plan();
  fireText(ctx, title, t, ink.drawing, { size: TITLE.size, heat: (i) => heat(i, t), tall: swell(t) });
  emberText(ctx, embers, t, { from: [960, 470], t0: LEAP[1] + 0.05, t1: HIT, heat: emberHeat, seed: 9 });
  burst(ctx, t, { t0: LEAP[1], x: 960, y: 470, count: 70, speed: 620, life: 0.9, spread: Math.PI, gravity: 200, seed: 91, size: 2.4 });
  for (let k = 0; k < 4; k++) {
    burst(ctx, t, { t0: HIT, x: 640 + k * 213, y: 380, count: 26, speed: 480, life: 1.1, angle: -Math.PI / 2, spread: 1.2, gravity: 160, seed: 92 + k, size: 2.2 });
  }
}

function lights(t) {
  const m = moment(t);
  if (m) return m.scene.lights?.(m.at) ?? [];
  const thirds = [[0, 3, 700], [3, 5, 960], [5, 10, 1220]];
  const s = swell(t);
  return thirds.map(([a, b, x]) => {
    let h = 0;
    for (let i = a; i < b; i++) h = Math.max(h, heat(i, t));
    return { x, y: 440, r: 560 * (0.85 + 0.15 * s), i: 0.42 * h * s * (1 + 0.05 * noise1(t * 9, a)) };
  }).filter((l) => l.i > 0.005).concat(emberHeat(t) > 0 && t > LEAP[1] ? [{ x: 960, y: 670, r: 260, i: 0.3 * emberHeat(t) }] : []);
}

function ambient(t) {
  const m = moment(t);
  if (m) return m.scene.ambient?.(m.at) ?? 0;
  const a = keys(t, [[85.9, 0], [HIT, 0.62, 'snap'], [86.6, 0.3], [87.4, 0.14], [88.0, 0]]);
  return [a, a * 0.78, a * 0.52];
}

export default { id: 's9', start: 82, end: 90, flame, draw, fx, lights, ambient };
