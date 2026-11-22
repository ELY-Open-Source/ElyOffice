// Strokes the line in width and colour buckets, culled.
import { APPS } from './path.js';
import { PAPER } from './paper.js';
import { W, H, clamp, smooth } from './time.js';

export const INK = [28, 31, 38];
export const COLOURS = {
  suite: INK, doc: [59, 111, 224], table: [30, 158, 97], show: [232, 105, 46],
  pdf: [213, 65, 58], mail: [122, 90, 240], tree: [201, 138, 26], mark: [42, 155, 176],
};
const RGB = APPS.map((a) => COLOURS[a]);
export const WET = 1.2;
const MIN = 0.7;
const WQ = 0.12;
const MQ = 12;
const AQ = 16;

function style(c, m, a) {
  const rgb = RGB[c].map((v, i) => v + (INK[i] - v) * m);
  return `rgb(${rgb.map((v, i) => Math.round(PAPER[i] + (v - PAPER[i]) * a)).join(',')})`;
}

export function wetMix(age) {
  return age >= WET ? 1 : smooth(clamp(age / WET));
}

export function drawLine(ctx, blocks, m, t) {
  const buckets = new Map();
  const pad = 12;
  let px = null;
  let py = null;
  let pkey = -1;
  let run = null;
  let index = 0;
  for (const b of blocks) {
    for (let i = 0; i < b.x.length; i++, index++) {
      const sx = m.a * b.x[i] + m.c * b.y[i] + m.e;
      const sy = m.b * b.x[i] + m.d * b.y[i] + m.f;
      if (px !== null) {
        const off = (sx < -pad && px < -pad) || (sx > W + pad && px > W + pad) || (sy < -pad && py < -pad) || (sy > H + pad && py > H + pad);
        if (off) pkey = -1;
        else {
          const wd = Math.max(MIN, b.wd[i] * m.z);
          const mix = RGB[b.c[i]] === INK ? MQ : Math.round(wetMix(t - b.tau[i]) * MQ);
          const key = ((b.c[i] * (MQ + 1) + mix) * (AQ + 1) + Math.round(clamp(b.a[i]) * AQ)) * 4096 + Math.round(wd / WQ);
          if (key !== pkey) {
            let list = buckets.get(key);
            if (!list) buckets.set(key, (list = []));
            run = [px, py];
            list.push(run);
            pkey = key;
          }
          run.push(sx, sy);
        }
      }
      px = sx;
      py = sy;
    }
  }
  const order = [...buckets.keys()].sort((p, q) => Math.floor(p / 4096) % (AQ + 1) - Math.floor(q / 4096) % (AQ + 1) || p - q);
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  for (const key of order) {
    const wq = key % 4096;
    const rest = Math.floor(key / 4096);
    const a = (rest % (AQ + 1)) / AQ;
    const cm = Math.floor(rest / (AQ + 1));
    ctx.strokeStyle = style(Math.floor(cm / (MQ + 1)), (cm % (MQ + 1)) / MQ, a);
    ctx.lineWidth = wq * WQ;
    ctx.beginPath();
    for (const r of buckets.get(key)) {
      ctx.moveTo(r[0], r[1]);
      for (let j = 2; j < r.length; j += 2) ctx.lineTo(r[j], r[j + 1]);
    }
    ctx.stroke();
  }
  return index;
}

export function drawBead(ctx, at, m, app, r) {
  if (r <= 0) return;
  const sx = m.a * at[0] + m.c * at[1] + m.e;
  const sy = m.b * at[0] + m.d * at[1] + m.f;
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.fillStyle = style(APPS.indexOf(app), 0, 1);
  ctx.beginPath();
  ctx.arc(sx, sy, r, 0, Math.PI * 2);
  ctx.fill();
}
