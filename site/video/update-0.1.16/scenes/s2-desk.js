// s2's desk: two windows, three files, the photo.
import { span, lerp, ease } from '../lib/time.js';
import { rgba } from '../lib/stroke.js';

export const BLUE = [61, 127, 229];
export const GREEN = [46, 158, 91];
export const GOLD = [255, 213, 79];
export const DOC = { x0: 240, y0: 90, x1: 880, y1: 560, bar: 142 };
export const TAB = { x0: 1080, y0: 170, x1: 1700, y1: 640, bar: 222 };
export const HALF = [88, 64];
export const SHEET = [39, 50, 22];
const BASE = [243, 234, 215];

export function poly(ctx, pts, color, alpha = 1) {
  ctx.fillStyle = rgba(color, alpha);
  ctx.beginPath();
  pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
  ctx.closePath();
  ctx.fill();
}

export const ring = (cx, cy, r, n = 18) => Array.from({ length: n }, (_, i) => {
  const a = (i / n) * Math.PI * 2;
  return [cx + Math.cos(a) * r, cy + Math.sin(a) * r];
});

export function frame(ink, t, win, o) {
  const { x0, y0, x1, y1, bar } = win;
  const at = (k) => span(t, o.t0 + k * 0.08, o.t0 + k * 0.08 + 0.24);
  const boil = 1.7 + (o.shiver ?? 0);
  const band = [[x0 + 3, y0 + 3], [x1 - 3, y0 + 3], [x1 - 3, bar - 2], [x0 + 3, bar - 2]];
  const tint = at(5) * o.tint;
  if (tint > 0) ink.wash([band], { id: `${o.id}-band`, color: o.color, alpha: tint, bleed: 5 });
  const edges = [[[x0, y0], [x1, y0]], [[x0, y0], [x0, y1]], [[x0, y1], [x1, y1]], [[x1, y0], [x1, y1]],
    [[x0 + 2, bar], [x1 - 2, bar]]];
  edges.forEach((e, k) => {
    const u = at(k);
    if (u > 0) ink.line(e, { id: `${o.id}-${k}`, width: k === 4 ? 2 : 3.2, upto: u, overshoot: k === 4 ? 4 : 16,
      boil: k === 4 ? boil * 0.7 : boil, taper: [0.05, 0.1], pressure: 0.4 });
  });
  const mid = (y0 + bar) / 2;
  const u = at(6);
  if (u > 0) {
    const pen = { width: 2.2, upto: u, overshoot: 3, boil: 0.8 };
    ink.line([[x1 - 70, mid], [x1 - 54, mid]], pen);
    ink.line([[x1 - 32, mid - 7], [x1 - 18, mid + 7]], pen);
    ink.line([[x1 - 18, mid - 7], [x1 - 32, mid + 7]], pen);
  }
  for (const [title, a, b] of o.titles) {
    if (t < a || t >= b) continue;
    ink.text(title, { x: (x0 + x1) / 2, y: mid + 9, size: 26, align: 'center', upto: span(t, a, a + 0.3), alpha: 0.85 });
  }
}

function scrawl(ink, x0, x1, y, o) {
  const pts = [];
  for (let x = x0; x <= x1; x += 6) pts.push([x, y + 1.6 * Math.sin(x * 0.31 + y)]);
  ink.curve(pts, { width: 1.8, alpha: 0.7, overshoot: 2, boil: 0.5, segs: 3, ...o });
}

export function sheet(ink, kind) {
  const [w, h, f] = SHEET;
  const out = [[-w, -h], [w - f, -h], [w, -h + f], [w, h], [-w, h]];
  const color = kind === 'doc' ? BLUE : GREEN;
  poly(ink.ctx, out, BASE);
  ink.wash([out], { color, alpha: 0.2, bleed: 2 });
  if (kind === 'doc') {
    ink.wash([[[-w + 8, -h + 12], [w - f - 4, -h + 12], [w - f - 4, -h + 26], [-w + 8, -h + 26]]], { color, alpha: 0.6, bleed: 2 });
    for (let k = 0; k < 5; k++) scrawl(ink, -w + 10, w - 10 - (k === 4 ? 26 : 0), -h + 42 + k * 11, {});
  } else {
    ink.wash([[[-w + 6, -h + 26], [w - 6, -h + 26], [w - 6, -h + 40], [-w + 6, -h + 40]]], { color, alpha: 0.6, bleed: 2 });
    const grid = { width: 1.4, alpha: 0.6, overshoot: 2, boil: 0.5 };
    for (let k = 0; k < 6; k++) ink.line([[-w + 6, -h + 26 + k * 14], [w - 6, -h + 26 + k * 14]], grid);
    for (const x of [-w + 6, -8, 18, w - 6]) ink.line([[x, -h + 26], [x, h - 14]], grid);
  }
  ink.line(out, { closed: true, width: 2.6, overshoot: 5, boil: 1 });
  ink.line([[w - f, -h], [w - f, -h + f], [w, -h + f]], { width: 2, overshoot: 2, boil: 0.8 });
}

export function photo(ink) {
  const [w, h] = HALF;
  const ctx = ink.ctx;
  const rect = [[-w, -h], [w, -h], [w, h], [-w, h]];
  poly(ctx, rect, BASE);
  ctx.save();
  ctx.beginPath();
  ctx.rect(-w - 2, -h - 2, w * 2 + 4, h * 2 + 4);
  ctx.clip();
  ink.wash([[[-w, -h], [w, -h], [w, 20], [-w, 20]]], { color: [128, 174, 218], alpha: 0.55, bleed: 4 });
  const sun = ring(38, -28, 15);
  ink.wash([sun], { color: [246, 150, 56], alpha: 0.9, bleed: 2 });
  const far = [[-w - 6, 6], [-52, -16], [-10, -4], [28, -22], [w + 6, -2], [w + 6, h + 6], [-w - 6, h + 6]];
  ink.wash([far], { color: [108, 154, 120], alpha: 0.65, bleed: 3 });
  const near = [[-w - 6, 30], [-30, 12], [18, 24], [58, 8], [w + 6, 18], [w + 6, h + 6], [-w - 6, h + 6]];
  ink.wash([near], { color: [72, 130, 66], alpha: 0.75, bleed: 3 });
  ink.hatch([near], { gap: 6, angle: -0.5, width: 1.1, alpha: 0.35 });
  ctx.restore();
  ink.curve(far.slice(0, 5), { width: 1.6, alpha: 0.6, overshoot: 0, boil: 0.6 });
  ink.curve(near.slice(0, 5), { width: 2, alpha: 0.7, overshoot: 0, boil: 0.6 });
  ink.line(sun, { closed: true, width: 1.8, overshoot: 4, boil: 0.6 });
  ink.line([[-48, 24], [-46, 2]], { width: 2.4, boil: 0.5 });
  const crown = ring(-46, -6, 13, 12);
  ink.wash([crown], { color: [40, 92, 52], alpha: 0.85, bleed: 2 });
  ink.line(crown, { closed: true, width: 1.6, overshoot: 3, boil: 0.6 });
  ink.line(rect, { closed: true, width: 2.6, overshoot: 7, boil: 1 });
}

export function unfold(ink, t, t0, [cx, cy], win) {
  const u = ease.out(span(t, t0, t0 + 0.3));
  const fade = 1 - span(t, t0 + 0.12, t0 + 0.34);
  if (u <= 0 || fade <= 0) return;
  const [w, h] = SHEET;
  const l = lerp(cx - w, win.x0 + 18, u);
  const r = lerp(cx + w, win.x1 - 18, u);
  const tp = lerp(cy - h, win.bar + 14, u);
  const b = lerp(cy + h, win.y1 - 14, u);
  const arm = 30;
  const pen = { width: 2.6, alpha: 0.85 * fade, overshoot: 4, boil: 1.2 };
  for (const [x, y, sx, sy] of [[l, tp, 1, 1], [r, tp, -1, 1], [r, b, -1, -1], [l, b, 1, -1]]) {
    ink.line([[x + sx * arm, y], [x, y], [x, y + sy * arm]], pen);
  }
}

export function bloom(ink, t, t0, [cx, cy], win, color) {
  const r = 60 + 520 * ease.out(span(t, t0, t0 + 0.45));
  const a = 0.32 * (1 - span(t, t0 + 0.1, t0 + 1.0));
  if (t < t0 || a <= 0) return;
  const ctx = ink.ctx;
  ctx.save();
  ctx.beginPath();
  ctx.rect(win.x0 + 3, win.bar + 3, win.x1 - win.x0 - 6, win.y1 - win.bar - 6);
  ctx.clip();
  ink.wash([ring(cx, cy, r, 40)], { color, alpha: a, bleed: 16 });
  ctx.restore();
}
