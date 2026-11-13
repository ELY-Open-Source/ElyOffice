// Drawn colour for fire: flat tones, tooth, hatching, ink rims.
import { createCanvas } from '@napi-rs/canvas';
import { paper } from './paper.js';
import { hatch } from './fill.js';
import { draw as stroke, rgba } from './stroke.js';
import { signed, hash } from './rand.js';
import { W, H, clamp } from './time.js';

export const OUTER = [236, 86, 34];
export const MID = [255, 158, 46];
export const CORE = [255, 238, 176];
export const LINE = [92, 24, 10];
export const SHADE = [168, 30, 12];

let layer = null;
let mask = null;

function toothMask() {
  if (mask) return mask;
  const { h } = paper();
  const c = createCanvas(W, H).getContext('2d');
  const img = c.createImageData(W, H);
  for (let i = 0; i < W * H; i++) {
    const v = clamp((-0.22 - h[i]) / 0.22);
    img.data[i * 4 + 3] = 255 * v * v * (3 - 2 * v);
  }
  c.putImageData(img, 0, 0);
  mask = c;
  return mask;
}

export function bounds(parts, pad) {
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const p of parts) for (const [x, y] of p) {
    x0 = Math.min(x0, x);
    y0 = Math.min(y0, y);
    x1 = Math.max(x1, x);
    y1 = Math.max(y1, y);
  }
  const bx = Math.max(0, Math.floor(x0 - pad));
  const by = Math.max(0, Math.floor(y0 - pad));
  return { x: bx, y: by, w: Math.max(1, Math.min(W, Math.ceil(x1 + pad)) - bx), h: Math.max(1, Math.min(H, Math.ceil(y1 + pad)) - by) };
}

export function sheet(box) {
  layer ??= createCanvas(W, H).getContext('2d');
  layer.clearRect(box.x, box.y, box.w, box.h);
  return layer;
}

export function grain(ctx, box, amount) {
  if (box.w < 1 || box.h < 1) return;
  ctx.save();
  ctx.globalCompositeOperation = 'destination-out';
  ctx.globalAlpha = amount;
  ctx.drawImage(toothMask().canvas, box.x, box.y, box.w, box.h, box.x, box.y, box.w, box.h);
  ctx.restore();
}

export function settle(ctx, box) {
  if (box.w < 1 || box.h < 1) return;
  ctx.drawImage(layer.canvas, box.x, box.y, box.w, box.h, box.x, box.y, box.w, box.h);
}

export function path(ctx, parts) {
  ctx.beginPath();
  for (const poly of parts) {
    poly.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
    ctx.closePath();
  }
}

export function tone(ctx, parts, color, alpha, wobble, seed) {
  const rough = parts.map((p, k) => p.map(([x, y], i) => [
    x + wobble * signed(seed, k, i >> 1, 1), y + wobble * signed(seed, k, i >> 1, 2)]));
  ctx.fillStyle = rgba(color, alpha);
  path(ctx, rough);
  ctx.fill('nonzero');
}

export function shade(ctx, parts, strip, o, seed) {
  ctx.save();
  path(ctx, parts);
  ctx.clip('nonzero');
  hatch(ctx, [strip], { style: 'charcoal', gap: o.gap, width: o.width, color: o.color, alpha: o.alpha, angle: o.angle }, seed);
  ctx.restore();
}

export function rims(ctx, parts, o, seed) {
  parts.forEach((poly, k) => {
    ctx.save();
    parts.forEach((other, j) => {
      if (j === k) return;
      ctx.beginPath();
      ctx.rect(0, 0, W, H);
      other.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
      ctx.closePath();
      ctx.clip('evenodd');
    });
    rim(ctx, poly, { ...o, width: o.width * (k ? 0.8 : 1) }, hash(seed, k));
    ctx.restore();
  });
}

export function rim(ctx, poly, o, seed) {
  const ring = poly.filter((_, i) => i % 2 === 0);
  stroke(ctx, ring, { width: o.width, color: o.color, alpha: o.alpha, closed: true, boil: o.boil ?? 1.2,
    overshoot: o.width * 4, taper: [0.03, 0.06], pressure: 0.55 }, hash(seed, 1));
  const from = 0.15 + 0.5 * (signed(seed, 2) * 0.5 + 0.5);
  stroke(ctx, ring, { width: o.width * 0.55, color: o.color, alpha: o.alpha * 0.7, from, upto: from + 0.28,
    boil: (o.boil ?? 1.2) * 1.8, overshoot: o.width * 3, pressure: 0.5 }, hash(seed, 3));
}
