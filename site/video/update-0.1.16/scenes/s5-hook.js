// Ink hooks that grow, snag, and burn away.
import { spline } from '../lib/ink.js';
import { resample } from '../lib/stroke.js';
import { noise1, rnd, signed } from '../lib/rand.js';
import { keys, clamp, span } from '../lib/time.js';
import { drawFlame } from '../lib/flame.js';
import { burst } from '../lib/embers.js';

const BURNT = [62, 30, 18];
const HOT = [255, 132, 40];
const WHITE = [255, 230, 170];
const twos = (t) => Math.floor(t * 12 + 1e-6) / 12;

function curl([tx, ty], side, hw, gh) {
  return [[tx - side * (hw + 15), ty - gh * 0.85], [tx - side * (hw + 13), ty - gh * 0.05], [tx - side * hw * 0.3, ty + 19],
    [tx + side * (hw + 9), ty + 11], [tx + side * (hw + 13), ty - 16]];
}

export function grown(h, t) {
  const [a, b] = h.grow;
  return keys(t, [[a, 0], [a + (b - a) * 0.55, 0.7, 'out'], [a + (b - a) * 0.75, 0.68], [b, 1, 'in']]);
}

export function pull(h, t) {
  if (t < h.snag) return 0;
  if (t < h.land + h.burn) {
    return keys(t, [[h.snag, 0], [h.snag + 0.12, 1, 'snap'], [h.snag + 0.3, 0.78], [h.snag + 0.5, 1.08], [h.snag + 0.75, 0.86], [h.land, 1.12]])
      + 0.06 * noise1(t * 9, h.seed);
  }
  const tau = t - h.land - h.burn;
  return 1.12 * Math.exp(-7 * tau) * Math.cos(15 * tau);
}

// The hook's points; t freezes once burning.
export function points(h, t) {
  const tt = t >= h.land ? h.land : t;
  const taut = smoothTaut(h, tt);
  const p = pull(h, Math.min(t, h.land + h.burn - 1e-3));
  const off = [h.yank[0] * p, h.yank[1] * p];
  const via = h.via.map(([x, y], i) => {
    const w = (i + 1) / (h.via.length + 1);
    const amp = 26 * w * (1 - 0.85 * taut);
    return [x + amp * noise1(tt * 1.7 + i * 0.7, h.seed) + off[0] * w * w,
      y + amp * noise1(tt * 1.5 + i * 0.9, h.seed + 1) + off[1] * w * w];
  });
  const end = curl([h.target[0] + off[0], h.target[1] + off[1]], h.side, h.hw, h.gh);
  const [rx, ry] = h.root();
  return spline([[rx, ry], [rx + 2, ry + 16], ...via, ...end], 10);
}

const smoothTaut = (h, t) => span(t, h.snag - 0.1, h.snag + 0.15);

export function fronts(h, t) {
  if (t < h.land) return null;
  const r = clamp((t - h.land) / h.burn);
  return { r, a: h.u0 * (1 - r), b: h.u0 + (1 - h.u0) * r };
}

function at(res, u) {
  const L = res[res.length - 1][2];
  const s = clamp(u) * L;
  let k = 1;
  while (k < res.length - 1 && res[k][2] < s) k++;
  return [res[k][0], res[k][1], Math.atan2(res[k][1] - res[k - 1][1], res[k][0] - res[k - 1][0])];
}

function barb(ink, pts, o) {
  const n = pts.length;
  const [x, y] = pts[n - 1];
  const [px, py] = pts[n - 4];
  const d = Math.hypot(x - px, y - py) || 1;
  const [ux, uy] = [(x - px) / d, (y - py) / d];
  ink.line([[x, y], [x - ux * 17 + uy * 12 * o.side, y - uy * 17 - ux * 12 * o.side]], { id: o.id, width: 4.4, overshoot: 2, boil: 0.8, taper: [0.05, 0.6] });
}

function ash(ink, res, h, t, f) {
  const L = res[res.length - 1][2];
  const step = 22;
  for (let s = 0, k = 0; s < L; s += step, k++) {
    const c = (s + step / 2) / L;
    if (c < f.a || c > f.b) continue;
    const reach = c < h.u0 ? (h.u0 - c) / h.u0 : (c - h.u0) / (1 - h.u0 || 1);
    const tb = h.land + h.burn * reach;
    const a = t - tb - 0.1 - 0.15 * rnd(h.seed, k, 1);
    const piece = res.filter((p) => p[2] >= s && p[2] <= s + step * (1 - 0.35 * clamp(a * 2)));
    if (piece.length < 2 || a > 0.9) continue;
    const [cx, cy] = piece[piece.length >> 1];
    const fall = a > 0 ? a : 0;
    const rot = signed(h.seed, k, 2) * 3 * fall;
    const dx = signed(h.seed, k, 3) * 40 * fall;
    const dy = 0.5 * 900 * fall * fall;
    const q = piece.map(([x, y]) => {
      const rx = x - cx;
      const ry = y - cy;
      return [cx + dx + rx * Math.cos(rot) - ry * Math.sin(rot), cy + dy + rx * Math.sin(rot) + ry * Math.cos(rot)];
    });
    ink.line(q, { style: 'charcoal', width: 5.5 * (1 - 0.4 * clamp(fall * 1.4)), color: BURNT,
      alpha: 0.9 * (1 - clamp(fall / 0.9)), overshoot: 0, boil: 1.2, taper: [0.2, 0.2] });
  }
}

// Draws the hooks of one attack, lifted off the page.
export function drawHooks(ink, list, t) {
  const live = list.filter((h) => t >= h.grow[0]);
  if (!live.length) return;
  const [ax, ay] = live[0].via[live[0].via.length >> 1];
  ink.lift(16, [ax, ay], (k) => {
    for (const h of live) {
      const pts = points(h, t);
      const res = resample(pts, 3);
      const f = fronts(h, t);
      const o = { id: `${h.seed}-ink`, width: 7.5, taper: [0.02, 0.4], overshoot: 0, boil: 1.4, pressure: 0.4 };
      const echo = { id: `${h.seed}-dry`, style: 'charcoal', width: 11, alpha: 0.32, taper: [0.05, 0.5], overshoot: 0, boil: 2 };
      if (!f) {
        const g = grown(h, t);
        k.line(pts, { ...o, upto: g, head: true });
        k.line(pts, { ...echo, upto: g });
        if (g > 0.985) barb(k, pts, { side: h.side, id: `${h.seed}-barb` });
        continue;
      }
      if (f.a > 0.01) {
        k.line(pts, { ...o, upto: f.a });
        k.line(pts, { ...echo, upto: f.a });
      }
      if (f.b < 0.99) {
        k.line(pts, { ...o, from: f.b });
        barb(k, pts, { side: h.side, id: `${h.seed}-barb` });
      }
      ash(k, res, h, t, f);
    }
  });
}

export function scorchAt(ink, h, t) {
  if (t < h.land + h.burn) return;
  const [x, y] = h.root();
  ink.smudge(x, y + 10, 14, { color: BURNT, alpha: 0.18 });
}

// Running fire on the fronts, on ones.
export function fireHooks(ctx, list, t, n) {
  const tb = twos(t);
  for (const h of list) sparks(ctx, h, t);
  for (const h of list) {
    const f = fronts(h, tb);
    if (!f || f.r >= 1) continue;
    const res = resample(points(h, tb), 3);
    for (const [u, dir] of [[f.a, -1], [f.b, 1]]) {
      const [x, y, ang] = at(res, u);
      const vx = Math.cos(ang) * dir;
      drawFlame(ctx, { x, y: y + 5, h: 38, energy: 1, lean: clamp(-vx * 0.5, -0.6, 0.6),
        glow: 0.7, pool: 0 }, t + h.seed, n + h.seed);
    }
    ctx.save();
    ctx.globalCompositeOperation = 'lighter';
    ctx.lineCap = 'round';
    for (const [u0, u1] of [[f.a, Math.min(h.u0, f.a + 0.07)], [Math.max(h.u0, f.b - 0.07), f.b]]) {
      const seg = res.filter((p) => p[2] >= u0 * res[res.length - 1][2] && p[2] <= u1 * res[res.length - 1][2]);
      if (seg.length < 2) continue;
      for (const [c, w, a] of [[HOT, 8, 0.75], [WHITE, 3, 0.8]]) {
        ctx.strokeStyle = `rgba(${c},${a})`;
        ctx.lineWidth = w;
        ctx.beginPath();
        seg.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
        ctx.stroke();
      }
    }
    ctx.restore();
  }
}

function sparks(ctx, h, t) {
  if (t < h.land || t > h.land + h.burn + 0.6) return;
  const res = resample(points(h, h.land), 6);
  for (let k = 0; k * 0.08 < h.burn; k++) {
    const t0 = h.land + k * 0.08;
    const g = fronts(h, t0);
    for (const [u, s] of [[g.a, 1], [g.b, 2]]) {
      const [x, y] = at(res, u);
      burst(ctx, t, { t0, x, y, count: 5, speed: 150, life: 0.5, gravity: -60, seed: h.seed * 31 + k * 3 + s, size: 1.6 });
    }
  }
}

export function hookLights(list, t) {
  const out = [];
  const tb = twos(t);
  for (const h of list) {
    const f = fronts(h, tb);
    if (!f || f.r >= 1) continue;
    const res = resample(points(h, tb), 6);
    for (const u of [f.a, f.b]) {
      const [x, y] = at(res, u);
      out.push({ x, y: y - 12, r: 230, i: 0.42 });
    }
  }
  return out;
}

export const hookAt = (h, t, u) => at(resample(points(h, t), 3), u);
