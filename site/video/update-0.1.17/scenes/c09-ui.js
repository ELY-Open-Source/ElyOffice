// The four windows below the line: PDF, glyphs, copy, page.
import { kit } from '../lib/kit/index.js';
import { LINE } from './c09-type.js';

const k = kit;
const mid = (size) => 0.41 * size;
const soft = (list, link = 0.4) => list.map((s) => ({ ...s, link }));
const rev = (s) => ({ ...s, pts: [...s.pts].reverse() });

// ElyPDF: a file saved twice keeps its size.
export const SAVE = { at: [134, 684], press: [81.5, 82.0] };

function floppy(cx, cy, s, o) {
  const x = cx - s / 2;
  const y = cy - s / 2;
  const c = s * 0.24;
  return [
    k.poly([[x, y + s], [x, y], [x + s - c, y], [x + s, y + c], [x + s, y + s]], { ...o, closed: true }),
    k.poly([[x + s * 0.2, y + s], [x + s * 0.2, y + s * 0.6], [x + s * 0.8, y + s * 0.6], [x + s * 0.8, y + s]], o),
    k.poly([[x + s * 0.66, y], [x + s * 0.66, y + s * 0.3], [x + s * 0.24, y + s * 0.3], [x + s * 0.24, y]], o),
  ];
}

function hop(t) {
  let dy = 0;
  for (const p of SAVE.press) {
    const u = t - p;
    if (u < 0 || u > 0.7) continue;
    dy += k.keys(u, [[0, 0], [0.06, 4, 'out'], [0.18, -15, 'out'], [0.32, 0, 'in']]) + k.spring(u - 0.32, -3.5, 34, 12);
  }
  return dy;
}

const PDF_HEAD = soft([
  ...k.window(100, 592, 420, 318, { chrome: 'mac', title: '春眠.pdf', app: 'pdf' }),
  k.rect(240, 668, 140, 176, { r: 3, app: 'pdf' }),
  k.line(262, 706, 358, 706, { app: 'pdf', w: 0.7 }),
  k.line(358, 732, 262, 732, { app: 'pdf', w: 0.7 }),
  k.line(262, 758, 330, 758, { app: 'pdf', w: 0.7 }),
  k.line(100, 862, 520, 862, { app: 'pdf' }),
]);
const SIZE_TEXT = k.text('2.4 MB', { x: 500, y: 899, size: 34, align: 'right', app: 'pdf' });
const FLOPPY = soft(floppy(SAVE.at[0], SAVE.at[1], 32, { app: 'pdf' }));

export function pdf(t) {
  const extra = SAVE.press.flatMap((p) => {
    const u = k.span(t, p + 0.04, p + 0.24);
    const head = k.clamp(k.ease.out(u) * 1.4);
    const tail = k.clamp(k.ease.out(u) * 1.4 - 0.4);
    return [
      k.line(124, 886, 380, 886, { w: 0.5, app: 'pdf', show: u > 0 && head > tail ? [tail, head] : [0, 0] }),
      k.ripple(SAVE.at[0], SAVE.at[1], k.span(t, p, p + 0.24), { r: 20, app: 'pdf' }),
    ];
  });
  const press = SAVE.press.reduce((a, p) => a + k.keys(t - p, [[-0.0625, 0], [0, -0.12, 'in'], [0.125, 0, 'out']]), 0);
  const floppy = k.move(FLOPPY, { ox: SAVE.at[0], oy: SAVE.at[1], s: 1 + press });
  return { draw: [...PDF_HEAD, ...k.move(SIZE_TEXT, { y: hop(t) }), ...floppy], extra };
}

// ElyMark export: a font's grid collapses to the five used.
export const EXPORT = { at: [740, 940], press: 83.5, land: 84.0 };
const CELL = 64;
const GRID = { x: 580, y: 604, cols: 5, rows: 4 };
const USED = new Map([[1, 0], [8, 1], [10, 2], [14, 3], [17, 4]]);
const FILL = '一二三十人八入口日月山川工土王';
const ROW = { y: GRID.y + GRID.rows * CELL / 2, size: 54, pitch: 62 };
const W5 = GRID.cols * CELL;
const H4 = GRID.rows * CELL;

const cells = [...Array(GRID.cols * GRID.rows)].map((_, i) => {
  const cx = GRID.x + (i % GRID.cols + 0.5) * CELL;
  const cy = GRID.y + (Math.floor(i / GRID.cols) + 0.5) * CELL;
  const used = USED.get(i);
  const ch = used === undefined ? FILL[i - [...USED.keys()].filter((u) => u < i).length] : [...LINE][used];
  return { cx, cy, used, strokes: k.text(ch, { x: cx, y: cy + mid(44), size: 44, align: 'center', app: 'mark', link: 0.4 }) };
});
const ICON = soft(k.icon('pdf', EXPORT.at[0], EXPORT.at[1], 92, { app: 'mark' }));
const OUTER = { ...k.rect(GRID.x, GRID.y, W5, H4, { r: 6, app: 'mark' }), link: 0.4 };
const HUG = k.rect(740 - 2.5 * ROW.pitch - 14, ROW.y - 46, 5 * ROW.pitch + 28, 92, { r: 14 });
const RULES = soft([
  ...[1, 2, 3].map((r) => {
    const s = k.line(GRID.x, GRID.y + r * CELL, GRID.x + W5, GRID.y + r * CELL, { w: 0.45, app: 'mark' });
    return r % 2 ? s : rev(s);
  }),
  ...[4, 3, 2, 1].map((c) => {
    const s = k.line(GRID.x + c * CELL, GRID.y, GRID.x + c * CELL, GRID.y + H4, { w: 0.45, app: 'mark' });
    return c % 2 ? s : rev(s);
  }),
]);

function swing(u) {
  return u <= 0 ? 0 : u >= 1 ? 1 : k.ease.back(u);
}

export function glyphs(t) {
  const p = EXPORT.press;
  const outer = { ...k.morph(OUTER, HUG, swing(k.span(t, p + 0.25, EXPORT.land))), link: 0.4 };
  const rules = RULES.map((r, n) => {
    const u = k.ease.in(k.span(t, p + 0.125 + n * 0.015, p + 0.3 + n * 0.015));
    return { ...r, show: u >= 1 ? [0, 0] : [u / 2, 1 - u / 2] };
  });
  const glyph = cells.flatMap((c) => {
    if (c.used === undefined) {
      const near = 1 - Math.min(1, Math.hypot(c.cx - 740, c.cy - ROW.y) / 220);
      return k.leave(c.strokes, k.ease.in(k.span(t, p + 0.125 + 0.1 * near, p + 0.3 + 0.1 * near)));
    }
    const u = k.span(t, p + 0.25 + c.used * 0.025, EXPORT.land - (4 - c.used) * 0.012);
    const e = swing(u);
    const tx = 740 + (c.used - 2) * ROW.pitch;
    return k.move(c.strokes, {
      ox: c.cx, oy: c.cy, s: 1 + (ROW.size / 44 - 1) * k.clamp(e),
      x: (tx - c.cx) * e, y: (ROW.y - c.cy) * e - 34 * Math.sin(Math.PI * k.clamp(u)),
    });
  });
  const extra = [k.ripple(EXPORT.at[0], EXPORT.at[1] - 6, k.span(t, p, p + 0.24), { r: 22, app: 'mark' })];
  return { draw: [...ICON, outer, ...rules, ...glyph], extra };
}

// A Japanese line selected in a PDF, pasted whole.
export const COPY = { press: 84.875, release: 85.1875, field: 85.375, paste: 85.5 };
const JP = { x: 1180, y: 736 + mid(40), size: 40, font: 'jp' };
const JP_W = k.measure('春眠暁を覚えず', JP).w;
export const SEL = { x0: JP.x - JP_W / 2 - 12, x1: JP.x + JP_W / 2 + 12, y: 736 - 32, h: 64 };
const FIELD = { x: 970, y: 860, w: 420, h: 76 };
export const CARET = [FIELD.x + 26, FIELD.y + FIELD.h / 2];
const PASTED = k.text('春眠暁を覚えず', { x: CARET[0] + 4, y: CARET[1] + mid(40), size: 40, font: 'jp', app: 'mark' });
const COPY_DRAW = [
  ...soft(k.window(970, 590, 420, 236, { chrome: 'linux', app: 'pdf' })),
  ...k.text('春眠暁を覚えず', { ...JP, align: 'center', app: 'mark', link: 0.4 }),
  { ...k.rect(FIELD.x, FIELD.y, FIELD.w, FIELD.h, { r: 14, app: 'mark' }), link: 0.4 },
];

export function copy(t) {
  const drag = k.ease.inOut(k.span(t, COPY.press, COPY.release));
  const sel = { ...k.rect(SEL.x0, SEL.y, Math.max(2, (SEL.x1 - SEL.x0) * drag), SEL.h, { r: 6, app: 'mark', w: 0.6 }), link: 0.4 };
  if (t < COPY.press) sel.show = [0, 0];
  const pop = t >= COPY.paste ? 1 + k.spring(t - COPY.paste, 0.07, 26, 9) : 1;
  const pasted = k.move(PASTED, { ox: CARET[0], oy: CARET[1], s: pop }).map((s) => (t >= COPY.paste ? s : { ...s, show: [0, 0] }));
  const caretX = t >= COPY.paste ? CARET[0] + 8 + JP_W : CARET[0];
  const caret = k.line(caretX, CARET[1] - 22, caretX, CARET[1] + 22, { app: 'mark', show: t >= COPY.field ? [0, 1] : [0, 0] });
  const ripples = [COPY.press, COPY.field].map((p, n) => {
    const at = n ? CARET : [SEL.x0 + 4, SEL.y + SEL.h / 2];
    return k.ripple(at[0], at[1], k.span(t, p, p + 0.24), { r: 18, app: 'mark' });
  });
  return { draw: COPY_DRAW, extra: [sel, { ...caret, link: 0.4 }, ...pasted, ...ripples] };
}

// ElyDoc: what Word cached in an EQ field drops away.
export const DROP = { at: 87.0, close: 87.25 };
const PAGE = { x: 1466, y: 584, w: 308, h: 400 };
const L = PAGE.x + 32;
const R = PAGE.x + PAGE.w - 32;
const FOLD = [R + 4, PAGE.y];
const CUT = [PAGE.x + PAGE.w, PAGE.y + 36];
const PAGE_DRAW = soft([
  k.poly([[PAGE.x, PAGE.y + PAGE.h], [PAGE.x, PAGE.y], FOLD, CUT, [PAGE.x + PAGE.w, PAGE.y + PAGE.h]], { closed: true, app: 'doc' }),
  k.line(L, 632, L + 150, 632, { app: 'doc', w: 1.3 }),
  k.poly([FOLD, [FOLD[0], CUT[1]], CUT], { app: 'doc', w: 0.7 }),
  k.line(R, 666, L, 666, { app: 'doc', w: 0.7 }),
  k.line(L, 694, R, 694, { app: 'doc', w: 0.7 }),
  k.line(R - 52, 722, L, 722, { app: 'doc', w: 0.7 }),
  ...k.text('x =', { x: L + 6, y: 788, size: 34, font: 'serif', app: 'doc' }),
  ...k.text('a', { x: L + 106, y: 768, size: 30, font: 'serif', align: 'center', app: 'doc' }),
  k.line(L + 86, 777, L + 126, 777, { app: 'doc' }),
  ...k.text('b', { x: L + 106, y: 810, size: 30, font: 'serif', align: 'center', app: 'doc' }),
]);
const STRAY = [
  soft([k.rect(L, 830, 112, 64, { r: 3, app: 'doc' }), k.line(L + 14, 852, L + 96, 852, { app: 'doc', w: 0.6 }),
    k.line(L + 14, 874, L + 74, 874, { app: 'doc', w: 0.6 })]),
  soft([k.rect(L + 128, 830, 116, 64, { r: 3, app: 'doc' }),
    k.poly([[L + 138, 886], [L + 168, 852], [L + 190, 874], [L + 204, 862], [L + 234, 886]], { app: 'doc', w: 0.8 }),
    k.circle(L + 218, 848, 7, { app: 'doc', w: 0.8 })]),
];
const BELOW = soft([k.line(L, 924, R, 924, { app: 'doc', w: 0.7 }), k.line(R - 70, 952, L, 952, { app: 'doc', w: 0.7 })].map(rev));
const RISE = 92;

export function page(t) {
  const stray = STRAY.flatMap((box, n) => {
    const u = t - DROP.at - n * 0.0625;
    const lift = k.keys(u, [[-0.125, 0], [0, -6, 'out']]);
    const fall = u > 0 ? 0.5 * 2600 * u * u : 0;
    const [ox, oy] = n ? [L + 186, 862] : [L + 56, 862];
    const turn = (n ? 1 : -1) * 0.5 * k.ease.in(k.span(u, 0, 0.4));
    const moved = k.move(box, { ox, oy, r: turn, y: lift + fall, x: (n ? 1 : -1) * 40 * k.span(u, 0, 0.4) });
    return k.leave(moved, k.ease.in(k.span(u, 0.06, 0.36)));
  });
  const rise = -RISE * k.ease.back(k.span(t, DROP.close - 0.0625, DROP.close + 0.1875));
  return { draw: [...PAGE_DRAW, ...stray, ...k.move(BELOW, { y: rise })], extra: [] };
}
