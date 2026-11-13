// s3 line two: tofu boxes burn into 思源宋体.
import { glyphs } from '../lib/glyph.js';
import { span, clamp, smooth, spring, ease } from '../lib/time.js';
import { noise1, rnd, signed } from '../lib/rand.js';
import { burst } from '../lib/embers.js';
import { draw as stroke } from '../lib/stroke.js';
import { SCORCH, BURNT, ring, clip, front, band, licks, glow } from './s3-burn.js';

const SIZE = 150;
const BASE = 800;
export const CATCH = [26.0, 26.5, 27.0, 27.5];
export const MARK = 28.0;
export const LEAP = 27.72;
const BURN = 0.4;
const MARK_BURN = 0.55;
const RED = [170, 46, 36];
const AMBER = [236, 176, 70];
const GREY = [150, 140, 128];

let made = null;
export function plan() {
  if (made) return made;
  const list = glyphs('思源宋体', { x: 960, y: BASE, size: SIZE, align: 'center' });
  const x0 = 960 - (SIZE * 4) / 2;
  const cells = list.map((g, k) => {
    const l = x0 + SIZE * k + 12;
    const r = l + 126;
    const rect = [[l, BASE - 128], [r, BASE - 128], [r, BASE + 18], [l, BASE + 18]];
    const at = [l + 3, BASE - 52];
    const reach = Math.max(...rect.map(([x, y]) => Math.hypot(x - at[0], y - at[1]))) * 0.92 + 18;
    const cx = (g.box.x0 + g.box.x1) / 2;
    const cy = (g.box.y0 + g.box.y1) / 2;
    return { g, rect, at, reach, c: [cx, cy], seed: 300 + k };
  });
  const tri = [[1340, 688], [1384, 770], [1296, 770]];
  const markAt = [1306, 766];
  made = { cells, tri, markAt, markReach: 110 };
  return made;
}

export const travel = (k) => 0.15 + 0.04 * k;

function scaled(g, u, [cx, cy]) {
  return g.contours.map((c) => c.map(([x, y]) => [cx + (x - cx) * u, cy + (y - cy) * u]));
}

const pop = (t, tk) => 1 + spring(t - tk - 0.1, -0.12, 14, 6.5);

function tofu(ink, cell, t, k) {
  const upto = span(t, 20.5 + 0.09 * k, 20.66 + 0.09 * k);
  if (upto <= 0) return;
  ink.wash([cell.rect], { color: GREY, alpha: 0.22 * upto, bleed: 3 });
  ink.line([...cell.rect, cell.rect[0]], { style: 'charcoal', width: 3.4, alpha: 0.9, upto, overshoot: 7 });
}

function cellDraw(ink, t, k) {
  const cell = plan().cells[k];
  const tk = CATCH[k];
  const rho = front(t, tk, BURN, cell.reach);
  const ctx = ink.ctx;
  const d = ink.drawing;
  if (rho < cell.reach) {
    ctx.save();
    if (rho > 0) clip(ctx, [ring(...cell.at, rho + 14, cell.seed, d)], true);
    tofu(ink, cell, t, k);
    ctx.restore();
  }
  if (rho <= 0) return;
  const fade = 1 - smooth(span(t, tk + 0.25, tk + 1.6));
  if (fade > 0) {
    ctx.save();
    clip(ctx, [ring(...cell.at, rho, cell.seed, d)], false);
    ink.wash([cell.rect], { color: SCORCH, alpha: 0.5 * fade, bleed: 14 });
    ink.wash([cell.rect], { color: BURNT, alpha: 0.18 * fade, bleed: 18 });
    ctx.restore();
  }
  ink.smudge(...cell.c, 70, { color: SCORCH, alpha: 0.16 * (1 - 0.5 * smooth(span(t, tk + 0.25, tk + 1.6))) * smooth(span(t, tk, tk + 0.3)), blots: 4, id: cell.seed });
  if (rho < cell.reach) {
    ctx.save();
    clip(ctx, [ring(...cell.at, rho + 22, cell.seed, d), ring(...cell.at, Math.max(0, rho - 12), cell.seed, d)], false);
    ink.wash([cell.rect], { color: BURNT, alpha: 0.9, bleed: 3 });
    ctx.restore();
  }
  ctx.save();
  clip(ctx, [ring(...cell.at, Math.max(0, rho - 6), cell.seed, d)], false);
  const g = { ...cell.g, contours: scaled(cell.g, pop(t, tk), cell.c) };
  ink.write([g], { size: SIZE, fill: 'hatch', alpha: 0.95, color: [34, 22, 18] });
  ctx.restore();
}

function markDraw(ink, t) {
  const { tri, markAt, markReach } = plan();
  const upto = span(t, 20.86, 21.0);
  if (upto <= 0) return;
  const rho = front(t, MARK, MARK_BURN, markReach);
  const ctx = ink.ctx;
  const d = ink.drawing;
  if (rho < markReach) {
    ctx.save();
    if (rho > 0) clip(ctx, [ring(...markAt, rho + 10, 77, d)], true);
    ink.wash([tri], { color: AMBER, alpha: 0.55 * upto, bleed: 3 });
    ink.line([...tri, tri[0]], { width: 3.6, color: RED, upto, overshoot: 6 });
    if (upto >= 1) {
      ink.line([[1340, 712], [1340, 744]], { width: 6, color: RED, taper: [0.05, 0.4] });
      ink.ctx.fillStyle = 'rgba(170,46,36,0.92)';
      ink.ctx.beginPath();
      ink.ctx.arc(1340 + signed(d, 1), 756, 4.2, 0, Math.PI * 2);
      ink.ctx.fill();
    }
    ctx.restore();
  }
  if (rho <= 0) return;
  const fade = 1 - 0.75 * smooth(span(t, MARK + 0.4, MARK + 2.4));
  ctx.save();
  clip(ctx, [ring(...markAt, rho, 77, d)], false);
  ink.smudge(1340, 742, 52, { color: SCORCH, alpha: 0.4 * fade, blots: 4 });
  ink.smudge(1340, 750, 34, { color: BURNT, alpha: 0.3 * fade, blots: 3 });
  ctx.restore();
  ash(ink, t);
}

function ash(ink, t) {
  for (let i = 0; i < 9; i++) {
    const t0 = MARK + 0.08 + 0.05 * i;
    const a = t - t0;
    const life = 1.2 + 0.6 * rnd(91, i);
    if (a < 0 || a > life) continue;
    const x = 1300 + 80 * rnd(92, i) + 30 * a * signed(93, i) + 14 * noise1(a * 2 + i, 94);
    const y = 760 - 60 * rnd(95, i) - a * (70 + 40 * rnd(96, i));
    const r = 5 + 5 * rnd(97, i);
    const spin = a * (3 + 4 * signed(98, i));
    const pts = [0, 1, 2, 3].map((j) => {
      const q = spin + j * 1.7 + 0.6 * signed(99, i, j);
      return [x + Math.cos(q) * r, y + Math.sin(q) * r * (0.5 + 0.5 * Math.abs(Math.cos(spin)))];
    });
    ink.wash([pts], { color: [36, 26, 22], alpha: 0.75 * clamp(1 - a / life) ** 0.6, bleed: 1 });
  }
}

export function lineTwo(ink, t) {
  for (let k = 0; k < 4; k++) cellDraw(ink, t, k);
  markDraw(ink, t);
}

function breath(ctx, t, from, k) {
  const tk = CATCH[k];
  const t0 = tk - travel(k);
  const head = ease.out(span(t, t0, tk));
  const tail = ease.in(span(t, t0 + 0.05, tk + 0.16));
  if (head <= tail + 0.02) return;
  const to = plan().cells[k].at;
  const pts = [];
  for (let i = 0; i <= 26; i++) {
    const s = tail + ((head - tail) * i) / 26;
    const bow = Math.sin(Math.PI * s);
    const wob = 12 * bow * noise1(s * 4 - t * 9, 40 + k);
    const dx = to[0] - from[0];
    const dy = to[1] - from[1];
    const len = Math.hypot(dx, dy) || 1;
    pts.push([from[0] + dx * s - (dy / len) * wob, from[1] + dy * s + (dx / len) * wob - 26 * bow]);
  }
  const n = Math.floor(t * 24);
  const o = { boil: 2.2, overshoot: 0 };
  ctx.save();
  ctx.globalCompositeOperation = 'lighter';
  ctx.filter = 'blur(6px)';
  stroke(ctx, pts, { ...o, width: 54, color: [236, 86, 34], alpha: 0.55, taper: [0.7, 0.1] }, 900 + n);
  ctx.filter = 'none';
  ctx.globalCompositeOperation = 'source-over';
  stroke(ctx, pts, { ...o, width: 34, color: [236, 86, 34], alpha: 0.95, taper: [0.7, 0.12] }, 905 + n);
  stroke(ctx, pts, { ...o, width: 20, color: [255, 160, 52], alpha: 0.95, taper: [0.7, 0.15] }, 910 + n);
  stroke(ctx, pts, { ...o, width: 8, color: [255, 238, 184], alpha: 0.95, taper: [0.7, 0.2] }, 920 + n);
  stroke(ctx, pts, { ...o, width: 2.4, color: [122, 30, 12], alpha: 0.7, from: 0.3, upto: 0.85, taper: [0.2, 0.3] }, 930 + n);
  ctx.restore();
}

function leap(ctx, t) {
  const a = span(t, LEAP, MARK);
  if (a <= 0 || a >= 1) return;
  const from = [1250, 682];
  const { markAt } = plan();
  const at = (u) => [from[0] + (markAt[0] - from[0]) * u, from[1] + (markAt[1] - from[1]) * u - 90 * Math.sin(Math.PI * u)];
  ctx.save();
  ctx.globalCompositeOperation = 'lighter';
  ctx.lineCap = 'round';
  for (let j = 0; j < 6; j++) {
    const u0 = Math.max(0, a - 0.05 * (j + 1));
    const [x0, y0] = at(u0);
    const [x1, y1] = at(Math.max(0, a - 0.05 * j));
    ctx.strokeStyle = `rgba(255,${200 - j * 20},${120 - j * 15},${0.9 - j * 0.14})`;
    ctx.lineWidth = 5 - j * 0.6;
    ctx.beginPath();
    ctx.moveTo(x0, y0);
    ctx.lineTo(x1, y1);
    ctx.stroke();
  }
  const [x, y] = at(a);
  const g = ctx.createRadialGradient(x, y, 0, x, y, 22);
  g.addColorStop(0, 'rgba(255,240,190,1)');
  g.addColorStop(1, 'rgba(255,120,40,0)');
  ctx.fillStyle = g;
  ctx.fillRect(x - 22, y - 22, 44, 44);
  ctx.restore();
}

export function fxTwo(ctx, t, tip) {
  const d = Math.floor(t * 12 + 1e-6);
  const tb = d / 12;
  plan().cells.forEach((cell, k) => {
    const tk = CATCH[k];
    if (tip && t > tk - travel(k) - 0.01 && t < tk + 0.2) breath(ctx, t, tip, k);
    const rho = front(tb, tk, BURN, cell.reach);
    if (rho > 0 && rho < cell.reach) {
      band(ctx, t, ring(...cell.at, rho + 3, cell.seed, d), ring(...cell.at, Math.max(0, rho - 20), cell.seed, d), [cell.rect]);
      licks(ctx, t, ring(...cell.at, rho, cell.seed, d), [cell.rect], cell.seed);
    }
    if (rho > 0) {
      ctx.save();
      clip(ctx, [ring(...cell.at, Math.max(0, rho - 6), cell.seed, d)], false);
      glow(ctx, scaled(cell.g, pop(tb, tk), cell.c), 1 - smooth(span(t, tk + 0.12, tk + 1.1)));
      ctx.restore();
    }
    burst(ctx, t, { t0: tk, x: cell.at[0], y: cell.at[1], count: 26, speed: 380, life: 0.6, spread: 1.4, angle: -0.6, gravity: 320, seed: 60 + k, size: 2 });
    burst(ctx, t, { t0: tk + 0.15, x: cell.c[0], y: cell.c[1] - 30, count: 14, speed: 120, life: 1.1, gravity: -110, seed: 70 + k, size: 1.6 });
  });
  leap(ctx, t);
  const { tri, markAt, markReach } = plan();
  const rho = front(tb, MARK, MARK_BURN, markReach);
  if (rho > 0 && rho < markReach) {
    band(ctx, t, ring(...markAt, rho + 4, 77, d), ring(...markAt, Math.max(0, rho - 16), 77, d), [tri]);
    licks(ctx, t, ring(...markAt, rho, 77, d), [tri], 77, 1.3);
  }
  burst(ctx, t, { t0: MARK, x: markAt[0], y: markAt[1], count: 40, speed: 460, life: 0.8, spread: 1.6, angle: -1.2, gravity: 300, seed: 80, size: 2.3 });
}

export function lightsTwo(t) {
  const out = [];
  const heat = (t0, dur) => smooth(span(t, t0, t0 + 0.08)) * (1 - smooth(span(t, t0 + 0.15, t0 + dur)));
  plan().cells.forEach((cell, k) => {
    const h = heat(CATCH[k], 1.3);
    if (h > 0.01) out.push({ x: cell.c[0], y: cell.c[1], r: 330, i: 0.6 * h, c: [1.08, 0.96, 0.85] });
  });
  const m = heat(MARK, 0.9);
  if (m > 0.01) out.push({ x: 1340, y: 735, r: 300, i: 0.65 * m, c: [1.08, 0.96, 0.85] });
  return out;
}
