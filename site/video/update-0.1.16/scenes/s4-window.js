// The Windows window: grey and waiting, then filled.
import { keys, span, clamp, ease } from '../lib/time.js';
import { noise1, rnd } from '../lib/rand.js';
import { table, place, rect, circle } from './s4-icons.js';

export const LAND = [1350, 284];
export const FAR = 560;
export const OPEN = 40.0;
const X0 = 1020;
const Y0 = 250;
const X1 = 1680;
const Y1 = 700;
const TOP = Y0 + 44;
const HEAD = TOP + 28;
const COL = 110;
const ROW = 34;
const CARD = [1350, 492];
const GREEN = [46, 158, 91];
const FRAME = rect(X0, Y0, X1 - X0, Y1 - Y0, 12);
const CLIENT = [[X0, TOP], [X1, TOP], [X1, Y1], [X0, Y1]];

const VALUES = [
  ['', 'Q1', 'Q2', 'Q3', 'Q4'],
  ['2023', '1,204', '1,380', '1,512', '1,766'],
  ['2024', '1,690', '1,845', '2,032', '2,310'],
  ['2025', '2,288', '2,476', '2,615', '2,940'],
  ['2026', '2,904', '3,155', '3,402', '3,870'],
  ['', '8,086', '8,856', '9,561', '10,886'],
];

export const reach = (t) => (t < OPEN ? 0 : keys(t, [[OPEN, 0], [OPEN + 0.5, FAR, 'out']]));

export function frontPoly(r, d) {
  const pts = [];
  for (let i = 0; i < 64; i++) {
    const a = (i / 64) * Math.PI * 2;
    const k = Math.max(0, r * (1 + 0.14 * noise1(a * 2.2 + 5, 81) + 0.05 * noise1(a * 7, 83)) + 1.5 * noise1(i * 0.7, d));
    pts.push([LAND[0] + Math.cos(a) * k, LAND[1] + Math.sin(a) * k]);
  }
  return pts;
}

function clip(ctx, rings, outside) {
  ctx.beginPath();
  if (outside) ctx.rect(-10, -10, 1940, 1100);
  for (const r of rings) {
    r.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
    ctx.closePath();
  }
  ctx.clip('evenodd');
}

function chrome(ink, t) {
  const u = span(t, 36.5, 37.5);
  if (u <= 0) return;
  ink.line(FRAME, { id: 'frame', width: 3.4, closed: true, upto: ease.inOut(u), head: true, overshoot: 8 });
  const v = span(t, 37.5, 37.75);
  if (v > 0) ink.line([[X0 + 2, TOP], [X1 - 2, TOP]], { id: 'bar', width: 1.8, alpha: 0.6, upto: v, overshoot: 5 });
  const w = span(t, 37.6, 37.9);
  const bx = X1 - 140;
  if (w > 0) {
    ink.line([[bx, Y0 + 22], [bx + 16, Y0 + 22]], { width: 2, upto: clamp(w * 3) });
    ink.line(rect(bx + 46, Y0 + 15, 14, 14), { width: 2, closed: true, upto: clamp(w * 3 - 1) });
    ink.line([[bx + 94, Y0 + 15], [bx + 108, Y0 + 29]], { width: 2, upto: clamp(w * 3 - 1.5) });
    ink.line([[bx + 108, Y0 + 15], [bx + 94, Y0 + 29]], { width: 2, upto: clamp(w * 3 - 2) });
  }
}

function grey(ink, t, d) {
  const g = span(t, 37.6, 38.0);
  if (g <= 0) return;
  const r = reach(t);
  const ctx = ink.ctx;
  ctx.save();
  if (r > 0) clip(ctx, [frontPoly(r + 24, d)], true);
  ink.wash([CLIENT], { color: [112, 112, 118], alpha: 0.8 * g, bleed: 3 });
  ink.wash([[[X0 + 8, Y0 + 3], [X1 - 8, Y0 + 3], [X1, TOP], [X0, TOP]]], { color: [160, 158, 160], alpha: 0.35 * g, bleed: 2 });
  ink.hatch([CLIENT], { gap: 9, angle: -0.8, width: 1.4, alpha: 0.3 * g });
  if (t < OPEN) busy(ink, t, g);
  ctx.restore();
}

function busy(ink, t, g) {
  const [cx, cy] = [1350, 497];
  const a0 = t * 7.5;
  const pts = Array.from({ length: 22 }, (_, i) => {
    const a = a0 + (i / 21) * 5.2;
    return [cx + Math.cos(a) * 20, cy + Math.sin(a) * 20];
  });
  ink.line(pts, { width: 5.5, color: [0, 103, 192], alpha: 0.9 * g, taper: [0.6, 0.05], overshoot: 0, boil: 0.8 });
}

function cells(ink, t, d) {
  const r = reach(t);
  if (r <= 0) return;
  const ctx = ink.ctx;
  ctx.save();
  clip(ctx, [frontPoly(r, d)], false);
  ink.wash([[[X0, TOP], [X1, TOP], [X1, HEAD], [X0, HEAD]]], { color: [214, 210, 202], alpha: 0.6, bleed: 2 });
  ink.wash([[[X0, HEAD], [X0 + 40, HEAD], [X0 + 40, Y1], [X0, Y1]]], { color: [214, 210, 202], alpha: 0.6, bleed: 2 });
  for (let k = 0; k <= 5; k++) {
    const x = X0 + 40 + k * COL;
    ink.line([[x, TOP + 2], [x, Y1 - 3]], { id: `col${k}`, width: 1.2, alpha: 0.32, style: 'charcoal', overshoot: 0 });
    if (x + COL / 2 < X1) ink.text('ABCDEF'[k], { x: x + COL / 2, y: TOP + 21, size: 18, align: 'center', fill: 'solid', alpha: 0.7 });
  }
  for (let k = 0; k <= 11; k++) {
    const y = HEAD + k * ROW;
    ink.line([[X0 + 3, y], [X1 - 3, y]], { id: `row${k}`, width: 1.2, alpha: 0.32, style: 'charcoal', overshoot: 0 });
    if (y + ROW <= Y1) ink.text(String(k + 1), { x: X0 + 20, y: y + 24, size: 17, align: 'center', fill: 'solid', alpha: 0.6 });
  }
  ctx.restore();
  VALUES.forEach((row, j) => row.forEach((v, k) => {
    if (!v) return;
    const x = X0 + 40 + k * COL;
    const y = HEAD + j * ROW;
    const dist = Math.hypot(x + COL / 2 - LAND[0], y + ROW / 2 - LAND[1]);
    const upto = clamp((r - dist) / 140);
    if (upto <= 0) return;
    const right = k > 0 && j > 0;
    ink.text(v, { x: right ? x + COL - 9 : x + 9, y: y + 24, size: 21, align: right ? 'right' : 'left', fill: 'solid', upto, alpha: 0.9 });
  }));
  const sel = span(t, 40.55, 40.75);
  if (sel > 0) ink.line(rect(X0 + 40 + COL, HEAD + ROW, COL, ROW), { id: 'sel', width: 3.4, color: GREEN, closed: true, upto: sel, overshoot: 3 });
  const tab = span(t, 40.3, 40.6);
  if (tab > 0) {
    place(ink, { x: X0 + 26, y: Y0 + 22, a: 0, s: 0.14 }, table);
    ink.text('sales.xlsx', { x: X0 + 46, y: Y0 + 29, size: 20, fill: 'solid', upto: tab, alpha: 0.85 });
  }
}

function bloom(ink, t) {
  const r = reach(t);
  if (r <= 0 || r >= FAR - 5) return;
  const fade = 1 - r / FAR;
  const ctx = ink.ctx;
  ctx.save();
  clip(ctx, [FRAME], false);
  for (let k = 0; k < 16; k++) {
    const a = (k / 16) * Math.PI * 2 + 0.3 * rnd(91, k);
    const rr = r * (0.84 + 0.12 * rnd(92, k));
    const size = (10 + 22 * rnd(93, k)) * (0.6 + 0.6 * fade);
    ink.wash([circle(LAND[0] + Math.cos(a) * rr, LAND[1] + Math.sin(a) * rr, size, 10)], { color: GREEN, alpha: 0.75 * fade, bleed: 5 });
  }
  ctx.restore();
}

function splash(ink, t) {
  if (t < OPEN || t > 41.2) return;
  const sc = keys(t, [[OPEN, 0.35], [OPEN + 0.1, 1.08, 'out'], [OPEN + 0.22, 1], [40.98, 1], [41.16, 0.02, 'in']]);
  const ctx = ink.ctx;
  ctx.save();
  ctx.translate(CARD[0], CARD[1]);
  ctx.scale(sc * 1.35, sc);
  const g = ctx.createRadialGradient(0, 4, 0, 0, 4, 128);
  g.addColorStop(0, 'rgba(250,246,236,0.95)');
  g.addColorStop(0.6, 'rgba(250,246,236,0.75)');
  g.addColorStop(1, 'rgba(250,246,236,0)');
  ctx.fillStyle = g;
  ctx.fillRect(-130, -124, 260, 256);
  ctx.scale(1 / 1.35, 1);
  ctx.translate(-CARD[0], -CARD[1]);
  place(ink, { x: CARD[0], y: CARD[1] - 22, a: 0, s: 0.5 }, table);
  ink.text('ElyTable', { x: CARD[0], y: CARD[1] + 72, size: 28, align: 'center', fill: 'solid', alpha: 0.9 });
  ctx.restore();
}

export function drawWindow(ink, t) {
  if (t < 36.5) return;
  const d = ink.drawing;
  grey(ink, t, d);
  cells(ink, t, d);
  bloom(ink, t);
  chrome(ink, t);
  splash(ink, t);
}

export function frontGlow(ctx, t, d) {
  const r = reach(t);
  if (r <= 0 || r >= FAR - 5) return;
  const heat = 1 - r / FAR;
  ctx.save();
  clip(ctx, [FRAME], false);
  ctx.globalCompositeOperation = 'lighter';
  ctx.filter = 'blur(3px)';
  ctx.strokeStyle = `rgba(255,170,80,${0.55 * heat})`;
  ctx.lineWidth = 10 * heat + 3;
  ctx.beginPath();
  frontPoly(r, d).forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
  ctx.closePath();
  ctx.stroke();
  ctx.restore();
}
