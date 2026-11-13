// s6: the linked book, its tear, the thread.
import { span, clamp, lerp, ease, keys } from '../lib/time.js';
import { signed, rnd, noise1 } from '../lib/rand.js';
import { advance } from '../lib/glyph.js';

const B = { x0: 1260, x1: 1720, y0: 704, y1: 1030 };
const ROOT = [888, 768];
const SOURCE = [B.x0 + 244, B.y0 + 201];
const RED = [196, 34, 40];
const PAPER = 'rgba(244,234,214,0.97)';
export const TEAR = [59.0, 59.4];
const HANG = 59.65;
export const SNAP = 60.0;
const STRANDS = [59.75, 59.875, SNAP, SNAP];
const CUT = 0.38;
const NAME = { text: '销售.xlsx', x: B.x0 + 22, y: B.y0 + 46, size: 30 };

const JAG = Array.from({ length: 25 }, (_, i) => [B.x0 + (i * (B.x1 - B.x0)) / 24, B.y0 + 76 + 6 * signed(77, i) + 3 * signed(78, i)]);
const PIVOT = JAG[0];
const STUB = [[B.x0, B.y0], [B.x1, B.y0], ...JAG.slice().reverse()];
const PIECE = [...JAG, [B.x1, B.y1], [B.x0, B.y1]];

const slide = (t) => keys(t, [[57.7, 720], [58.15, 0, 'out']]);

export function pieceAt(t) {
  if (t < TEAR[0]) return { rot: 0, dx: 0, dy: 0 };
  if (t < TEAR[1]) return { rot: 0.14 * span(t, TEAR[0], TEAR[1]) ** 2, dx: 0, dy: 0 };
  if (t < HANG) {
    const tau = t - TEAR[1];
    return { rot: 0.14 + 0.6 * tau, dx: 60 * tau, dy: 1100 * tau * tau };
  }
  if (t < SNAP) {
    const s = t - HANG;
    return { rot: 0.29 + 0.05 * Math.exp(-5 * s) * Math.sin(14 * s), dx: 15, dy: 68.75 - 16 * Math.exp(-9 * s) * Math.sin(20 * s) };
  }
  const tau = t - SNAP;
  return { rot: 0.29 + 0.9 * tau, dx: 15 + 70 * tau, dy: 68.75 + 150 * tau + 1300 * tau * tau };
}

function map([x, y], p) {
  const c = Math.cos(p.rot);
  const s = Math.sin(p.rot);
  const rx = x - PIVOT[0];
  const ry = y - PIVOT[1];
  return [PIVOT[0] + rx * c - ry * s + p.dx, PIVOT[1] + rx * s + ry * c + p.dy];
}

function poly(ctx, pts) {
  ctx.beginPath();
  pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
  ctx.closePath();
}

function sheet(k) {
  const { x0, x1, y0 } = B;
  for (const y of [126, 176, 226, 276]) k.line([[x0 + 4, y0 + y], [x1 - 6, y0 + y]], { id: 1000 + y, style: 'charcoal', width: 1.4, alpha: 0.35 });
  for (const x of [64, 194, 324]) k.line([[x0 + x, y0 + 80], [x0 + x, y0 + 276]], { id: 1400 + x, style: 'charcoal', width: 1.3, alpha: 0.3 });
  ['5', '6', '7', '8'].forEach((r, i) => k.text(r, { x: x0 + 32, y: y0 + 112 + i * 50, size: 24, align: 'center', color: k.CHARCOAL, alpha: 0.55 }));
  ['1102', '968', '1284', '1350'].forEach((v, i) => k.text(v, { x: x0 + 312, y: y0 + 113 + i * 50, size: 30, align: 'right', alpha: 0.9 }));
  k.text('Q3', { x: x0 + 24, y: y0 + 308, size: 26, alpha: 0.85 });
  k.line([[x0 + 16, y0 + 316], [x0 + 72, y0 + 316]], { id: 760, width: 2.2 });
}

function stub(k) {
  k.text(NAME.text, { ...NAME, alpha: 0.9 });
  k.line([[B.x0, B.y0 + 62], [B.x1, B.y0 + 62]], { id: 761, style: 'charcoal', width: 1.6, alpha: 0.4 });
}

function frame(k, pts, id) {
  k.ctx.fillStyle = PAPER;
  poly(k.ctx, pts);
  k.ctx.fill();
  k.line(pts, { id, width: 2.6, overshoot: 8, pressure: 0.45 });
}

export function book(ink, t) {
  const dx = slide(t);
  if (dx >= 720) return;
  const u = 1 - dx / 720;
  const p = pieceAt(t);
  const torn = t >= TEAR[0];
  const { x0, x1, y0, y1 } = B;
  ink.lift(lerp(40, 12, u), [1490 + dx, 860], (k) => {
    const ctx = k.ctx;
    ctx.save();
    ctx.translate(dx, 0);
    ctx.rotate(-0.05 * (1 - u));
    if (!torn) {
      frame(k, [[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]], 762);
      stub(k);
      sheet(k);
    } else {
      frame(k, [[x0, JAG[0][1]], [x0, y0], [x1, y0], [x1, JAG[24][1]]], 763);
      ctx.fillStyle = PAPER;
      poly(ctx, STUB);
      ctx.fill();
      stub(k);
      const front = x1 - (x1 - x0) * span(t, TEAR[0], TEAR[1]);
      const rag = JAG.filter(([x]) => x >= front - 1);
      if (rag.length > 1) k.line(rag, { id: 764, width: 1.8, alpha: 0.8, overshoot: 0 });
      rag.forEach(([x, y], i) => i % 2 || k.line([[x, y], [x + 3 * signed(79, i), y + 5]], { id: 765 + i, width: 1, alpha: 0.6 }));
    }
    ctx.restore();
  });
  if (!torn || p.dy > 700) return;
  ink.lift(12 + 60 * clamp(p.dy / 300), map([1490, 900], p), (k) => {
    const ctx = k.ctx;
    ctx.save();
    ctx.translate(PIVOT[0] + p.dx, PIVOT[1] + p.dy);
    ctx.rotate(p.rot);
    ctx.translate(-PIVOT[0], -PIVOT[1]);
    ctx.fillStyle = PAPER;
    poly(ctx, PIECE);
    ctx.fill();
    ctx.save();
    poly(ctx, PIECE);
    ctx.clip();
    sheet(k);
    ctx.restore();
    k.line([...JAG, [x1, y1], [x0, y1], JAG[0]], { id: 790, width: 2.4, overshoot: 4 });
    ctx.restore();
  });
}

export function sourceAt(t) {
  return map([SOURCE[0] + slide(t), SOURCE[1]], pieceAt(t));
}

export const strainAt = (t) => span(t, HANG, SNAP);

const sagAt = (t) => 70 * (1 - ease.inOut(span(t, TEAR[1], HANG)));

function curveAt(a, b, sag, n = 16) {
  return Array.from({ length: n + 1 }, (_, i) => {
    const u = i / n;
    return [lerp(a[0], b[0], u), lerp(a[1], b[1], u) + 4 * sag * u * (1 - u)];
  });
}

const along = (pts, u) => {
  const f = u * (pts.length - 1);
  const i = Math.min(pts.length - 2, Math.floor(f));
  return [lerp(pts[i][0], pts[i + 1][0], f - i), lerp(pts[i][1], pts[i + 1][1], f - i)];
};

export function breakAt(t) {
  const s = sourceAt(Math.min(t, SNAP));
  return along(curveAt(ROOT, s, sagAt(t)), CUT);
}

function knot(ink, [x, y], id) {
  ink.smudge(x, y, 7, { id, color: ink.INK, alpha: 0.5, blots: 2 });
  ink.line([[x - 4, y - 3], [x + 4, y + 3], [x - 3, y + 4]], { id: id + 1, width: 3.4, overshoot: 2 });
}

function tuft(ink, [tx, ty], dir, id) {
  for (let f = 0; f < 5; f++) {
    const a = dir + (f - 2) * 0.32 + 0.1 * signed(id, f);
    const l = 14 + 12 * rnd(id, f, 2);
    ink.curve([[tx, ty], [tx + Math.cos(a) * l * 0.5, ty + Math.sin(a) * l * 0.5 + 2], [tx + Math.cos(a) * l, ty + Math.sin(a) * l + 5]],
      { id: id + 10 + f, width: 1, alpha: 0.75, overshoot: 0, taper: [0.1, 0.6] });
  }
}

function dangle(ink, t, from, a0, length, id) {
  const tau = t - SNAP;
  const settle = 1 - Math.exp(-5 * tau);
  const swing = 0.55 * Math.exp(-3 * tau) * Math.cos(9 * tau) + 0.07 * noise1(t * 1.4, id);
  const pts = [from];
  for (let i = 0; i < 9; i++) {
    const th = lerp(a0, Math.PI / 2 + swing, settle * (0.25 + 0.75 * (i / 8)));
    const [x, y] = pts[pts.length - 1];
    pts.push([x + Math.cos(th) * (length / 9), y + Math.sin(th) * (length / 9)]);
  }
  ink.curve(pts, { id, width: 3, taper: [0.02, 0.1], overshoot: 0 });
  const tip = pts[pts.length - 1];
  tuft(ink, tip, Math.atan2(tip[1] - pts[7][1], tip[0] - pts[7][0]), id);
}

function strands(ink, t, pts) {
  const a = along(pts, CUT - 0.08);
  const b = along(pts, CUT + 0.08);
  const nl = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
  const [nx, ny] = [-(b[1] - a[1]) / nl, (b[0] - a[0]) / nl];
  const open = ease.out(strainAt(t));
  STRANDS.forEach((at, f) => {
    const off = (f - 1.5) * 8 * open;
    const m = along(pts, CUT + 0.015 * signed(803, f));
    const mid = [m[0] + nx * off, m[1] + ny * off];
    if (t < at) {
      ink.curve([a, mid, b], { id: 804 + f, width: 1.2, overshoot: 0, taper: [0, 0] });
      return;
    }
    const curl = 1 - Math.exp(-14 * (t - at));
    for (const [end, s] of [[a, 1], [b, -1]]) {
      const tip = [lerp(end[0], mid[0], 0.6) + nx * off * curl, lerp(end[1], mid[1], 0.6) + 10 * curl * s];
      ink.curve([end, [lerp(end[0], tip[0], 0.5), lerp(end[1], tip[1], 0.5) - 3 * s], tip],
        { id: 880 + f * 2 + (s > 0 ? 0 : 1), width: 1.1, overshoot: 0, taper: [0, 0.6] });
    }
  });
}

export function thread(ink, t) {
  if (t < 58.0) return;
  const s = sourceAt(t);
  if (t < SNAP) {
    const pts = curveAt(ROOT, s, sagAt(t));
    const reach = ease.out(span(t, 58.0, 58.5));
    if (t < HANG) ink.line(pts, { id: 800, width: 3.2, upto: reach, head: true, overshoot: 0, taper: [0.02, 0.02] });
    else {
      const width = 3.2 - 0.8 * strainAt(t);
      ink.line(pts, { id: 801, width, upto: CUT - 0.08, overshoot: 0, taper: [0.02, 0] });
      ink.line(pts, { id: 802, width, from: CUT + 0.08, overshoot: 0, taper: [0, 0.02] });
      strands(ink, t, pts);
    }
    knot(ink, ROOT, 810);
    if (reach >= 1) knot(ink, s, 812);
    return;
  }
  const brk = breakAt(t);
  dangle(ink, t, ROOT, Math.atan2(brk[1] - ROOT[1], brk[0] - ROOT[0]), Math.hypot(brk[0] - ROOT[0], brk[1] - ROOT[1]) * 0.8, 820);
  knot(ink, ROOT, 810);
  if (s[1] < 1250) dangle(ink, t, s, Math.atan2(brk[1] - s[1], brk[0] - s[0]) - 0.3, 220, 840);
  const tau = t - SNAP;
  for (let i = 0; i < 9 && tau < 1.1; i++) {
    const x = brk[0] + signed(850, i) * 90 * tau + 12 * Math.sin(tau * 7 + i);
    const y = brk[1] + (rnd(851, i) * 60 - 50) * tau + 260 * tau * tau;
    const a = 6 * tau * signed(852, i) + i;
    ink.line([[x, y], [x + Math.cos(a) * 9, y + Math.sin(a) * 9]], { id: 860 + i, width: 1.1, alpha: 0.75 * (1 - tau / 1.1), overshoot: 0 });
  }
}

export function formula(ink, t) {
  const u = ease.out(span(t, 58.0, 58.4));
  if (u <= 0) return;
  ink.text('C10', { x: 460, y: 180, size: 30, align: 'center', alpha: 0.85, upto: u });
  ink.text('=[销售.xlsx]Q3!B7', { x: 522, y: 180, size: 30, alpha: 0.92, upto: u });
}

export function circle(ink, t) {
  const u = span(t, 61.0, 61.75);
  if (u <= 0) return;
  const w = advance(NAME.text, NAME.size);
  const cx = NAME.x + w / 2;
  const cy = NAME.y - 10;
  const pts = [];
  for (let i = 0; i <= 80; i++) {
    const a = -2.6 + (i / 80) * Math.PI * 2 * 1.22;
    const g = 1 + 0.07 * (i / 80) + 0.02 * Math.sin(a * 3);
    pts.push([cx + Math.cos(a) * (w / 2 + 34) * g + 6 * (i / 80), cy + Math.sin(a) * 40 * g - 4 * (i / 80)]);
  }
  ink.line(pts, { id: 870, width: 4.8, color: RED, alpha: 0.9, upto: ease.inOut(u), head: true, overshoot: 0, taper: [0.03, 0.12], pressure: 0.5 });
}
