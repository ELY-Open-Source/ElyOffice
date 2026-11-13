// Hatching and watercolour washes inside drawn shapes.
import { draw, resample, length, rgba, CHARCOAL } from './stroke.js';
import { noise1, rnd, signed, hash } from './rand.js';

function crossings(shape, cos, sin, v) {
  const us = [];
  for (const c of shape) {
    for (let i = 0; i < c.length; i++) {
      const a = c[i];
      const b = c[(i + 1) % c.length];
      const av = -a[0] * sin + a[1] * cos;
      const bv = -b[0] * sin + b[1] * cos;
      if ((av <= v) === (bv <= v)) continue;
      const k = (v - av) / (bv - av);
      const au = a[0] * cos + a[1] * sin;
      const bu = b[0] * cos + b[1] * sin;
      us.push(au + (bu - au) * k);
    }
  }
  return us.sort((x, y) => x - y);
}

export function hatch(ctx, shape, o, seed) {
  const angle = o.angle ?? -0.9;
  const gap = o.gap ?? 7;
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  let lo = Infinity;
  let hi = -Infinity;
  for (const c of shape) for (const [x, y] of c) {
    const v = -x * sin + y * cos;
    lo = Math.min(lo, v);
    hi = Math.max(hi, v);
  }
  const line = { style: o.style ?? 'charcoal', width: o.width ?? 2.2, color: o.color ?? CHARCOAL,
    alpha: o.alpha ?? 0.7, taper: [0.15, 0.3], pressure: 0.35 };
  let k = 0;
  for (let v = lo + gap * (0.3 + 0.5 * rnd(seed, 1)); v < hi; v += gap * (0.8 + 0.4 * rnd(seed, 2, k))) {
    const us = crossings(shape, cos, sin, v);
    for (let i = 0; i + 1 < us.length; i += 2) {
      const reach = o.reach ?? 1;
      const a = us[i] + signed(seed, 3, k, i) * gap * 0.4;
      const b = a + (us[i + 1] - a) * Math.min(1, reach + 0.12 * signed(seed, 4, k, i));
      if (b - a < 2) continue;
      const p0 = [a * cos - v * sin, a * sin + v * cos];
      const p1 = [b * cos - v * sin, b * sin + v * cos];
      draw(ctx, [p0, p1], { ...line, overshoot: gap * 0.5 }, hash(seed, 5, k, i));
    }
    k++;
  }
}

function deform(contour, bleed, seed) {
  const closed = [...contour, contour[0]];
  const len = length(closed);
  const pts = resample(closed, Math.max(2, Math.min(7, len / 40)));
  const out = [];
  for (let i = 0; i < pts.length; i++) {
    const p = pts[Math.max(i - 1, 0)];
    const q = pts[Math.min(i + 1, pts.length - 1)];
    const nx = -(q[1] - p[1]);
    const ny = q[0] - p[0];
    const nl = Math.hypot(nx, ny) || 1;
    const d = bleed * (0.6 * noise1(pts[i][2] / 45, seed) + 0.4 * noise1(pts[i][2] / 13, seed + 9));
    out.push([pts[i][0] + (nx / nl) * d, pts[i][1] + (ny / nl) * d]);
  }
  return out;
}

function trace(ctx, shape) {
  ctx.beginPath();
  for (const c of shape) {
    c.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
    ctx.closePath();
  }
}

export function wash(ctx, shape, o, seed) {
  const color = o.color;
  const alpha = o.alpha ?? 0.35;
  const bleed = o.bleed ?? 5;
  ctx.save();
  ctx.globalCompositeOperation = 'multiply';
  for (let layer = 0; layer < 3; layer++) {
    const s = hash(seed, 11, layer);
    const ox = signed(s, 1) * bleed * 0.5;
    const oy = signed(s, 2) * bleed * 0.5;
    trace(ctx, shape.map((c) => deform(c, bleed, s).map(([x, y]) => [x + ox, y + oy])));
    ctx.fillStyle = rgba(color, alpha * (layer ? 0.3 : 0.55));
    ctx.fill('evenodd');
  }
  const edge = shape.map((c) => deform(c, bleed * 0.8, hash(seed, 12)));
  trace(ctx, edge);
  ctx.strokeStyle = rgba(color.map((v) => v * 0.7), alpha * 0.5);
  ctx.lineWidth = 1.6;
  ctx.stroke();
  ctx.restore();
}

export function smudge(ctx, x, y, r, o, seed) {
  ctx.save();
  ctx.globalCompositeOperation = 'multiply';
  const n = o.blots ?? 5;
  for (let i = 0; i < n; i++) {
    const bx = x + signed(seed, i, 1) * r * 0.5;
    const by = y + signed(seed, i, 2) * r * 0.35;
    const br = r * (0.35 + 0.5 * rnd(seed, i, 3));
    const g = ctx.createRadialGradient(bx, by, 0, bx, by, br);
    g.addColorStop(0, rgba(o.color ?? CHARCOAL, (o.alpha ?? 0.18) / Math.sqrt(n / 3)));
    g.addColorStop(1, rgba(o.color ?? CHARCOAL, 0));
    ctx.fillStyle = g;
    ctx.beginPath();
    ctx.ellipse(bx, by, br, br * (0.5 + 0.4 * rnd(seed, i, 4)), signed(seed, i, 5), 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.restore();
}
