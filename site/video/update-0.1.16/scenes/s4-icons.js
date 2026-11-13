// The seven app icons, after their masters, by hand.
import { rgba } from '../lib/stroke.js';
import { signed } from '../lib/rand.js';

const WHITE = [252, 248, 238];
const PAGE = [[40, 16], [112, 16], [160, 64], [160, 168], [152, 176], [40, 176], [32, 168], [32, 24]];
const FOLD = [[112, 16], [112, 56], [120, 64], [160, 64]];

export function rect(x, y, w, h, r = 0) {
  if (!r) return [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
  const out = [];
  const corners = [[x + w - r, y + r, -Math.PI / 2], [x + w - r, y + h - r, 0], [x + r, y + h - r, Math.PI / 2], [x + r, y + r, Math.PI]];
  for (const [cx, cy, a0] of corners) {
    for (let k = 0; k <= 3; k++) {
      const a = a0 + (k / 3) * (Math.PI / 2);
      out.push([cx + Math.cos(a) * r, cy + Math.sin(a) * r]);
    }
  }
  return out;
}

export function circle(cx, cy, r, n = 18) {
  return Array.from({ length: n }, (_, i) => {
    const a = (i / n) * Math.PI * 2;
    return [cx + Math.cos(a) * r, cy + Math.sin(a) * r];
  });
}

export function solid(ink, shape, color, alpha = 0.97, amp = 0.9) {
  const ctx = ink.ctx;
  const s = ink.seed();
  ctx.beginPath();
  shape.forEach((c, k) => {
    c.forEach(([x, y], i) => {
      const p = [x + signed(s, k, i, 1) * amp, y + signed(s, k, i, 2) * amp];
      if (i) ctx.lineTo(...p);
      else ctx.moveTo(...p);
    });
    ctx.closePath();
  });
  ctx.fillStyle = rgba(color, alpha);
  ctx.fill('evenodd');
}

function body(ink, shape, color, o = {}) {
  if (o.opaque) solid(ink, [shape], color, 0.96);
  else {
    ink.wash([shape], { color, alpha: 0.95, bleed: 3 });
    ink.wash([shape], { color, alpha: 0.6, bleed: 1.5 });
  }
  ink.line(shape, { width: 4.4, closed: true, overshoot: 7 });
}

function fold(ink, color) {
  solid(ink, [FOLD], color);
  ink.line(FOLD.slice(0, 3), { width: 3, overshoot: 4 });
}

const bar = (ink, a, b, width, color) => ink.line([a, b], { width, color, alpha: 0.97, taper: [0.02, 0.02], overshoot: 0, boil: 1 });

function ring(ink, cx, cy, fill, rim) {
  solid(ink, [circle(cx, cy, 24)], fill);
  ink.line(circle(cx, cy, 24), { width: 3, closed: true, overshoot: 5 });
  solid(ink, [circle(cx, cy, 11, 12)], rim);
}

function pie(cx, cy, r) {
  const pts = [[cx, cy]];
  for (let k = 0; k <= 12; k++) {
    const a = -Math.PI / 2 - (k / 12) * 1.5 * Math.PI;
    pts.push([cx + Math.cos(a) * r, cy + Math.sin(a) * r]);
  }
  return pts;
}

const BLUE = [61, 127, 229];
const GREEN = [46, 158, 91];
const ORANGE = [240, 123, 42];
const RED = [224, 52, 79];
const SKY = [30, 143, 224];
const VIOLET = [126, 87, 194];
const AMBER = [242, 169, 0];

function doc(ink) {
  body(ink, PAGE, BLUE);
  fold(ink, [148, 185, 241]);
  bar(ink, [60, 90], [116, 90], 12, WHITE);
  for (const y of [112, 128, 144]) bar(ink, [60, y], [132, y], 8, WHITE);
  bar(ink, [60, 160], [100, 160], 8, [168, 197, 243]);
}

export function table(ink) {
  body(ink, PAGE, GREEN);
  fold(ink, [140, 202, 165]);
  solid(ink, [rect(52, 92, 88, 64), rect(60, 108, 32, 12), rect(100, 108, 32, 12), rect(60, 128, 32, 12), rect(100, 128, 32, 12)], WHITE);
  solid(ink, [rect(100, 128, 32, 12)], [255, 213, 79]);
}

function show(ink) {
  bar(ink, [96, 136], [64, 176], 12, [93, 93, 93]);
  bar(ink, [96, 136], [128, 176], 12, [93, 93, 93]);
  body(ink, rect(16, 28, 160, 112, 10), ORANGE);
  solid(ink, [rect(20, 30, 152, 13, 6)], [244, 163, 106]);
  solid(ink, [rect(32, 52, 128, 72, 4)], WHITE);
  for (const [x, y, h] of [[48, 96, 16], [72, 80, 32], [96, 66, 46]]) solid(ink, [rect(x, y, 16, h, 2)], ORANGE);
  solid(ink, [pie(136, 82, 16)], [93, 127, 201]);
}

function pdf(ink) {
  body(ink, PAGE, RED);
  fold(ink, [238, 143, 158]);
  solid(ink, [rect(52, 88, 88, 60, 6)], WHITE);
  solid(ink, [[[60, 140], [82, 112], [98, 128], [108, 118], [132, 140]]], RED);
  solid(ink, [circle(112, 104, 7, 10)], [236, 133, 149]);
  bar(ink, [56, 160], [104, 160], 8, [241, 164, 176]);
}

function mail(ink) {
  body(ink, rect(16, 40, 160, 112, 12), SKY);
  solid(ink, [[[28, 42], [164, 42], [104, 92], [96, 96], [88, 92]]], [109, 182, 235]);
  ink.line([[30, 54], [96, 108], [162, 54]], { width: 12, color: WHITE, alpha: 0.97, overshoot: 0, taper: [0.02, 0.02] });
  bar(ink, [32, 138], [72, 104], 8, [109, 182, 235]);
  bar(ink, [160, 138], [120, 104], 8, [109, 182, 235]);
}

function tree(ink) {
  bar(ink, [64, 44], [64, 148], 20, VIOLET);
  ink.curve([[64, 128], [70, 100], [122, 98], [128, 72]], { width: 20, color: VIOLET, alpha: 0.97, overshoot: 0, taper: [0.02, 0.02] });
  ring(ink, 64, 40, VIOLET, WHITE);
  ring(ink, 128, 64, VIOLET, WHITE);
  ring(ink, 64, 152, AMBER, WHITE);
}

function mark(ink) {
  body(ink, rect(28, 24, 136, 152, 10), [240, 240, 238], { opaque: true });
  solid(ink, [[[30, 50], [30, 34], [38, 26], [154, 26], [162, 34], [162, 50]]], [79, 79, 79]);
  for (let k = 0; k < 5; k++) solid(ink, [rect(46 + k * 24, 32, 10, 14, 5)], WHITE);
  solid(ink, [[[44, 156], [44, 76], [66, 76], [82, 100], [98, 76], [120, 76], [120, 156], [100, 156], [100, 110], [82, 136], [64, 110], [64, 156]]], [62, 74, 92]);
  solid(ink, [[[128, 76], [148, 76], [148, 124], [164, 124], [138, 158], [112, 124], [128, 124]]], AMBER);
}

export const ICONS = [
  { paint: doc, heart: [96, 150], tone: BLUE },
  { paint: table, heart: [96, 150], tone: GREEN },
  { paint: show, heart: [96, 118], tone: ORANGE },
  { paint: pdf, heart: [96, 150], tone: RED },
  { paint: mail, heart: [96, 132], tone: SKY },
  { paint: tree, heart: [100, 120], tone: VIOLET },
  { paint: mark, heart: [96, 150], tone: AMBER },
];

export function place(ink, { x, y, a, s }, paint) {
  const ctx = ink.ctx;
  ctx.save();
  ctx.translate(x, y);
  ctx.rotate(a);
  ctx.scale(s, s);
  ctx.translate(-96, -96);
  paint(ink);
  ctx.restore();
}

export function local({ x, y, a, s }, [u, v]) {
  const dx = (u - 96) * s;
  const dy = (v - 96) * s;
  return [x + dx * Math.cos(a) - dy * Math.sin(a), y + dx * Math.sin(a) + dy * Math.cos(a)];
}
