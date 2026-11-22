// Warm paper with a faint tooth fixed to the sheet.
import { createCanvas } from '@napi-rs/canvas';
import { rnd } from './rand.js';
import { W, H, clamp } from './time.js';

export const PAPER = [243, 240, 232];
const TILE = 512;
let made = null;
const patterns = new WeakMap();

function periodic(x, y, cell, seed) {
  const n = TILE / cell;
  const i = Math.floor(x / cell);
  const j = Math.floor(y / cell);
  const fx = x / cell - i;
  const fy = y / cell - j;
  const s = (u) => u * u * (3 - 2 * u);
  const v = (a, b) => rnd(seed, ((a % n) + n) % n, ((b % n) + n) % n) * 2 - 1;
  const top = v(i, j) + (v(i + 1, j) - v(i, j)) * s(fx);
  const bot = v(i, j + 1) + (v(i + 1, j + 1) - v(i, j + 1)) * s(fx);
  return top + (bot - top) * s(fy);
}

function make() {
  const tile = createCanvas(TILE, TILE);
  const tctx = tile.getContext('2d');
  const img = tctx.createImageData(TILE, TILE);
  for (let y = 0; y < TILE; y++) {
    for (let x = 0; x < TILE; x++) {
      const v = 0.45 * periodic(x, y, 4, 3) + 0.3 * periodic(x, y, 2, 5) + 0.5 * (rnd(7, x, y) - 0.5);
      const p = (y * TILE + x) * 4;
      const dark = v > 0;
      img.data[p] = dark ? 120 : 255;
      img.data[p + 1] = dark ? 104 : 253;
      img.data[p + 2] = dark ? 78 : 246;
      img.data[p + 3] = clamp(Math.abs(v) * 0.06) * 255;
    }
  }
  tctx.putImageData(img, 0, 0);
  const vignette = createCanvas(W, H);
  const vctx = vignette.getContext('2d');
  const g = vctx.createRadialGradient(W / 2, H / 2, H * 0.35, W / 2, H / 2, W * 0.62);
  g.addColorStop(0, 'rgba(90,70,40,0)');
  g.addColorStop(1, 'rgba(90,70,40,0.07)');
  vctx.fillStyle = g;
  vctx.fillRect(0, 0, W, H);
  return { tile, vignette };
}

export function drawPaper(ctx, m) {
  made ??= make();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.globalAlpha = 1;
  ctx.fillStyle = `rgb(${PAPER.join(',')})`;
  ctx.fillRect(0, 0, W, H);
  const alpha = clamp((m.z - 0.3) / 0.5);
  if (alpha > 0) {
    if (!patterns.has(ctx)) patterns.set(ctx, ctx.createPattern(made.tile, 'repeat'));
    ctx.setTransform(m.a, m.b, m.c, m.d, m.e, m.f);
    ctx.globalAlpha = alpha;
    ctx.fillStyle = patterns.get(ctx);
    const det = m.a * m.d - m.b * m.c;
    const inv = (sx, sy) => [(m.d * (sx - m.e) - m.c * (sy - m.f)) / det, (-m.b * (sx - m.e) + m.a * (sy - m.f)) / det];
    const cs = [inv(0, 0), inv(W, 0), inv(0, H), inv(W, H)];
    const x0 = Math.min(...cs.map((p) => p[0]));
    const y0 = Math.min(...cs.map((p) => p[1]));
    ctx.fillRect(x0, y0, Math.max(...cs.map((p) => p[0])) - x0, Math.max(...cs.map((p) => p[1])) - y0);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
  }
  ctx.globalAlpha = 1;
  ctx.drawImage(made.vignette, 0, 0);
}
