// ElyShow window, slide pane and the slide's still shapes.
import { kit } from '../lib/kit/index.js';

export const WIN = { x: 330, y: 100, w: 1520, h: 840, bar: 56 };
export const PANE = 620;
export const SLIDE = { x: 680, y: 236, w: 1110, h: 624 };
export const CENTRE = [SLIDE.x + SLIDE.w / 2, SLIDE.y + SLIDE.h / 2];
export const A = { x: 780, y: 320, w: 240, h: 150 };
export const B = { x: 1640, y: 395, r: 85 };
export const C = { x: 772, y: 790, size: 60, text: 'Roadmap', left: 780, right: 1029 };
export const D = { w: 230, h: 140, r: 16 };
export const THUMB = { x: 365, w: 220, h: 124, ys: [190, 344, 498] };
const SCALE = THUMB.w / SLIDE.w;

function arcPts(cx, cy, r, from, to) {
  const n = Math.max(6, Math.ceil((Math.abs(to - from) * r) / 4));
  const out = [];
  for (let i = 1; i <= n; i++) {
    const a = from + ((to - from) * i) / n;
    out.push([cx + r * Math.cos(a), cy + r * Math.sin(a)]);
  }
  return out;
}

// Rounded rectangle from its bottom centre, clockwise, closed.
export function roundRect(cx, cy, w, h, r, o = {}) {
  const x = cx - w / 2;
  const y = cy - h / 2;
  const q = Math.PI / 2;
  const pts = [
    [cx, y + h],
    [x + r, y + h],
    ...arcPts(x + r, y + h - r, r, q, 2 * q),
    [x, y + r],
    ...arcPts(x + r, y + r, r, 2 * q, 3 * q),
    [x + w - r, y],
    ...arcPts(x + w - r, y + r, r, -q, 0),
    [x + w, y + h - r],
    ...arcPts(x + w - r, y + h - r, r, 0, q),
    [cx, y + h],
  ];
  return kit.stroke(pts, o);
}

// Title bar rule, then the frame, as one stroke.
function frame({ x, y, w, h, bar }, r, o) {
  const q = Math.PI / 2;
  const pts = [
    [x + w, y + bar],
    [x, y + bar],
    [x, y + r],
    ...arcPts(x + r, y + r, r, 2 * q, 3 * q),
    [x + w - r, y],
    ...arcPts(x + w - r, y + r, r, -q, 0),
    [x + w, y + h - r],
    ...arcPts(x + w - r, y + h - r, r, 0, q),
    [x + r, y + h],
    ...arcPts(x + r, y + h - r, r, q, 2 * q),
    [x, y + bar],
  ];
  return kit.stroke(pts, o);
}

function title() {
  const { x, y, bar } = WIN;
  return kit.text('Pitch.pptx', { x: x + 22, y: y + bar / 2 + 10, size: 28 });
}

function controls() {
  const { x, y, w, bar } = WIN;
  const mid = y + bar / 2;
  const g = 7;
  const o = { w: 0.7 };
  return [
    kit.line(x + w - 156 - g, mid, x + w - 156 + g, mid, o),
    kit.rect(x + w - 102 - g, mid - g, g * 2, g * 2, o),
    kit.line(x + w - 46 - g, mid - g, x + w - 46 + g, mid + g, o),
    kit.line(x + w - 46 + g, mid - g, x + w - 46 - g, mid + g, o),
  ];
}

// Rectangle from its top right corner, clockwise.
function box(x, y, w, h, r, o) {
  const q = Math.PI / 2;
  const pts = [
    [x + w, y + r],
    [x + w, y + h - r],
    ...arcPts(x + w - r, y + h - r, r, 0, q),
    [x + r, y + h],
    ...arcPts(x + r, y + h - r, r, q, 2 * q),
    [x, y + r],
    ...arcPts(x + r, y + r, r, 2 * q, 3 * q),
    [x + w - r, y],
    ...arcPts(x + w - r, y + r, r, -q, 0),
  ];
  return kit.stroke(pts, o);
}

export function mini(px, py, y0) {
  return [THUMB.x + (px - SLIDE.x) * SCALE, y0 + (py - SLIDE.y) * SCALE];
}

function thumbs(dc) {
  const { x, w, h, ys } = THUMB;
  const o = { w: 0.55 };
  const [ax, ay] = mini(A.x, A.y, ys[0]);
  const [bx, by] = mini(B.x, B.y, ys[0]);
  const [c0x, cy] = mini(C.left, C.y - 18, ys[0]);
  const [c1x] = mini(C.right, C.y, ys[0]);
  const [dx, dy] = mini(dc[0], dc[1], ys[0]);
  return [
    box(x, ys[0], w, h, 4, { w: 1.25 }),
    kit.circle(bx, by, B.r * SCALE, o),
    roundRect(dx, dy, D.w * SCALE, D.h * SCALE, 4, o),
    box(ax, ay, A.w * SCALE, A.h * SCALE, 0, o),
    kit.line(c0x, cy, c1x, cy, o),
    box(x, ys[1], w, h, 4, { w: 0.7 }),
    kit.line(x + 180, ys[1] + 48, x + 40, ys[1] + 48, o),
    kit.line(x + 70, ys[1] + 78, x + 150, ys[1] + 78, o),
    box(x, ys[2], w, h, 4, { w: 0.7 }),
    kit.line(x + 186, ys[2] + 62, x + 122, ys[2] + 62, o),
    box(x + 36, ys[2] + 30, 64, 64, 0, o),
  ].map((st) => ({ ...st, link: 0.35 }));
}

// Slide frame from its top right, counter-clockwise.
function slide() {
  const { x, y, w, h } = SLIDE;
  return kit.stroke([[x + w, y], [x, y], [x, y + h], [x + w, y + h], [x + w, y]], { w: 0.95 });
}

// Rectangle from its top left, counter-clockwise.
function rectA() {
  const { x, y, w, h } = A;
  return kit.stroke([[x, y], [x, y + h], [x + w, y + h], [x + w, y], [x, y]], { w: 1.15 });
}

let hush = null;

// Long hops between still strokes fade to hairlines.
function quiet(list) {
  if (!hush) {
    const m = kit.marks(list).list;
    hush = m.map((r, i) => (i ? Math.min(1, 150 / Math.max(1, r.start - m[i - 1].end)) : 1));
  }
  return list.map((st, i) => ({ ...st, link: Math.min(st.link ?? 1, hush[i]) }));
}

export function stage(dc) {
  return quiet([
    frame(WIN, 8, { w: 0.8 }),
    ...title(),
    ...thumbs(dc),
    kit.line(PANE, WIN.y + WIN.h, PANE, WIN.y + WIN.bar, { w: 0.6 }),
    ...controls(),
    slide(),
    kit.circle(B.x, B.y, B.r, { w: 1.15 }),
    rectA(),
    ...kit.text(C.text, { x: C.x, y: C.y, size: C.size }),
  ]);
}
