// Burn fronts for s3: a ragged ring eats a shape.
import { noise1, noise2, rnd, signed, hash } from '../lib/rand.js';
import { rgba } from '../lib/stroke.js';
import { keys } from '../lib/time.js';

export const SCORCH = [128, 64, 32];
export const BURNT = [62, 30, 18];
const OUTER = [236, 86, 34];
const MID = [255, 168, 60];
const HOT = [255, 228, 160];

export function ring(cx, cy, rho, seed, d = 0) {
  const pts = [];
  for (let i = 0; i < 64; i++) {
    const a = (i / 64) * Math.PI * 2;
    const c = Math.cos(a);
    const s = Math.sin(a);
    const wob = 0.2 * noise2(c * 1.4 + 3, s * 1.4, seed) + 0.08 * noise2(c * 4, s * 4, seed + 1);
    const r = Math.max(0, rho * (1 + wob) + 1.5 * noise1(i * 0.7, hash(seed, d)));
    pts.push([cx + c * r, cy + s * r]);
  }
  return pts;
}

export function trace(ctx, rings) {
  ctx.beginPath();
  for (const r of rings) {
    r.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
    ctx.closePath();
  }
}

export function clip(ctx, rings, outside) {
  ctx.beginPath();
  if (outside) ctx.rect(-10, -10, 1940, 1100);
  for (const r of rings) {
    r.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
    ctx.closePath();
  }
  ctx.clip('evenodd');
}

export const front = (t, t0, dur, reach) => (t < t0 ? 0 : keys(t, [[t0, 0], [t0 + dur, reach, 'out']]));

export function inside(shape, x, y) {
  let on = false;
  for (const c of shape) {
    for (let i = 0, j = c.length - 1; i < c.length; j = i++) {
      const [xi, yi] = c[i];
      const [xj, yj] = c[j];
      if ((yi > y) !== (yj > y) && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) on = !on;
    }
  }
  return on;
}

export function band(ctx, t, outer, inner, shape) {
  ctx.save();
  clip(ctx, [outer, inner], false);
  trace(ctx, shape);
  ctx.clip('nonzero');
  ctx.filter = 'blur(2px)';
  const flick = 0.8 + 0.2 * noise1(t * 26, 53);
  for (const [color, a, op] of [[OUTER, 0.95, 'source-over'], [HOT, 0.7 * flick, 'lighter']]) {
    ctx.globalCompositeOperation = op;
    ctx.fillStyle = rgba(color, a);
    trace(ctx, shape);
    ctx.fill('nonzero');
  }
  ctx.restore();
}

export function licks(ctx, t, edge, shape, seed, size = 1) {
  ctx.save();
  for (let i = 0; i < edge.length; i += 2) {
    const [x, y] = edge[i];
    if (!inside(shape, x, y)) continue;
    const beat = Math.floor(t * 24);
    const h = size * (9 + 13 * rnd(seed, i)) * (0.55 + 0.45 * noise1(t * 13 + i * 0.7, seed));
    const sway = h * 0.35 * noise1(t * 9 + i, seed + 3) + signed(seed, i, beat) * 1.2;
    for (const [color, k] of [[OUTER, 1], [MID, 0.6], [HOT, 0.3]]) {
      const hh = h * k;
      const w = hh * 0.34;
      ctx.fillStyle = rgba(color, 0.92);
      ctx.beginPath();
      ctx.moveTo(x - w, y);
      ctx.quadraticCurveTo(x - w * 0.9, y - hh * 0.6, x + sway * k, y - hh);
      ctx.quadraticCurveTo(x + w * 0.9, y - hh * 0.6, x + w, y);
      ctx.closePath();
      ctx.fill();
    }
  }
  ctx.restore();
}

export function glow(ctx, shape, heat, blur = 3) {
  if (heat <= 0.01) return;
  ctx.save();
  ctx.globalCompositeOperation = 'lighter';
  ctx.filter = `blur(${blur}px)`;
  ctx.fillStyle = rgba([255, 110, 30], 0.9 * heat);
  trace(ctx, shape);
  ctx.fill('nonzero');
  ctx.filter = 'none';
  ctx.fillStyle = rgba(HOT, 0.75 * heat ** 1.6);
  trace(ctx, shape);
  ctx.fill('nonzero');
  ctx.restore();
}
