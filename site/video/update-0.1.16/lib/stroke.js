// Boiling strokes: resample, jitter per drawing, press, overshoot.
import { noise1, rnd, signed, hash } from './rand.js';
import { smooth, clamp } from './time.js';

export const INK = [30, 23, 20];
export const CHARCOAL = [46, 40, 36];

export function length(pts) {
  let len = 0;
  for (let i = 1; i < pts.length; i++) len += Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
  return len;
}

export function resample(pts, step) {
  const out = [[pts[0][0], pts[0][1], 0]];
  let s = 0;
  let next = step;
  for (let i = 1; i < pts.length; i++) {
    const [ax, ay] = pts[i - 1];
    const [bx, by] = pts[i];
    const seg = Math.hypot(bx - ax, by - ay);
    while (seg > 0 && next <= s + seg) {
      const u = (next - s) / seg;
      out.push([ax + (bx - ax) * u, ay + (by - ay) * u, next]);
      next += step;
    }
    s += seg;
  }
  const last = pts[pts.length - 1];
  if (s - out[out.length - 1][2] > step * 0.25) out.push([last[0], last[1], s]);
  return out;
}

function extend(pts, before, after) {
  const n = pts.length;
  const [ax, ay] = pts[0];
  const [bx, by] = pts[Math.min(2, n - 1)];
  const [cx, cy] = pts[n - 1];
  const [dx, dy] = pts[Math.max(n - 3, 0)];
  const a = Math.hypot(ax - bx, ay - by) || 1;
  const c = Math.hypot(cx - dx, cy - dy) || 1;
  return [
    [ax + ((ax - bx) / a) * before, ay + ((ay - by) / a) * before],
    ...pts,
    [cx + ((cx - dx) / c) * after, cy + ((cy - dy) / c) * after],
  ];
}

export function boil(points, o, seed) {
  const pts = o.closed ? [...points, points[0], points[1] ?? points[0]] : points;
  const raw = length(pts);
  if (raw < 0.5) return [];
  const over = o.overshoot ?? Math.min(o.width * 2.2, raw * 0.12);
  const before = over * (0.25 + 0.75 * rnd(seed, 1));
  const after = over * (0.25 + 0.75 * rnd(seed, 2));
  const path = over > 0 ? extend(pts, before, after) : pts;
  const len = raw + (over > 0 ? before + after : 0);
  const step = clamp(o.step ?? Math.min(len / 6, o.width * 0.9, 4), 0.6, 6);
  const res = resample(path, step);
  const amp = o.boil ?? Math.min(1.7, 0.35 + o.width * 0.32);
  const wave = o.wave ?? Math.max(18, Math.min(110, len * 0.35));
  const sa = hash(seed, 3);
  const sb = hash(seed, 4);
  const sp = hash(seed, 5);
  const tx = signed(seed, 6) * amp * 0.6;
  const ty = signed(seed, 7) * amp * 0.6;
  const rot = (signed(seed, 8) * amp * 0.5) / Math.max(len, 1);
  const cx = res[res.length >> 1][0];
  const cy = res[res.length >> 1][1];
  const tin = Math.max(len * (o.taper?.[0] ?? 0.1), o.width * 0.8);
  const tout = Math.max(len * (o.taper?.[1] ?? 0.22), o.width * 1.2);
  const press = o.pressure ?? 0.28;
  const out = [];
  for (let i = 0; i < res.length; i++) {
    const [x, y, s] = res[i];
    const p = res[Math.max(i - 1, 0)];
    const q = res[Math.min(i + 1, res.length - 1)];
    let nx = -(q[1] - p[1]);
    let ny = q[0] - p[0];
    const nl = Math.hypot(nx, ny) || 1;
    nx /= nl;
    ny /= nl;
    const d = amp * (0.7 * noise1(s / wave, sa) + 0.3 * noise1(s / (wave * 0.31), sb));
    const rx = x - cx;
    const ry = y - cy;
    const w = o.width * smooth(clamp(s / tin)) * (0.15 + 0.85 * smooth(clamp((len - s) / tout)))
      * (1 + press * noise1(s / 70, sp));
    out.push({
      x: x + nx * d + tx - ry * rot,
      y: y + ny * d + ty + rx * rot,
      w: Math.max(w, 0),
      s: s / len,
      nx,
      ny,
    });
  }
  return out;
}

export function cut(pts, from = 0, upto = 1) {
  if (from <= 0 && upto >= 1) return pts;
  return pts.filter((p) => p.s >= from && p.s <= upto);
}

function ribbon(ctx, pts, side) {
  ctx.beginPath();
  for (let i = 0; i < pts.length; i++) {
    const p = pts[i];
    const k = (p.w / 2) * side;
    if (i === 0) ctx.moveTo(p.x + p.nx * k, p.y + p.ny * k);
    else ctx.lineTo(p.x + p.nx * k, p.y + p.ny * k);
  }
  for (let i = pts.length - 1; i >= 0; i--) {
    const p = pts[i];
    const k = (p.w / 2) * side;
    ctx.lineTo(p.x - p.nx * k, p.y - p.ny * k);
  }
  ctx.closePath();
  ctx.fill();
}

export const rgba = (c, a) => `rgba(${c[0] | 0},${c[1] | 0},${c[2] | 0},${a})`;

function ink(ctx, pts, o) {
  ctx.fillStyle = rgba(o.color ?? INK, o.alpha ?? 0.93);
  ribbon(ctx, pts, 1);
  const head = pts[pts.length - 1];
  if (o.head && head.w > 1) {
    ctx.beginPath();
    ctx.arc(head.x, head.y, head.w / 2, 0, Math.PI * 2);
    ctx.fill();
  }
}

function charcoal(ctx, pts, o, seed) {
  const color = o.color ?? CHARCOAL;
  const base = o.alpha ?? 0.9;
  for (let k = 0; k < 4; k++) {
    const off = ((k - 1.5) / 1.5) * 0.32;
    const sk = hash(seed, 40 + k);
    const thin = 0.42 + 0.2 * rnd(sk, 1);
    ctx.fillStyle = rgba(color, base * (0.26 + 0.12 * rnd(sk, 2)));
    let run = [];
    for (let i = 0; i < pts.length; i++) {
      const p = pts[i];
      const dry = noise1(p.s * 60 + rnd(sk, 3) * 9, sk) < -0.5 + 0.2 * k / 3;
      if (dry) {
        if (run.length > 1) ribbon(ctx, run, 1);
        run = [];
        continue;
      }
      const j = off + 0.06 * noise1(p.s * 24, sk);
      run.push({ x: p.x + p.nx * p.w * j, y: p.y + p.ny * p.w * j, w: p.w * thin, nx: p.nx, ny: p.ny });
    }
    if (run.length > 1) ribbon(ctx, run, 1);
  }
}

export function draw(ctx, points, o, seed) {
  if (points.length < 2) return;
  const pts = cut(boil(points, o, seed), o.from, o.upto);
  if (pts.length < 2) return;
  if (o.style === 'charcoal') charcoal(ctx, pts, o, seed);
  else ink(ctx, pts, o);
}
