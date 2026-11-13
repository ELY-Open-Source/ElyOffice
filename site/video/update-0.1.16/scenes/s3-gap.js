// s3 line one: the gap before Microsoft.
import { glyphs, advance } from '../lib/glyph.js';
import { keys, span, spring, clamp } from '../lib/time.js';
import { BURNT } from './s3-burn.js';

const SIZE = 110;
export const BASE = 400;
export const STAMPS = [22.5, 23.0, 23.5];
export const SNAP = 24.0;
const JOLTS = [[22.5, 3.5], [23.0, 3.5], [23.5, 3.5], [SNAP, 7]];

let made = null;
function plan() {
  if (made) return made;
  const han = '微软雅黑';
  const x0 = 960 - advance(`${han} Microsoft`, SIZE) / 2;
  const cjk = glyphs(han, { x: x0, y: BASE, size: SIZE });
  const latin = glyphs('Microsoft', { x: x0 + advance(`${han} `, SIZE), y: BASE, size: SIZE });
  made = { cjk, latin, seam: [cjk[3].box.x1, latin[0].box.x0] };
  return made;
}

export const open = (t) => keys(t, [[22.44, 150], [22.5, 115, 'snap'], [22.94, 115], [23.0, 85, 'snap'],
  [23.44, 85], [23.5, 60, 'snap'], [23.75, 60], [23.88, 70, 'out'], [SNAP, 0, 'in']]);

export const gapX = (t) => (plan().seam[0] + plan().seam[1] + open(t)) / 2;
export const seamX = () => (plan().seam[0] + plan().seam[1]) / 2;

const shove = (t, k) => JOLTS.reduce((s, [tj, a]) => s + spring(t - tj, -a * (0.15 + 0.3 * k), 17, 8), 0);
const kick = (t, i) => spring(t - SNAP - 0.02 * (3 - i), -(2 + 2.4 * i), 15, 7);

function moved(g, dx) {
  return {
    ...g,
    contours: g.contours.map((c) => c.map(([x, y]) => [x + dx, y])),
    box: g.box && { ...g.box, x0: g.box.x0 + dx, x1: g.box.x1 + dx },
  };
}

export function lineOne(ink, t) {
  const { cjk, latin } = plan();
  const g = open(t);
  const list = [...cjk.map((c, i) => moved(c, kick(t, i))), ...latin.map((c, k) => moved(c, g + shove(t, k)))];
  const upto = span(t, 20.2, 20.8);
  if (upto <= 0) return;
  ink.lift(7, [960, BASE - 50], (k) => k.write(list, { size: SIZE, fill: 'hatch', alpha: 0.92, upto }));
  streaks(ink, t, list);
  impact(ink, t);
}

function streaks(ink, t, list) {
  const tj = [...STAMPS, SNAP].find((s) => t >= s - 0.09 && t < s + 0.17);
  if (tj === undefined) return;
  const fade = clamp(1 - (t - tj) / 0.17);
  const big = tj === SNAP;
  for (const [k, g] of list.slice(4).entries()) {
    if (!g.box || k % (big ? 1 : 3)) continue;
    const y = g.box.y0 + (g.box.y1 - g.box.y0) * (0.3 + 0.4 * ((k * 0.618) % 1));
    const len = (big ? 70 : 34) * fade;
    ink.line([[g.box.x1 + 6, y], [g.box.x1 + 6 + len, y]], { style: 'charcoal', width: 2.2, alpha: 0.55 * fade, taper: [0.1, 0.7] });
  }
}

function impact(ink, t) {
  const age = t - SNAP;
  if (age < 0 || age > 0.42) return;
  const x = seamX();
  const u = clamp(age / 0.42);
  for (let k = 0; k < 5; k++) {
    const a = -Math.PI / 2 + (k - 2) * 0.32;
    const r0 = 104 + 30 * u;
    const r1 = r0 + 46 * (1 - u * 0.6);
    const cy = BASE - 44;
    for (const sg of [1, -1]) {
      const c = Math.cos(a);
      const s = Math.sin(a) * sg;
      ink.line([[x + c * r0, cy + s * r0 * 0.9], [x + c * r1, cy + s * r1 * 0.9]], { width: 3.2, alpha: 0.85 * (1 - u), taper: [0.2, 0.5] });
    }
  }
  ink.smudge(x, BASE + 8, 40 + 60 * u, { color: BURNT, alpha: 0.22 * (1 - u), blots: 4 });
  ink.smudge(x, BASE - 100, 30 + 50 * u, { alpha: 0.12 * (1 - u), blots: 3 });
}
