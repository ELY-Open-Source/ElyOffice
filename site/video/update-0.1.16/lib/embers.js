// Embers and spark bursts: positions are pure functions of time.
import { noise1, rnd, signed } from './rand.js';
import { FPS } from './time.js';
import { rgba } from './stroke.js';

const HOT = [255, 236, 170];
const WARM = [255, 150, 54];

function speck(ctx, x, y, vx, vy, heat, size) {
  const k = 1 / FPS;
  const c = heat > 0.55 ? HOT : WARM;
  ctx.strokeStyle = rgba(c, Math.min(1, heat * 1.2));
  ctx.lineWidth = size;
  ctx.lineCap = 'round';
  ctx.beginPath();
  ctx.moveTo(x - vx * k * 0.8, y - vy * k * 0.8);
  ctx.lineTo(x, y);
  ctx.stroke();
  ctx.fillStyle = rgba(WARM, 0.18 * heat);
  ctx.beginPath();
  ctx.arc(x, y, size * 3.2, 0, Math.PI * 2);
  ctx.fill();
}

export function embers(ctx, t, flameAt, o = {}) {
  const life = 1.5;
  const k1 = Math.floor(t * FPS);
  ctx.save();
  ctx.globalCompositeOperation = 'lighter';
  for (let k = Math.floor((t - life) * FPS); k <= k1; k++) {
    const b = (k + rnd(201, k)) / FPS;
    const a = t - b;
    if (a < 0) continue;
    const s = flameAt(b);
    if (!s || s.alpha < 0.05) continue;
    const rate = (1.5 + 9 * s.energy + 14 * s.roar) * (1 - s.ember * 0.7) * s.sparks * (o.rate ?? 1);
    if (rnd(202, k) > rate / FPS) continue;
    const span = life * (0.45 + 0.55 * rnd(203, k));
    if (a > span) continue;
    const h = s.h * (1 + 0.6 * s.roar) * (1 - 0.86 * s.ember);
    const x0 = s.x + signed(204, k) * h * 0.22;
    const y0 = s.y - h * (0.45 + 0.4 * rnd(205, k));
    const vx = signed(206, k) * 45;
    const vy = -(70 + 120 * rnd(207, k));
    const x = x0 + vx * a + 22 * noise1(a * 2.2 + k, 208);
    const y = y0 + vy * a + 18 * a * a;
    const heat = (1 - a / span) * (0.75 + 0.25 * noise1(t * 14 + k, 209));
    speck(ctx, x, y, vx, vy + 36 * a, heat, 1.4 + 1.6 * rnd(210, k));
  }
  ctx.restore();
}

export function burst(ctx, t, o) {
  const a = t - o.t0;
  if (a < 0) return;
  ctx.save();
  ctx.globalCompositeOperation = 'lighter';
  for (let i = 0; i < o.count; i++) {
    const span = (o.life ?? 0.7) * (0.4 + 0.6 * rnd(o.seed, i, 1));
    if (a > span) continue;
    const ang = (o.angle ?? -Math.PI / 2) + signed(o.seed, i, 2) * (o.spread ?? Math.PI);
    const sp = (o.speed ?? 400) * (0.3 + 0.7 * rnd(o.seed, i, 3));
    const drag = Math.exp(-(o.drag ?? 2.2) * a);
    const reach = (1 - drag) / (o.drag ?? 2.2);
    const vx = Math.cos(ang) * sp;
    const vy = Math.sin(ang) * sp;
    const g = o.gravity ?? 260;
    const x = o.x + vx * reach;
    const y = o.y + vy * reach + 0.5 * g * a * a;
    const heat = (1 - a / span) ** 0.7;
    speck(ctx, x, y, vx * drag, vy * drag + g * a, heat, (o.size ?? 2.2) * (0.6 + 0.8 * rnd(o.seed, i, 4)));
  }
  ctx.restore();
}
