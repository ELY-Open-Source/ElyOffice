// ElyPDF: a blank page opens at A4, lands Letter.
import { keys, span, clamp, smooth, spring } from '../lib/time.js';

const MM = 2.5;
const A4 = [210 * MM, 297 * MM];
const LETTER = [215.9 * MM, 279.4 * MM];
export const TOP = 164;
export const CREASE = TOP + A4[1] / 2;
export const DROP = [74.0, 74.5];
export const OPEN = [75.0, 75.5];
export const LAND = 76.0;
const SHEET = [246, 238, 222];
const BACK = [226, 212, 188];
const RED = [224, 52, 79];

export const theta = (t) => Math.PI * smooth(span(t, OPEN[0], OPEN[1]));
export const edgeY = (t) => CREASE - (A4[1] / 2) * Math.cos(theta(t));

export function size(t) {
  const k = t < LAND ? 0 : 1 + spring(t - LAND, 0.9, 19, 7);
  return [A4[0] + (LETTER[0] - A4[0]) * k, A4[1] + (LETTER[1] - A4[1]) * k];
}

export function corner(t) {
  const [w, h] = size(t);
  return [960 + w / 2, TOP + h];
}

export const dropY = (t) => keys(t, [[DROP[0], -540], [DROP[1], 0, 'in']]) - (t > DROP[1] ? spring(t - DROP[1], 10, 20, 9) : 0);

function quad(ink, pts, color, alpha) {
  const ctx = ink.ctx;
  ctx.fillStyle = `rgba(${color},${alpha})`;
  ctx.beginPath();
  pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
  ctx.closePath();
  ctx.fill();
}

function edges(ink, pts, widths) {
  pts.forEach((p, i) => ink.line([p, pts[(i + 1) % pts.length]], { width: widths[i % widths.length] }));
}

function flap(ink, t, x0, x1) {
  const th = theta(t);
  const ye = edgeY(t);
  const near = 16 * Math.sin(th);
  const pts = [[x0, CREASE], [x1, CREASE], [x1 + near, ye], [x0 - near, ye]];
  const front = th > Math.PI / 2;
  quad(ink, pts, front ? SHEET : BACK, 1);
  const shade = Math.abs(Math.cos(th));
  ink.wash([pts], { color: [150, 120, 96], alpha: 0.32 * (1 - shade) + (front ? 0 : 0.12), bleed: 2 });
  if (th > 0.05 && th < Math.PI - 0.05) ink.hatch([pts], { gap: 9, angle: 0.25, width: 1.3, alpha: 0.35 * (1 - shade), reach: 0.7 });
  edges(ink, [pts[1], pts[2], pts[3], pts[0]], [2.4, 2.8, 2.4]);
}

export function sheet(ink, t) {
  const ctx = ink.ctx;
  const [w, h] = size(t);
  const x0 = 960 - w / 2;
  const x1 = 960 + w / 2;
  const y1 = TOP + h;
  const open = t >= OPEN[1];
  const bottom = open ? y1 : CREASE;
  ctx.save();
  ctx.translate(0, t < DROP[1] + 0.6 ? dropY(t) : 0);
  ink.smudge(x1 + 10, (TOP + bottom) / 2, 80, { alpha: 0.12, blots: 4 });
  ink.smudge(960, bottom + 10, 120, { alpha: 0.1, blots: 4 });
  quad(ink, [[x0, TOP], [x1, TOP], [x1, bottom], [x0, bottom]], SHEET, 1);
  if (open) {
    const c = TOP + h / 2;
    ink.line([[x0 + 4, c], [x1 - 4, c + 1]], { style: 'charcoal', width: 2.4, alpha: 0.28 });
    edges(ink, [[x0, TOP], [x1, TOP], [x1, y1], [x0, y1]], [2.6, 2.4, 2.8, 2.2]);
    if (t >= LAND) {
      ink.line([[x0 + 2, y1 - 7], [x1 - 2, y1 - 6]], { style: 'charcoal', width: 2, alpha: 0.35 });
      ink.wash([[[x0, y1 - 9], [x1, y1 - 8], [x1, y1], [x0, y1]]], { color: [150, 120, 96], alpha: 0.2, bleed: 2 });
    }
  } else {
    edges(ink, [[x0, CREASE], [x0, TOP], [x1, TOP], [x1, CREASE]], [2.2, 2.6, 2.4]);
    if (t >= DROP[0]) {
      ink.smudge(960, (CREASE + edgeY(t)) / 2, 140, { alpha: 0.2 * Math.sin(theta(t)), blots: 4 });
      flap(ink, t, x0, x1);
    }
  }
  ctx.restore();
}

export function label(ink, t) {
  const [cx, cy] = corner(t);
  const at = { x: cx - 40, y: cy - 22, size: 30, align: 'right', style: 'charcoal', color: [60, 44, 38] };
  if (t >= 75.55) {
    const burn = span(t, LAND + 0.02, LAND + 0.24);
    if (burn < 1) {
      ink.text('A4', { ...at, upto: span(t, 75.55, 75.72), alpha: 0.9 * (1 - burn) });
      if (burn > 0) ink.smudge(at.x - 20, at.y - 10, 26, { color: [70, 34, 20], alpha: 0.4 * Math.sin(Math.PI * burn), blots: 3 });
    }
  }
  if (t >= LAND + 0.12) ink.text('Letter', { ...at, upto: span(t, LAND + 0.12, LAND + 0.5) });
}

export function pin(ink, [x, y], scale, angle) {
  if (scale <= 0.01) return;
  const ctx = ink.ctx;
  ctx.save();
  ctx.translate(x, y);
  ctx.rotate(angle);
  ctx.scale(scale, scale);
  const head = [];
  for (let i = 0; i < 16; i++) {
    const a = (i / 16) * Math.PI * 2;
    head.push([Math.cos(a) * 9, -30 + Math.sin(a) * 9]);
  }
  ink.line([[0, 0], [0.6, -22]], { width: 2.2, overshoot: 2 });
  ink.wash([head], { color: RED, alpha: 0.95, bleed: 1.5 });
  ink.line([...head, head[0], head[1]], { width: 2.2, overshoot: 4 });
  ink.smudge(-3, -33, 3, { color: [255, 240, 220], alpha: 0.5, blots: 1 });
  ctx.restore();
}

export const wobble = (t, k) => clamp(spring(t - (78.68 + 0.07 * k), 0.55, 15, 5), -0.6, 0.6);
