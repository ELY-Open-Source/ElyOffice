// The ElyMark page of s5: window, lines, picture.
import { glyphs, advance } from '../lib/glyph.js';
import { span, clamp, smooth } from '../lib/time.js';
import { noise1 } from '../lib/rand.js';
import { lay } from './s5-math.js';

const BURNT = [62, 30, 18];
const SCORCH = [128, 64, 32];
const CODE = [214, 194, 160];

export const COL = 520;
const BODY = 42;
const SRC = 34;
const ALT = 30;

let made = null;
export function page() {
  if (made) return made;
  const lead = 'Wrap display math in ';
  const code = glyphs(lead, { x: COL, y: 262, size: BODY });
  const cx = COL + advance(lead, BODY) + 10;
  const dollars = lay(['$', '$'], { x: cx, y: 264, size: 58 });
  const f1 = glyphs('$$ a^2 + b^2 = c^2 $$', { x: COL, y: 410, size: SRC });
  const f2 = glyphs('$$ e^{i\\pi} + 1 = 0 $$', { x: COL, y: 834, size: SRC });
  const pre = '![literal ';
  const ax = COL + advance(pre, ALT) + 2;
  const alt = [glyphs(pre, { x: COL, y: 524, size: ALT }), lay(['$', '$'], { x: ax, y: 525, size: 42 })];
  const bx = alt[1][1].at.x1 + 6;
  alt.push(glyphs('](sea.png)', { x: bx, y: 524, size: ALT }));
  const span0 = [cx - 12, dollars[1].at.x1 + 12];
  made = { code, dollars, span0, f1, f2, alt };
  return made;
}

const WINDOW = [
  [[372, 178], [960, 172], [1552, 175]],
  [[386, 160], [383, 560], [380, 1000]],
  [[1534, 160], [1538, 600], [1541, 1000]],
];

export function frame(ink, t, t0) {
  WINDOW.forEach((pts, i) => {
    const u = span(t, t0 + i * 0.08, t0 + 0.3 + i * 0.08);
    if (u > 0) ink.curve(pts, { id: `win${i}`, width: 2.6, upto: u, overshoot: 14, style: i ? 'ink' : 'charcoal', alpha: 0.85 });
  });
  const u = span(t, t0 + 0.2, t0 + 0.45);
  if (u > 0) ink.text('euler.md', { x: 960, y: 150, size: 28, align: 'center', upto: u, alpha: 0.75 });
}

export function codeLine(ink, t, t0, wriggle) {
  const { code, dollars, span0 } = page();
  const u = span(t, t0, t0 + 0.4);
  if (u <= 0) return;
  ink.write(code, { size: BODY, upto: u });
  const w = smooth(span(t, t0 + 0.3, t0 + 0.5));
  if (w <= 0) return;
  const [x0, x1] = span0;
  const blob = [[x0, 226], [(x0 + x1) / 2, 222], [x1, 227], [x1 + 3, 274], [(x0 + x1) / 2, 279], [x0 - 2, 275]];
  ink.wash([blob], { color: CODE, alpha: 0.55 * w, bleed: 3 });
  hand(ink, dollars, { upto: w, wriggle, t, width: 3.6, id: 'code' });
}

// Hand-drawn glyphs; wriggle shakes them.
export function hand(ink, list, o) {
  list.forEach((g, gi) => g.strokes.forEach((st, si) => {
    const shake = o.wriggle ?? 0;
    const pts = shake > 0 ? st.map(([x, y], i) => [x + shake * Math.sin(i * 0.9 + o.t * 34 + gi * 2),
      y + shake * 0.6 * Math.cos(i * 0.7 + o.t * 29)]) : st;
    ink.line(pts, { id: `${o.id}-${gi}-${si}`, width: o.width, upto: clamp((o.upto ?? 1) * 1.6 - si * 0.6),
      overshoot: 3, boil: 1, taper: [0.08, 0.2], color: o.color, alpha: o.alpha, style: o.style });
  }));
}

export const moveGlyph = (g, [dx, dy]) => (dx || dy
  ? { ...g, contours: g.contours.map((c) => c.map(([x, y]) => [x + dx, y + dy])) } : g);

// A source line that burns away glyph by glyph.
export function source(ink, list, o) {
  const u = span(o.t, o.t0, o.t0 + 0.35);
  if (u <= 0) return;
  const keep = [];
  list.forEach((g, i) => {
    if (!g.contours.length) return;
    const tb = o.burnAt?.(g, i) ?? Infinity;
    const off = o.offset?.(i) ?? [0, 0];
    const age = o.t - tb;
    const m = moveGlyph(g, off);
    if (age < 0) {
      keep.push(m);
      return;
    }
    const life = o.life ?? 0.4;
    if (age > life) {
      ink.smudge((g.box.x0 + g.box.x1) / 2, g.box.y1 - 12, 14, { color: BURNT, alpha: 0.05, blots: 2 });
      return;
    }
    const k = 1 - age / life;
    ink.wash(m.contours, { color: SCORCH, alpha: 0.7 * k, bleed: 4 });
    ink.write([m], { size: SRC, color: BURNT, alpha: 0.9 * k * k });
  });
  ink.write(keep, { size: SRC, upto: u, alpha: 0.82 });
}

export function altLine(ink, t, t0, wriggle) {
  const { alt } = page();
  const u = span(t, t0, t0 + 0.35);
  if (u <= 0) return;
  ink.write(alt[0], { size: ALT, upto: u, alpha: 0.8 });
  hand(ink, alt[1], { upto: span(t, t0 + 0.15, t0 + 0.4), wriggle, t, width: 2.8, id: 'alt', alpha: 0.85 });
  ink.write(alt[2], { size: ALT, upto: span(t, t0 + 0.2, t0 + 0.45), alpha: 0.8 });
}

function ragged(pts, amp, seed) {
  const out = [];
  pts.forEach(([x, y], i) => {
    const [nx, ny] = pts[(i + 1) % pts.length];
    for (let k = 0; k < 12; k++) {
      const u = k / 12;
      const j = i * 12 + k;
      out.push([x + (nx - x) * u + amp * noise1(j * 0.45, seed), y + (ny - y) * u + amp * noise1(j * 0.4, seed + 1)]);
    }
  });
  return out;
}

const SKY = ragged([[534, 556], [700, 548], [832, 560], [830, 648], [530, 652]], 5, 3);
const SEA = ragged([[528, 650], [700, 646], [834, 648], [826, 718], [690, 730], [536, 722]], 6, 5);
const DEEP = ragged([[530, 676], [700, 672], [830, 680], [820, 716], [540, 718]], 5, 7);
const SUN = Array.from({ length: 16 }, (_, i) => [772 + 24 * Math.cos(i * 0.39), 606 + 24 * Math.sin(i * 0.39)]);

export function picture(ink, t, t0) {
  const u = smooth(span(t, t0, t0 + 0.5));
  if (u <= 0) return;
  ink.wash([SKY], { color: [176, 198, 206], alpha: 0.55 * u, bleed: 7 });
  ink.wash([SUN], { color: [236, 142, 60], alpha: 0.8 * u, bleed: 3 });
  ink.wash([SEA], { color: [62, 108, 138], alpha: 0.72 * u, bleed: 6 });
  ink.wash([DEEP], { color: [40, 78, 104], alpha: 0.35 * u, bleed: 9 });
  const l = span(t, t0 + 0.2, t0 + 0.5);
  if (l <= 0) return;
  ink.line([[534, 650], [700, 645], [832, 649]], { id: 'horizon', width: 2, upto: l, alpha: 0.7 });
  for (let i = 0; i < 4; i++) {
    const y = 672 + i * 14;
    const x = 560 + ((i * 97) % 180);
    ink.curve([[x, y], [x + 18, y - 5], [x + 36, y], [x + 54, y - 4]], { id: `wave${i}`, width: 1.6, upto: l, alpha: 0.55 });
  }
  ink.line([[612, 642], [626, 590], [628, 642]], { id: 'sail', width: 2, upto: l, alpha: 0.8 });
  ink.line([[598, 643], [644, 642], [636, 652], [606, 652], [598, 643]], { id: 'hull', width: 2, upto: l, alpha: 0.8 });
}
