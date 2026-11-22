// ElyShow parts in line art: chrome, menu, slides, keys, hand.
import { kit } from '../lib/kit/index.js';

export const WIN = { x: 280, y: 150, w: 1360, h: 780, r: 12, bar: 56 };
export const SLIDE = { x: 520, y: 286, w: 1088, h: 612 };
export const THUMB = { x: 312, w: 176, h: 99, ys: [286, 405, 524] };
export const KEY_H = 64;

// A closed loop restarted at its point nearest p.
function restart(pts, p, back) {
  const ring = pts.slice(0, -1);
  let best = 0;
  ring.forEach((q, i) => {
    if (Math.hypot(q[0] - p[0], q[1] - p[1]) < Math.hypot(ring[best][0] - p[0], ring[best][1] - p[1])) best = i;
  });
  const cw = [...ring.slice(best), ...ring.slice(0, best)];
  return back ? [cw[0], ...cw.slice(1).reverse(), cw[0]] : [...cw, cw[0]];
}

export function chrome() {
  const { x, y, w, h, r, bar } = WIN;
  const o = { w: 1.05 };
  const outline = kit.stroke(restart(kit.rect(x, y, w, h, { r }).pts, [x + w - r, y], true), o);
  const mid = y + bar / 2;
  const g = 5;
  const out = [outline];
  const close = x + w - 36;
  const max = x + w - 74;
  const min = x + w - 112;
  out.push(kit.circle(close, mid, 13, o), kit.line(close - g, mid - g, close + g, mid + g, o), kit.line(close + g, mid - g, close - g, mid + g, o));
  out.push(kit.circle(max, mid, 13, o), kit.rect(max - g, mid - g, g * 2, g * 2, o));
  out.push(kit.circle(min, mid, 13, o), kit.line(min - g, mid + 1, min + g, mid + 1, o));
  out.push(kit.line(x + w, y + bar, x, y + bar, o));
  return out;
}

// The slide, clockwise from sx on its top edge.
export function canvas(sx) {
  const { x, y, w, h } = SLIDE;
  return kit.stroke([[sx, y], [x + w, y], [x + w, y + h], [x, y + h], [x, y], [sx, y]], { w: 1.1 });
}

export function menu() {
  const words = ['File', 'Edit', 'View', 'Insert'];
  const size = 28;
  let x = WIN.x + 32;
  const out = [];
  const at = {};
  for (const word of words) {
    const s = kit.text(word, { x, y: WIN.y + WIN.bar + 36, size, w: 0.9 });
    s[0].link = 1;
    at[word] = { x, w: kit.measure(word, { size }).w };
    out.push(...s);
    x += at[word].w + 30;
  }
  return { strokes: out, insert: at.Insert };
}

export function dropdown(insert) {
  const bx = insert.x - 14;
  const by = WIN.y + WIN.bar + 50;
  const cy = by + 46;
  const cell = (i) => bx + 46 + i * 76;
  const o = { w: 0.95 };
  const box = kit.rect(bx, by, 236, 92, { r: 8, w: 0.9 });
  const shape = kit.rect(cell(0) - 25, cy - 18, 50, 36, { r: 3, ...o });
  const oval = kit.circle(cell(1), cy, 20, o);
  const tri = kit.poly([[cell(2), cy - 21], [cell(2) - 23, cy + 18], [cell(2) + 23, cy + 18]], { closed: true, ...o });
  const lit = kit.rect(cell(0) - 34, cy - 30, 68, 60, { r: 8, w: 0.5 });
  return { strokes: [box, tri, oval, shape], lit, target: [cell(0), cy], top: by, left: bx, right: bx + 236 };
}

// Eight handles; the last one sits where the click lands.
export function handles(x0, y0, x1, y1, s) {
  const mx = (x0 + x1) / 2;
  const my = (y0 + y1) / 2;
  const spots = [[x0, y0], [x0, my], [x0, y1], [mx, y1], [x1, y1], [x1, my], [x1, y0], [mx, y0]];
  const h = 8 * s;
  return spots.map(([x, y]) => kit.rect(x - h, y - h, h * 2, h * 2, { w: 0.9 }));
}

export const thumb = (i) => kit.rect(THUMB.x, THUMB.ys[i], THUMB.w, THUMB.h, { w: 0.8 });

export function mini(strokes, i) {
  const s = THUMB.w / SLIDE.w;
  return kit.move(strokes, { x: THUMB.x - SLIDE.x, y: THUMB.ys[i] - SLIDE.y, s, ox: SLIDE.x, oy: SLIDE.y })
    .map((st) => ({ ...st, w: 0.6 }));
}

const sx = (v) => SLIDE.x + v;
const sy = (v) => SLIDE.y + v;

export function diagram() {
  const o = { w: 1.2 };
  const ring = kit.circle(sx(250), sy(380), 78, o);
  const wire = kit.poly([[sx(328), sy(380)], [sx(470), sy(380)], [sx(470), sy(300)], [sx(612), sy(300)]], o);
  const barb = kit.poly([[sx(594), sy(288)], [sx(614), sy(300)], [sx(594), sy(312)]], o);
  const box = kit.rect(sx(620), sy(220), 160, 160, { r: 16, ...o });
  return { strokes: [ring, wire, barb, box], hit: [sx(470), sy(340)] };
}

export function tilted() {
  const cx = sx(560);
  const cy = sy(320);
  const card = kit.rect(cx - 200, cy - 115, 400, 230, { r: 22, w: 1.25 });
  const turned = kit.move([card], { r: -0.26, ox: cx, oy: cy });
  return { strokes: turned, hit: [cx, cy] };
}

export function titleBox() {
  const x = sx(72);
  const y = sy(40);
  const w = 620;
  const h = 116;
  const pts = [[x, y + h / 2], [x, y], [x + w, y], [x + w, y + h], [x, y + h], [x, y + h / 2]];
  return { frame: kit.stroke(pts, { w: 0.45 }), click: [x, y + h / 2], text: { x: x + 24, y: y + 84, size: 64 } };
}

export const S1_RECT = { x: sx(200), y: sy(150), w: 360, h: 240 };

// Curly braces, top to bottom; the right one mirrored.
function brace(cx, cy, h, side) {
  const w = 34 * side;
  const pts = [[0.5, -0.5], [0.12, -0.47], [0, -0.38], [0, -0.13], [-0.14, -0.035], [-0.5, 0], [-0.14, 0.035], [0, 0.13],
    [0, 0.38], [0.12, 0.47], [0.5, 0.5]].map(([x, y]) => [cx + x * w, cy + y * h]);
  return kit.stroke(smooth(pts, 6), { w: 1.2 });
}

export function braces() {
  const cx = S1_RECT.x + S1_RECT.w / 2;
  const cy = S1_RECT.y + S1_RECT.h / 2;
  const right = brace(cx + 62, cy, 124, -1);
  return [brace(cx - 62, cy, 124, 1), { ...right, pts: right.pts.slice().reverse() }];
}

function arrow(cx, cy, dir) {
  const d = dir;
  return [
    kit.line(cx - 15 * d, cy, cx + 15 * d, cy, { w: 0.95 }),
    kit.poly([[cx + 4 * d, cy - 11], [cx + 15 * d, cy], [cx + 4 * d, cy + 11]], { w: 0.95 }),
  ];
}

// Key cap: base, raised face, label; press sinks it.
export function keycap(label, x, y, w, press) {
  const sink = 6 * press;
  const base = kit.rect(x, y, w, KEY_H, { r: 12, w: 0.85 });
  const fy = y + 4 + sink;
  const face = kit.rect(x + 7, fy, w - 14, KEY_H - 18, { r: 8, w: 0.85 });
  const cx = x + w / 2;
  const cy = fy + (KEY_H - 18) / 2;
  let mark;
  if (label === '←' || label === '→') mark = arrow(cx, cy, label === '→' ? 1 : -1);
  else {
    const size = 26;
    mark = kit.text(label, { x: cx, y: cy + size * 0.36, size, align: 'center', w: 0.85 });
  }
  return [base, face, ...mark];
}

function smooth(pts, n = 5) {
  const out = [];
  for (let i = 0; i < pts.length - 1; i++) {
    const p0 = pts[Math.max(0, i - 1)];
    const p1 = pts[i];
    const p2 = pts[i + 1];
    const p3 = pts[Math.min(pts.length - 1, i + 2)];
    for (let j = 0; j < n; j++) {
      const u = j / n;
      const u2 = u * u;
      const u3 = u2 * u;
      out.push([0, 1].map((k) => 0.5 * (2 * p1[k] + (-p0[k] + p2[k]) * u + (2 * p0[k] - 5 * p1[k] + 4 * p2[k] - p3[k]) * u2
        + (-p0[k] + 3 * p1[k] - 3 * p2[k] + p3[k]) * u3)));
    }
  }
  out.push(pts[pts.length - 1]);
  return out;
}

// A pointing hand, fingertip at the origin, ending there.
const HAND = smooth([
  [0, 0], [-5, 3], [-5.5, 9], [-5.5, 25], [-10, 22.5], [-14.5, 24], [-14, 29.5], [-7, 39], [-5, 46], [-1, 51],
  [14, 51], [19.5, 45], [20.5, 33], [20, 28.5], [17, 26], [14, 27.5], [13.5, 24.5], [11, 22], [8, 23], [7.5, 20.5],
  [5.5, 18.5], [5, 18], [5.5, 9], [5, 3], [0, 0],
]);

export function hand(x, y, s) {
  return [kit.stroke(HAND.map(([px, py]) => [x + px * s, y + py * s]), { w: 0.8, link: 0.3 })];
}
