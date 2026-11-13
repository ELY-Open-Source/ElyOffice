// The flame: one character, shape only, drawn on ones.
import { noise1, rnd, signed, hash } from './rand.js';
import { smooth, clamp, FPS } from './time.js';
import { draw as stroke, rgba } from './stroke.js';
import { bounds, sheet, grain, settle, path, tone, shade, rim, rims, OUTER, MID, CORE, LINE, SHADE } from './paint.js';

export const REST = Object.freeze({
  x: 960, y: 610, h: 140, lean: 0, squash: 1, energy: 0.5, curl: 0,
  ember: 0, roar: 0, alpha: 1, pool: 520, glow: 1, sparks: 1,
});

export const rested = (s) => ({ ...REST, ...s });


function dims(s) {
  const e = s.ember;
  const r = s.roar;
  return {
    hh: s.h * (1 + 0.6 * r) * (1 - 0.86 * e) * s.squash,
    ww: s.h * 0.62 * (1 + 0.22 * r) * (1 - 0.55 * e) / s.squash ** 0.65,
    en: (s.energy + 0.9 * r) * (1 - e),
  };
}

function half(u) {
  const ub = 0.28;
  if (u < ub) return Math.sqrt(Math.max(0, 1 - ((ub - u) / ub) ** 2));
  const v = (u - ub) / (1 - ub);
  return (1 - v) ** 1.2 * (1 + 0.18 * Math.sin(Math.PI * v));
}

function tongue(bx, by, hh, ww, p, t, n) {
  const N = 26;
  const spine = [];
  let x = bx;
  let y = by;
  for (let i = 0; i <= N; i++) {
    const u = i / N;
    const phi = -Math.PI / 2 + p.lean * (0.35 + 0.9 * u)
      + p.curl * 3.5 * smooth(clamp((u - 0.32) / 0.68)) ** 1.3
      + p.en * 0.42 * u * noise1(t * 2.4 - u * 1.3, p.seed);
    const w = (ww / 2) * half(u) * (1 - 0.4 * Math.abs(p.curl));
    spine.push([x, y, Math.cos(phi), Math.sin(phi), w]);
    x += (Math.cos(phi) * hh) / N;
    y += (Math.sin(phi) * hh) / N;
  }
  const side = (sg) => spine.map(([sx, sy, cx, cy, hw], i) => {
    const u = i / N;
    const lick = 1 + p.en * 0.34 * u ** 1.3
      * Math.sin(2 * Math.PI * (1.6 * u - 2.2 * t) + sg * 1.9 + (p.seed % 7))
      + 0.04 * signed(n, p.seed, i >> 1, sg);
    const w = hw * Math.max(lick, 0.2);
    return [sx - cy * w * sg, sy + cx * w * sg];
  });
  const left = side(1);
  const right = side(-1).reverse();
  return { poly: [...left, ...right], tip: spine[N], spine };
}

function shape(s, t, n, k) {
  const { hh, ww, en } = dims(s);
  const lean = s.lean;
  const parts = [];
  const main = tongue(s.x, s.y, hh * k.h, ww * k.w, { lean, curl: s.curl, en: en * k.e, seed: hash(k.seed, 1) }, t, n);
  parts.push(main.poly);
  if (k.tongues) {
    const keep = 1 - smooth(clamp(s.curl * 1.6));
    for (const sg of [-1, 1]) {
      const wob = (sg < 0 ? 0.5 : 0.6) + 0.2 * noise1(t * 2.7 + sg * 5, hash(k.seed, sg)) + 0.1 * en;
      const p = {
        lean: lean * 1.1 + sg * (0.3 + 0.16 * noise1(t * 2.2, sg + 9)),
        curl: -sg * (0.1 + 0.08 * noise1(t * 1.7, sg + 13)), en, seed: hash(k.seed, sg, 2),
      };
      const side = tongue(s.x + sg * ww * 0.17, s.y - hh * 0.1, hh * wob * keep, ww * 0.56 * keep, p, t + sg * 0.37, n);
      if (keep > 0.05) parts.push(side.poly);
    }
  }
  return { parts, tip: main.tip, spine: main.spine, hh, ww, en };
}

function brush(ctx, body, n, alpha) {
  ctx.save();
  path(ctx, body.parts);
  ctx.clip('nonzero');
  const sp = body.spine;
  for (let j = 0; j < 7; j++) {
    const off = (j / 6 - 0.5) * 1.5 + 0.12 * signed(n, j, 1);
    const top = 0.55 + 0.4 * rnd(n, j, 2) - Math.abs(off) * 0.3;
    const pts = [];
    for (let i = 1; i < sp.length * top; i++) {
      const [x, y, cx, cy, w] = sp[i];
      pts.push([x - cy * w * off, y + cx * w * off]);
    }
    if (pts.length < 3) continue;
    const warm = j % 3 === 1 ? [255, 196, 92] : j % 3 ? [214, 58, 26] : [250, 120, 40];
    stroke(ctx, pts, { width: body.ww * (0.06 + 0.05 * rnd(n, j, 3)), color: warm, alpha: 0.75 * alpha, taper: [0.3, 0.5], boil: 1.6,
      style: j % 2 ? 'charcoal' : 'ink' }, hash(n, 300, j));
  }
  ctx.restore();
}

function flicks(ctx, s, t, tip, hh, en) {
  const rate = 3 + 5 * en;
  const k0 = Math.floor(t * rate);
  for (let k = k0 - 2; k <= k0; k++) {
    const b = (k + 0.6 * rnd(97, k)) / rate;
    const a = (t - b) / 0.3;
    if (a < 0 || a > 1 || rnd(98, k) > 0.75) continue;
    const size = hh * 0.13 * (1 - a) * (0.7 + 0.6 * rnd(99, k));
    const x = tip[0] + signed(100, k) * hh * 0.08 + Math.sin(a * 5 + k) * hh * 0.04;
    const y = tip[1] - a * hh * 0.55 + size;
    const bend = size * 0.9 * signed(101, k);
    ctx.beginPath();
    ctx.moveTo(x, y);
    ctx.quadraticCurveTo(x + size * 0.55, y - size * 1.1, x + bend, y - size * 3);
    ctx.quadraticCurveTo(x - size * 0.35, y - size * 1.2, x, y);
    ctx.fillStyle = rgba(a < 0.5 ? MID : OUTER, 0.9 * s.alpha);
    ctx.fill();
  }
}

function blob(x, y, r, seed, n) {
  const pts = [];
  for (let i = 0; i < 14; i++) {
    const q = (i / 14) * Math.PI * 2;
    const k = r * (1 + 0.16 * signed(n, seed, i));
    pts.push([x + Math.cos(q) * k, y + Math.sin(q) * k * 0.86]);
  }
  return pts;
}

function coal(L, s, t, n) {
  const pulse = 1 + 0.18 * Math.sin(t * 6.3) + 0.06 * noise1(t * 11, 7);
  const r = s.h * 0.11 * pulse * smooth(s.ember) * (0.8 + 0.35 * s.glow);
  const cy = s.y - r;
  const outer = blob(s.x, cy, r * 1.5, 1, n);
  tone(L, [outer], OUTER, 0.95 * s.alpha, 0.6, hash(n, 31));
  tone(L, [blob(s.x, cy, r, 2, n)], MID, 0.95 * s.alpha, 0.5, hash(n, 32));
  tone(L, [blob(s.x - r * 0.1, cy - r * 0.1, r * 0.5, 3, n)], CORE, s.alpha, 0.4, hash(n, 33));
  return outer;
}

function glow(ctx, s, hh) {
  const cx = s.x;
  const cy = s.y - hh * 0.45;
  const r = Math.max(hh, s.h * 0.4) * (2.3 + 0.8 * s.roar);
  const g = ctx.createRadialGradient(cx, cy, 0, cx, cy, r);
  const a = 0.2 * s.glow * s.alpha * (1 + 0.6 * s.roar);
  g.addColorStop(0, rgba([255, 200, 120], a));
  g.addColorStop(0.35, rgba([255, 130, 50], a * 0.45));
  g.addColorStop(1, rgba([255, 90, 30], 0));
  ctx.fillStyle = g;
  ctx.fillRect(cx - r, cy - r, r * 2, r * 2);
}

function strip(body, sg) {
  const sp = body.spine;
  const outside = [];
  const inside = [];
  for (let i = 1; i < sp.length * 0.74; i++) {
    const [x, y, cx, cy, w] = sp[i];
    outside.push([x - cy * w * 1.2 * sg, y + cx * w * 1.2 * sg]);
    inside.push([x - cy * w * 0.22 * sg, y + cx * w * 0.22 * sg]);
  }
  return [...outside, ...inside.reverse()];
}

export function drawFlame(ctx, state, t, n) {
  const s = rested(state);
  if (s.alpha <= 0.002 || s.h <= 0.5) return;
  const outer = shape(s, t, n, { h: 1, w: 1, e: 1, seed: 11, tongues: true });
  ctx.save();
  ctx.globalCompositeOperation = 'lighter';
  glow(ctx, s, outer.hh);
  ctx.restore();
  const box = bounds([...outer.parts, [[outer.tip[0], outer.tip[1] - 0.62 * outer.hh]]], 24 + s.h * 0.3);
  const L = sheet(box);
  const a = s.alpha * (1 - smooth(clamp((s.ember - 0.6) / 0.4)));
  const lineW = Math.max(1.4, outer.hh * 0.022);
  if (a > 0.01) {
    tone(L, outer.parts, OUTER, 0.97 * a, 1.1, hash(n, 21));
    const sg = s.lean > 0.05 ? -1 : 1;
    shade(L, outer.parts, strip(outer, sg), { gap: Math.max(3.4, outer.hh * 0.034), width: Math.max(1.1, outer.hh * 0.013),
      color: SHADE, alpha: 0.95 * a, angle: -1.2 * sg }, hash(n, 22));
    brush(L, outer, n, a);
    const mid = shape({ ...s, y: s.y - outer.hh * 0.015 }, t + 0.05, n, { h: 0.78, w: 0.68, e: 0.8, seed: 12 });
    tone(L, mid.parts, MID, 0.96 * a, 0.9, hash(n, 23));
    const core = shape({ ...s, y: s.y - outer.hh * 0.05, lean: s.lean + 0.08 * noise1(t * 3, 21) }, t + 0.11, n, { h: 0.46, w: 0.4, e: 0.6, seed: 13 });
    tone(L, core.parts, CORE, 0.98 * a, 0.7, hash(n, 24));
    flicks(L, { ...s, alpha: a }, t, outer.tip, outer.hh, outer.en);
  }
  const ember = s.ember > 0.02 ? coal(L, s, t, n) : null;
  grain(L, box, 0.6);
  settle(ctx, box);
  if (a > 0.01) {
    rims(ctx, outer.parts, { width: lineW, color: LINE, alpha: 0.9 * a }, hash(n, 78));
  }
  if (ember) rim(ctx, ember, { width: Math.max(1.2, lineW * 0.8), color: LINE, alpha: 0.85 * s.alpha, boil: 0.6 }, hash(n, 80));
}

export function lightOf(state, t) {
  const s = rested(state);
  const { hh } = dims(s);
  const n = Math.round(t * FPS);
  const flicker = 1 + 0.045 * noise1(t * 8.5, 3) + 0.025 * noise1(t * 21, 5) + 0.012 * signed(n, 61);
  return {
    x: s.x,
    y: s.y - Math.max(hh, s.h * 0.1) * 0.45,
    r: s.pool * (1 - 0.62 * s.ember) * (1 + 0.22 * s.roar) * (1 + 0.02 * noise1(t * 6, 9)),
    i: s.alpha * s.glow * (1 + 0.3 * s.roar) * (1 - 0.5 * s.ember) * flicker,
    c: [1, 1, 1],
  };
}
