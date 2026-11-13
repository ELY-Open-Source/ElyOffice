// s4: seven apps, one engine; Windows opens at once.
import { keys, span, clamp, ease, spring } from '../lib/time.js';
import { hash } from '../lib/rand.js';
import { REST, drawFlame } from '../lib/flame.js';
import { hop } from '../lib/act.js';
import { burst } from '../lib/embers.js';
import { ICONS, local } from './s4-icons.js';
import { HIT, fly, ENGINE, FACE, HOLE, landAt, iconAt, velocity, back, front, sitting, flying, glowOf } from './s4-box.js';
import { LAND, OPEN, FAR, reach, drawWindow, frontGlow } from './s4-window.js';

const HOME = [REST.x, REST.y];
const PERCH = [480, 748];
const POUR = 35.52;
const toBox = { t0: 38.6, t1: 39.0, from: HOME, to: PERCH, height: 110, pre: 0.15, post: 0.35 };
const leap = { t0: 39.55, t1: OPEN, from: PERCH, to: LAND, height: 250, pre: 0.2, post: 0.35 };
const home = { t0: 40.5, t1: 40.85, from: LAND, to: HOME, height: 120, pre: 0.15, post: 0.45 };

function watch(t) {
  let lean = 0;
  let weight = 0;
  for (let i = 0; i < 7; i++) {
    const u = (t - (landAt(i) - fly(i))) / fly(i);
    if (u <= 0 || u >= 1.3) continue;
    const p = iconAt(i, t);
    const w = Math.sin(Math.PI * clamp(u / 1.3));
    lean += w * clamp(Math.atan2(p.x - REST.x, REST.y - p.y) * 0.3, -0.4, 0.4);
    weight += w;
  }
  return weight > 0 ? lean / Math.max(1, weight) : 0;
}

function bob(t) {
  let s = 0;
  for (let i = 0; i < 7; i++) s += spring(t - landAt(i), 0.12, 18, 9);
  return s;
}

function stand(t) {
  const duck = keys(t, [[34.12, 0], [34.24, 0.4, 'out'], [34.36, 0.4], [34.55, 0, 'back']]);
  const look = keys(t, [[32.0, 0], [32.35, -0.14], [35.7, -0.14], [35.9, -0.3], [36.3, -0.1], [36.7, 0.32], [37.6, 0.3], [38.3, 0.34], [38.45, -0.2]]);
  const curl = keys(t, [[37.75, 0], [37.92, 0.72, 'back'], [38.18, 0.72], [38.3, 0, 'snap']]);
  const joy = hop(t, { t0: 36.02, t1: 36.3, from: HOME, to: HOME, height: 46, pre: 0.1, post: 0.35 });
  const squash = joy ? joy.squash : 1 - bob(t) - duck;
  return { x: HOME[0], y: HOME[1] + (joy ? joy.y - HOME[1] : 0), squash, lean: look + (t < 36 ? watch(t) * 0.8 : 0) + 0.5 * duck, curl };
}

function body(t) {
  if (t < toBox.t0 - toBox.pre) return stand(t);
  if (t < leap.t0 - leap.pre) return hop(t, toBox);
  if (t < home.t0 - home.pre) return hop(t, leap);
  return hop(t, home) ?? { lean: keys(t, [[41.3, 0], [41.5, 0.12], [41.8, 0]]) };
}

function flame(t) {
  return {
    ...body(t),
    h: keys(t, [[38.9, REST.h], [39.2, 176, 'out'], [40.4, 176], [40.85, REST.h]]),
    roar: keys(t, [[35.96, 0], [36.03, 0.6, 'out'], [36.5, 0], [39.0, 0], [39.08, 0.8, 'out'], [39.4, 0.3], [OPEN, 0.35], [OPEN + 0.04, 0.95, 'out'], [40.45, 0.15], [41.0, 0]]),
    energy: keys(t, [[32.0, 0.5], [35.6, 0.6], [36.0, 0.95], [36.7, 0.55], [38.9, 0.6], [39.1, 1], [40.4, 0.9], [41.6, REST.energy]]),
    pool: keys(t, [[36.4, REST.pool], [37.0, 660, 'out'], [38.45, 660], [38.9, 600], [39.25, 760, 'out'], [39.9, 700], [40.05, 640, 'out'], [41.0, REST.pool]]),
  };
}

function draw(ctx, t, ink) {
  back(ink, t);
  sitting(ink, t);
  front(ink, t);
  drawWindow(ink, t);
  flying(ink, t);
}

const twos = (t) => Math.floor(t * 12 + 1e-6) / 12;

const RANK = [2, 3, 6, 4, 5, 0, 1];

function hover(i) {
  const at = local(iconAt(i, POUR), ICONS[i].heart);
  const x = ENGINE[0] + (at[0] - ENGINE[0]) * 1.4;
  return { at, to: [x, 640 + ((x - ENGINE[0]) / 200) ** 2 * 26] };
}

function poured(i, t) {
  const { at, to } = hover(i);
  const r0 = POUR + 0.03 * RANK[i];
  const d0 = 35.78 + 0.02 * RANK[i];
  const up = ease.out(span(t, r0, r0 + 0.22));
  const down = ease.in(span(t, d0, d0 + 0.1));
  const x = at[0] + (to[0] - at[0]) * up + (ENGINE[0] - to[0]) * down;
  const y = at[1] + (to[1] - at[1]) * up + (ENGINE[1] - to[1]) * down + 4 * Math.sin(t * 9 + i) * up * (1 - down);
  return {
    x, y, h: 34 + 14 * up - 22 * down, energy: 0.9, glow: 0.75, alpha: 1, inside: true,
    lean: 0.25 * Math.sin(t * 5 + i * 2) * up * (1 - down) + clamp((ENGINE[0] - to[0]) / 160, -0.9, 0.9) * down,
    squash: 1 - 0.25 * Math.sin(Math.PI * up) * (1 - down) + 0.5 * down,
  };
}

function lamp(i, t) {
  if (t >= 35.9 + 0.02 * RANK[i]) return null;
  if (t >= POUR) return poured(i, t);
  const tb = twos(t);
  const p = iconAt(i, tb);
  if (!p || p.y < -120) return null;
  const [x, y] = local(p, ICONS[i].heart);
  const [vx, vy] = velocity(i, tb);
  return {
    x, y, h: 34, energy: 0.85, glow: 0.7, alpha: 1, inside: p.inside,
    lean: clamp(-vx / 1700, -0.7, 0.7), squash: clamp(1 + Math.max(0, vy) / 2600, 1, 1.4),
  };
}

function engine(t) {
  if (t < 35.85) return null;
  return {
    x: ENGINE[0], y: ENGINE[1], energy: 1, glow: 0.9, alpha: 1,
    h: keys(t, [[35.85, 0], [HIT - 0.05, 110, 'in'], [HIT, 235, 'out'], [37.0, 108], [39.0, 108], [39.12, 62, 'out'], [39.6, 62], [40.3, 108]]),
    roar: keys(t, [[HIT - 0.05, 0.2], [HIT, 1, 'out'], [36.6, 0.25], [37.4, 0]]),
    lean: 0.06 * Math.sin(t * 2.3),
  };
}

function clipFace(ctx) {
  ctx.beginPath();
  ctx.rect(-10, -10, 1940, 1100);
  FACE.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
  ctx.closePath();
  ctx.clip('evenodd');
}

function fx(ctx, t, ink) {
  const n = Math.round(t * 24);
  for (let i = 0; i < 7; i++) {
    const [bx, by] = local(iconAt(i, landAt(i)), ICONS[i].heart);
    burst(ctx, t, { t0: landAt(i), x: bx, y: by - 20, count: 12, speed: 260, life: 0.5, spread: 1.1, gravity: 300, seed: 60 + i, size: 1.6 });
    const s = lamp(i, t);
    if (!s) continue;
    ctx.save();
    if (s.inside) clipFace(ctx);
    drawFlame(ctx, s, t + i * 0.37, hash(n, i));
    ctx.restore();
  }
  const e = engine(t);
  if (e) {
    ctx.save();
    clipFace(ctx);
    drawFlame(ctx, e, t + 3.1, hash(n, 9));
    ctx.restore();
    const g = glowOf(t);
    ctx.save();
    ctx.globalCompositeOperation = 'lighter';
    ctx.filter = 'blur(4px)';
    ctx.fillStyle = `rgba(255,150,60,${clamp(0.55 * g)})`;
    ctx.beginPath();
    HOLE.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
    ctx.closePath();
    ctx.fill();
    ctx.restore();
  }
  burst(ctx, t, { t0: HIT, x: ENGINE[0], y: 740, count: 110, speed: 980, life: 1.2, spread: 0.7, angle: -Math.PI / 2, gravity: 460, seed: 41, size: 2.6 });
  burst(ctx, t, { t0: HIT + 0.08, x: ENGINE[0], y: 730, count: 40, speed: 420, life: 1.4, spread: 1.4, angle: -Math.PI / 2, gravity: -60, seed: 43, size: 1.8 });
  for (let k = 0; k < 6; k++) {
    const t0 = leap.t0 + 0.07 * k;
    const s = hop(t0, leap);
    burst(ctx, t, { t0, x: s.x, y: s.y - 40, count: 8, speed: 140, life: 0.55, gravity: 80, seed: 70 + k, size: 1.8 });
  }
  burst(ctx, t, { t0: OPEN, x: LAND[0], y: LAND[1] - 20, count: 70, speed: 640, life: 0.8, spread: Math.PI, gravity: 320, seed: 44, size: 2.2 });
  frontGlow(ctx, t, ink.drawing >> 1);
}

function lights(t) {
  const out = [];
  for (let i = 0; i < 7; i++) {
    const s = lamp(i, t);
    if (s) out.push({ x: s.x, y: s.y - 18, r: 150, i: s.inside ? 0.4 : 0.55, c: [1.05, 0.95, 0.85] });
  }
  const g = glowOf(t);
  if (g > 0) out.push({ x: ENGINE[0], y: 700, r: 300 * (1 + 0.5 * clamp(g - 0.85)), i: g, c: [1.08, 0.95, 0.82] });
  const r = reach(t);
  if (r > 0 && r < FAR) out.push({ x: LAND[0], y: LAND[1] + r * 0.5, r: 260 + r * 0.5, i: 0.6 * (1 - r / FAR) });
  return out;
}

export default { id: 's4', start: 32, end: 42, flame, draw, fx, lights };
