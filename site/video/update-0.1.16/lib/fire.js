// Letters made of fire, and letters made of embers.
import { noise1, rnd, signed, hash } from './rand.js';
import { clamp, ease, FPS } from './time.js';
import { draw as stroke, resample, rgba } from './stroke.js';
import { bounds, sheet, grain, settle, path, tone, OUTER, MID, CORE, LINE } from './paint.js';

const EMBER = [200, 44, 12];
const memo = new Map();

function inside(contours, x, y) {
  let hit = false;
  for (const c of contours) {
    for (let i = 0, j = c.length - 1; i < c.length; j = i++) {
      const [xi, yi] = c[i];
      const [xj, yj] = c[j];
      if ((yi > y) !== (yj > y) && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) hit = !hit;
    }
  }
  return hit;
}

function roots(g, size) {
  const key = `${g.ch}|${g.box.x0.toFixed(1)}|${g.box.y0.toFixed(1)}|${size}`;
  if (memo.has(key)) return memo.get(key);
  const out = [];
  const step = Math.max(8, size * 0.1);
  for (const c of g.contours) {
    for (const [x, y] of resample([...c, c[0]], step)) {
      if (!inside(g.contours, x, y - 4) && inside(g.contours, x, y + 4)) out.push([x, y + 2]);
    }
  }
  memo.set(key, out);
  return out;
}

function lick(x, y, h, w, lean, t, seed) {
  const pts = [];
  const N = 10;
  for (let i = 0; i <= N; i++) {
    const u = i / N;
    const sway = (lean + 0.5 * noise1(t * 3 - u * 1.4, seed)) * h * u * u;
    pts.push([x + sway + w * Math.sin(Math.PI * Math.min(1, u * 1.4)) * (1 - u) ** 0.65 * 0.9, y - h * u]);
  }
  for (let i = N - 1; i > 0; i--) {
    const u = i / N;
    const sway = (lean + 0.5 * noise1(t * 3 - u * 1.4, seed)) * h * u * u;
    pts.push([x + sway - w * Math.sin(Math.PI * Math.min(1, u * 1.4)) * (1 - u) ** 0.65 * 0.9, y - h * u]);
  }
  return pts;
}

function edges(ctx, poly, alpha, seed) {
  const tip = poly.length >> 1;
  const sides = [poly.slice(0, tip + 1), [poly[0], ...poly.slice(tip + 1).reverse(), poly[tip]]];
  sides.forEach((side, k) => stroke(ctx, side, { width: 1.6, color: LINE, alpha, upto: 0.86, overshoot: 1.6,
    taper: [0.1, 0.5], pressure: 0.4, boil: 0.8 }, hash(seed, k)));
}

export function fireText(ctx, list, t, n, o) {
  const size = o.size;
  const lit = list.map((g, i) => ({ g, i, heat: g.contours.length ? clamp(o.heat(i)) : 0 })).filter((e) => e.heat > 0.01);
  if (!lit.length) return;
  const tongues = [];
  for (const { g, heat } of lit) {
    roots(g, size).forEach(([x, y], k) => {
      const s = hash(g.ch, Math.round(x), Math.round(y));
      const h = size * heat * (0.16 + 0.3 * rnd(s, 1)) * (1 + 0.35 * noise1(t * 4.2 + k, s)) * (o.tall ?? 1);
      if (h > 2) tongues.push({ x, y, h, w: size * (0.07 + 0.04 * rnd(s, 2)) * Math.sqrt(heat), lean: signed(s, 3) * 0.25, s, heat });
    });
  }
  const outerTongues = tongues.map((q) => lick(q.x, q.y, q.h, q.w, q.lean, t, q.s));
  const glyphs = lit.map((e) => e.g.contours);
  const box = bounds([...glyphs.flat(), ...outerTongues], size * 0.12);
  const L = sheet(box);
  const alpha = (heat) => clamp(heat * 1.6);
  for (const { g, i, heat } of lit) {
    L.lineJoin = 'round';
    L.lineWidth = size * 0.05 * heat;
    L.strokeStyle = rgba(OUTER, alpha(heat));
    path(L, g.contours);
    L.stroke();
    tone(L, g.contours, OUTER, alpha(heat), 0.8, hash(n, i, 1));
  }
  tongues.forEach((q, i) => tone(L, [outerTongues[i]], OUTER, alpha(q.heat), 0.8, hash(n, i, 2)));
  tongues.forEach((q, i) => tone(L, [lick(q.x, q.y - 2, q.h * 0.6, q.w * 0.6, q.lean, t + 0.07, q.s)], MID, alpha(q.heat), 0.6, hash(n, i, 3)));
  for (const { g, i, heat } of lit) {
    tone(L, g.contours, CORE, alpha(heat), 0.6, hash(n, i, 4));
    L.lineWidth = size * 0.045;
    L.strokeStyle = rgba(MID, alpha(heat));
    path(L, g.contours);
    L.stroke();
  }
  grain(L, box, 0.55);
  settle(ctx, box);
  ctx.save();
  ctx.beginPath();
  ctx.rect(box.x, box.y, box.w, box.h);
  for (const c of glyphs.flat()) {
    c.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
    ctx.closePath();
  }
  ctx.clip('evenodd');
  tongues.forEach((q, i) => {
    if (q.h > size * 0.12) edges(ctx, outerTongues[i], 0.75 * alpha(q.heat), hash(n, i, 5));
  });
  ctx.restore();
  for (const { g, i, heat } of lit) {
    g.contours.forEach((c, k) => stroke(ctx, c, { width: Math.max(1.4, size * 0.012), color: LINE, alpha: 0.8 * alpha(heat), closed: true,
      boil: 0.9, overshoot: 5, taper: [0.03, 0.06], pressure: 0.5 }, hash(n, i, k, 6)));
  }
}

export function emberPoints(list, size) {
  const pts = [];
  for (const g of list) {
    for (const c of g.contours) for (const [x, y] of resample([...c, c[0]], Math.max(3, size * 0.035))) pts.push([x, y]);
  }
  return pts;
}

function flight(o, x, y, i, t) {
  const arrive = o.t0 + (o.t1 - o.t0) * (0.35 + 0.65 * rnd(o.seed, i, 1));
  const leave = o.t0 - 0.05 + 0.1 * rnd(o.seed, i, 2);
  if (t < leave) return null;
  const u = ease.out(clamp((t - leave) / (arrive - leave)));
  const bend = 80 * signed(o.seed, i, 3) * Math.sin(Math.PI * u);
  return [o.from[0] + (x - o.from[0]) * u + bend, o.from[1] + (y - o.from[1]) * u - 60 * Math.sin(Math.PI * u), u];
}

export function emberText(ctx, pts, t, o) {
  const warm = clamp(o.heat(t));
  if (warm <= 0.01) return;
  ctx.save();
  ctx.lineCap = 'round';
  pts.forEach(([x, y], i) => {
    const now = flight(o, x, y, i, t);
    if (!now) return;
    const [px, py, u] = now;
    const was = flight(o, x, y, i, t - 1 / FPS) ?? now;
    const heat = warm * (0.7 + 0.3 * noise1(t * 9 + i * 0.37, o.seed)) * (u < 1 ? 1 : 0.85);
    const r = (o.dot ?? 3.4) * (0.75 + 0.5 * rnd(o.seed, i, 4));
    ctx.globalCompositeOperation = 'source-over';
    ctx.fillStyle = rgba(EMBER, Math.min(1, heat * 1.4));
    ctx.beginPath();
    ctx.arc(px, py, r * 0.9, 0, Math.PI * 2);
    ctx.fill();
    ctx.globalCompositeOperation = 'lighter';
    ctx.strokeStyle = rgba(heat > 0.6 ? CORE : MID, Math.min(1, heat * 1.2));
    ctx.lineWidth = r * 0.7;
    ctx.beginPath();
    ctx.moveTo(was[0], was[1]);
    ctx.lineTo(px + 0.01, py);
    ctx.stroke();
    ctx.fillStyle = rgba(OUTER, 0.22 * heat);
    ctx.beginPath();
    ctx.arc(px, py, r * 2.4, 0, Math.PI * 2);
    ctx.fill();
  });
  ctx.restore();
}
