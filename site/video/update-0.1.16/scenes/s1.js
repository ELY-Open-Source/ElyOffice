// s1: the match strikes; the flame burns 5 into 6.
import { keys, span, clamp, smooth, ease, spring } from '../lib/time.js';
import { noise1 } from '../lib/rand.js';
import { REST } from '../lib/flame.js';
import { hop, first } from '../lib/act.js';
import { burst } from '../lib/embers.js';
import { glyphs } from '../lib/glyph.js';
import { spline } from '../lib/ink.js';
import { resample } from '../lib/stroke.js';
import { head, box, scorch, match, smoke, STRIKE, LEAVE } from './s1-match.js';

const LAND = [800, 690];
const BURN = [5.02, 5.48];
const WRITE = [5.5, 6.0];
const SCORCH = [128, 64, 32];
const BURNT = [62, 30, 18];

let layout = null;
function plan() {
  if (layout) return layout;
  const list = glyphs('0.1.15', { x: 1010, y: 430, size: 240, align: 'center' });
  const five = list[5];
  const b = five.box;
  const w = b.x1 - b.x0;
  const h = b.y1 - b.y0;
  const at = (u, v) => [b.x0 + u * w, b.y0 + v * h];
  const six = spline([[0.86, 0.02], [0.52, 0.06], [0.24, 0.3], [0.12, 0.62], [0.24, 0.92], [0.56, 1.0],
    [0.86, 0.82], [0.84, 0.52], [0.56, 0.4], [0.28, 0.5], [0.16, 0.66]].map(([u, v]) => at(u, v)), 12);
  const spark = at(0.82, 0.04);
  const reach = Math.max(...[[b.x0, b.y0], [b.x0, b.y1], [b.x1, b.y1]].map(([x, y]) => Math.hypot(x - spark[0], y - spark[1]))) + 40;
  const ring = five.contours.map((c) => resample([...c, c[0]], 3));
  layout = { list, five, six, sixPts: resample(six, 2), spark, reach, ring, top: [spark[0] - 4, b.y0 + 6] };
  return layout;
}

const front = (t) => keys(t, [[BURN[0], 0], [BURN[1], plan().reach, 'inOut']]);
const frontAt = (rho, ang) => rho * (1 + 0.16 * noise1(ang * 2.4 + 3, 41) + 0.06 * noise1(ang * 7, 43));

function frontPoly(rho, d) {
  const [cx, cy] = plan().spark;
  const pts = [];
  for (let i = 0; i < 72; i++) {
    const a = (i / 72) * Math.PI * 2;
    const r = Math.max(0, frontAt(rho, a) + 1.5 * noise1(i * 0.7, d));
    pts.push([cx + Math.cos(a) * r, cy + Math.sin(a) * r]);
  }
  return pts;
}

function clipTo(ctx, rings, outside) {
  ctx.beginPath();
  if (outside) ctx.rect(-10, -10, 1940, 1100);
  for (const r of rings) {
    r.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
    ctx.closePath();
  }
  ctx.clip('evenodd');
}

const writeU = (t) => ease.inOut(span(t, WRITE[0], WRITE[1]));

function sixAt(u) {
  const pts = plan().sixPts;
  const i = clamp(u, 0, 1) * (pts.length - 1);
  const k = Math.min(pts.length - 2, Math.floor(i));
  const f = i - k;
  return [pts[k][0] + (pts[k + 1][0] - pts[k][0]) * f, pts[k][1] + (pts[k + 1][1] - pts[k][1]) * f];
}

function motion(f, t) {
  const a = f(t - 1 / 48);
  const b = f(t + 1 / 48);
  return [(b[0] - a[0]) * 24, (b[1] - a[1]) * 24];
}

function onMatch(t) {
  const [hx, hy] = head(Math.floor(t * 12 + 1e-6) / 12);
  const [vx, vy] = motion(head, t);
  return {
    x: hx + 4, y: hy - 6,
    h: keys(t, [[STRIKE, 62], [2.6, 78], [3.3, 92]]),
    roar: keys(t, [[STRIKE, 1], [2.1, 1, 'linear'], [2.75, 0.1, 'out'], [3.1, 0]]),
    energy: 0.7,
    lean: clamp(-vx / 1500 - 0.1 * span(t, 3.1, 3.35), -0.85, 0.85) + 0.12 * noise1(t * 2, 5),
    squash: clamp(1 + vy / 3000, 0.75, 1.3),
    pool: keys(t, [[STRIKE, 330], [2.6, 240], [3.3, 260]]),
  };
}

const toFive = () => ({ t0: 4.83, t1: 5.0, from: LAND, to: plan().top, height: 90, pre: 0.15, post: 0.5 });

function looking(t) {
  return first(
    hop(t, { t0: LEAVE, t1: 4.0, from: [head(LEAVE)[0] + 4, head(LEAVE)[1] - 6], to: LAND, height: 130, post: 0.45 }),
    t < 4.68 && { x: LAND[0], y: LAND[1], lean: keys(t, [[4.3, 0], [4.42, -0.3], [4.55, 0.22]]),
      curl: keys(t, [[4.4, 0], [4.52, 0.72, 'back'], [4.6, 0.72], [4.68, 0, 'snap']]) },
    hop(t, toFive()),
  );
}

function flame(t) {
  if (t < STRIKE) return { alpha: 0, pool: 0 };
  if (t < LEAVE - 0.17) return onMatch(t);
  const pool = keys(t, [[LEAVE, 260], [4.0, 280], [4.8, 600, 'out'], [6.4, 560], [7.6, REST.pool]]);
  const grow = keys(t, [[LEAVE, 92], [4.0, REST.h, 'out']]);
  if (t < 5.0) return { ...looking(t), h: grow, pool, energy: 0.6 };
  if (t < WRITE[0]) {
    return { ...hop(t, toFive()), pool, energy: 1,
      roar: keys(t, [[5.0, 0], [5.06, 0.55, 'out'], [5.3, 0.22], [5.5, 0.15]]) };
  }
  const u = writeU(t);
  const [x, y] = sixAt(u);
  const end = sixAt(1);
  if (t < 6.0) {
    const [vx, vy] = motion((s) => sixAt(writeU(s)), t);
    return { x, y: y + 6, pool, energy: 1, roar: 0.15, lean: clamp(vx / 1200, -0.7, 0.7), squash: clamp(1 + Math.abs(vy) / 4000, 1, 1.25) };
  }
  const roar = keys(t, [[6.0, 0.95], [6.12, 0.85], [6.6, 0.08, 'out'], [6.8, 0]]);
  return {
    ...first(hop(t, { t0: 6.82, t1: 7.0, from: [end[0], end[1] + 6], to: [REST.x, REST.y], height: 120, pre: 0.2, post: 0.6 }),
      t < 7 ? { x: end[0], y: end[1] + 6, squash: 1 - spring(t - 6.0, 0.3) } : {}),
    pool, roar, energy: keys(t, [[6.0, 1], [7.6, REST.energy]]),
  };
}

function numbers(ink, t) {
  const { list, five, ring } = plan();
  const d = ink.drawing;
  ink.write(list.slice(0, 5), { size: 240, style: 'charcoal', fill: 'hatch', gap: 5.5, width: 4.6, alpha: 1 });
  const rho = t < BURN[0] ? 0 : front(t);
  const ctx = ink.ctx;
  if (rho > 0) {
    const inner = frontPoly(Math.max(0, rho - 34), d);
    ctx.save();
    clipTo(ctx, [frontPoly(rho, d)], false);
    const clear = 1 - 0.8 * smooth(span(t, BURN[1], WRITE[1]));
    ink.wash(five.contours, { color: SCORCH, alpha: 0.85 * clear, bleed: 9 });
    ink.wash(five.contours, { color: BURNT, alpha: 0.25 * clear, bleed: 14 });
    ink.smudge(plan().spark[0] - 40, plan().spark[1] + 90, 80, { color: BURNT, alpha: 0.08 });
    ctx.restore();
    ctx.save();
    clipTo(ctx, [frontPoly(rho + 26, d), inner], false);
    ink.wash(five.contours, { color: BURNT, alpha: 0.95, bleed: 3 });
    ink.wash(five.contours, { color: BURNT, alpha: 0.6, bleed: 2 });
    ctx.restore();
  }
  if (rho < plan().reach) {
    ctx.save();
    if (rho > 0) clipTo(ctx, [frontPoly(rho + 26, d)], true);
    ink.write([five], { size: 240, style: 'charcoal', fill: 'hatch', gap: 5.5, width: 4.6, alpha: 1 });
    ctx.restore();
  }
  if (rho > 0 && rho < plan().reach) {
    for (const c of ring) {
      for (let i = 0; i < c.length; i += 4) {
        const [x, y] = c[i];
        const dd = Math.hypot(x - plan().spark[0], y - plan().spark[1]);
        const edge = frontAt(rho, Math.atan2(y - plan().spark[1], x - plan().spark[0]));
        if (dd < edge && dd > edge - 40) ink.smudge(x, y, 7, { color: BURNT, alpha: 0.25, blots: 2 });
      }
    }
  }
}

function six(ink, t) {
  const u = writeU(t);
  if (u <= 0) return;
  const { six: pts } = plan();
  ink.line(pts, { id: 'six-core', width: 16, color: BURNT, alpha: 0.82, upto: u, head: true, taper: [0.03, 0.05], overshoot: 4, boil: 1.4 });
  ink.line(pts, { id: 'six-char', style: 'charcoal', width: 26, color: [30, 20, 16], alpha: 0.9, upto: u, taper: [0.04, 0.06], overshoot: 8, boil: 1.8 });
}

function draw(ctx, t, ink) {
  box(ink);
  scorch(ink, t);
  numbers(ink, t);
  six(ink, t);
  smoke(ink, t);
  const [hx, hy, a] = head(t);
  const lift = 40 + 60 * span(t, LEAVE, 4.6);
  ink.lift(lift, [hx + Math.cos(a) * 170, hy + Math.sin(a) * 170], (k) => match(k, t));
}

function fx(ctx, t, ink) {
  const [hx, hy] = head(Math.min(t, STRIKE));
  for (const [t0, count, speed, seed] of [[1.0, 9, 220, 1], [1.5, 18, 330, 2]]) {
    burst(ctx, t, { t0, x: hx, y: hy + 4, count, speed, life: 0.3, spread: 0.9, angle: -2.6, gravity: 500, seed, size: 1.8 });
  }
  burst(ctx, t, { t0: STRIKE, x: 420, y: 852, count: 80, speed: 760, life: 0.85, spread: 1.2, angle: -2.3, gravity: 520, seed: 3, size: 2.4 });
  const flash = Math.exp(-(t - STRIKE) * 14) * (t >= STRIKE ? 1 : 0);
  if (flash > 0.01) {
    ctx.globalCompositeOperation = 'lighter';
    const g = ctx.createRadialGradient(420, 846, 0, 420, 846, 260);
    g.addColorStop(0, `rgba(255,246,220,${0.9 * flash})`);
    g.addColorStop(1, 'rgba(255,140,60,0)');
    ctx.fillStyle = g;
    ctx.fillRect(160, 586, 520, 520);
  }
  const { spark } = plan();
  const tb = Math.floor(t * 12 + 1e-6) / 12;
  const rho = tb < BURN[0] ? 0 : front(tb);
  if (rho > 0 && rho < plan().reach) {
    const d = ink.drawing >> 1;
    ctx.save();
    clipTo(ctx, [frontPoly(rho + 3, d), frontPoly(Math.max(0, rho - 20), d)], false);
    ctx.filter = 'blur(2px)';
    const flick = 0.8 + 0.2 * noise1(t * 26, 53);
    for (const [color, a, op] of [[[236, 84, 20], 0.95, 'source-over'], [[255, 210, 120], 0.7 * flick, 'lighter']]) {
      ctx.globalCompositeOperation = op;
      ctx.fillStyle = `rgba(${color},${a})`;
      ctx.beginPath();
      for (const c of plan().five.contours) {
        c.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
        ctx.closePath();
      }
      ctx.fill('nonzero');
    }
    ctx.restore();
  }
  for (let k = 0; k < 4; k++) {
    const t0 = BURN[0] + 0.03 + k * 0.1;
    const r0 = front(t0);
    const a = -0.6 - k * 0.7;
    burst(ctx, t, { t0, x: spark[0] + Math.cos(a) * r0 * 0.7, y: spark[1] + Math.sin(a) * r0 * 0.7, count: 10, speed: 160, life: 0.7, gravity: -90, seed: 10 + k, size: 1.7 });
  }
  const u = writeU(t);
  if (u > 0) {
    const cool = clamp(1 - (t - WRITE[1]) / 0.7);
    if (cool > 0) {
      ctx.globalCompositeOperation = 'lighter';
      ink.line(plan().six, { id: 'six-hot', width: 12, color: [255, 120, 36], alpha: 0.75 * cool, from: Math.max(0, u - 0.4 - (1 - cool)), upto: u, taper: [0.5, 0.05], overshoot: 0, boil: 1.2 });
      ink.line(plan().six, { id: 'six-white', width: 5, color: [255, 226, 160], alpha: 0.8 * cool, from: Math.max(0, u - 0.14), upto: u, taper: [0.6, 0.05], overshoot: 0, boil: 1 });
    }
  }
  const end = sixAt(1);
  burst(ctx, t, { t0: WRITE[1], x: end[0], y: end[1] - 40, count: 46, speed: 520, life: 0.9, spread: Math.PI, gravity: 240, seed: 20, size: 2.2 });
}

function lights(t) {
  const out = [];
  const [hx, hy] = head(Math.min(t, STRIKE));
  for (const [t0, r, i] of [[1.0, 150, 0.9], [1.5, 210, 1.15]]) {
    if (t >= t0) out.push({ x: hx, y: hy, r, i: i * Math.exp(-(t - t0) * 16), c: [1.05, 1, 0.95] });
  }
  if (t >= STRIKE) out.push({ x: 430, y: 830, r: 560, i: 1.05 * Math.exp(-(t - STRIKE) * 8), c: [1.1, 1.04, 0.96] });
  const rho = t < BURN[0] ? 0 : front(t);
  if (rho > 0 && rho < plan().reach) {
    const [sx, sy] = plan().spark;
    out.push({ x: sx - rho * 0.3, y: sy + rho * 0.55, r: 240, i: 0.35 * smooth(span(t, BURN[0], BURN[0] + 0.1)) });
  }
  return out;
}

export default { id: 's1', start: 0, end: 8, flame, draw, fx, lights };
