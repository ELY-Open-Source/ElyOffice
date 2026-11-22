// ElyMark search: a found folder opens to show its files.
import { kit as k } from '../lib/kit/index.js';
import { reveal, settle, shift, chevron, folder, page } from './c06-util.js';

const X = 80;
const Y = 70;
const WD = 600;
const HT = 430;
const QUERY = 'drafts';
const TYPE = 55.0;
const KEY = 0.0625;
const FOUND = 55.4375;
const OPEN = 56.0;
const STAGGER = 0.125;
const ROW = 250;
const FILES = ['outline.md', 'chapter-1.md', 'chapter-2.md'];

const chrome = k.window(X, Y, WD, HT, { chrome: 'mac', bar: 48, w: 0.8 });
const pill = k.rect(110, 136, 540, 48, { r: 24, w: 0.8 });
const lens = [k.circle(142, 157, 9, { w: 0.8 }), k.line(149, 164, 157, 172, { w: 0.8 })];
const letters = [...QUERY].map((_, i) => k.text(QUERY.slice(0, i + 1), { x: 172, y: 168, size: 22, w: 0.75 }).filter((s) => s.g === i));
const hit = [folder(112, ROW - 20, { w: 0.8 }), ...k.text(QUERY, { x: 158, y: ROW, size: 22, w: 0.8 })];
const files = FILES.map((name, i) => {
  const y = ROW + 50 * (i + 1);
  return [page(150, y - 21, { w: 0.7 }), ...k.text(name, { x: 186, y, size: 22, w: 0.75 })];
});

export const search = {
  t0: 54.5,
  t1: 55.0,
  strokes(t) {
    const typed = letters.flatMap((g, i) => reveal(g, k.span(t, TYPE + KEY * i, TYPE + KEY * (i + 0.75))));
    const turn = (Math.PI / 2) * settle(t - OPEN, 24, 10);
    const rows = files.flatMap((f, i) => {
      const at = OPEN + 0.0625 + STAGGER * i;
      const drop = 1 - settle(t - at, 18, 8);
      return reveal(shift(f, 0, -26 * drop), k.ease.out(k.span(t, at, at + 0.25)));
    });
    return [
      ...chrome, pill, ...lens, ...typed,
      ...reveal([...hit, chevron(640, ROW - 8, 7, turn, { w: 0.9 })], k.ease.out(k.span(t, FOUND, FOUND + 0.25))),
      ...rows,
    ];
  },
  // Pointer: caret while typing, then the folder row.
  cursor(t, tip, exit) {
    if (t < this.t1) return null;
    const caret = 172 + k.measure(QUERY.slice(0, Math.round(k.clamp((t - TYPE) / KEY, 0, QUERY.length))), { size: 22 }).w;
    if (t < FOUND) return k.keys(t, [[this.t1, tip], [TYPE, [caret + 6, 160], 'out']]).map((v, i) => (i === 0 && t >= TYPE ? caret + 6 : v));
    return k.keys(t, [
      [FOUND, [caret + 6, 160]], [OPEN - 0.125, [600, ROW - 8], 'inOut'], [OPEN + 0.0625, [600, ROW - 8]],
      [OPEN + 0.5, [420, ROW + 150], 'inOut'], [57.25, [400, ROW + 170], 'sine'], [58.0, exit, 'in'],
    ]);
  },
  clicks: [[OPEN, 600, ROW - 8]],
  keys: [...QUERY].map((_, i) => TYPE + KEY * i),
  cam: { x: X + WD / 2, y: Y + HT / 2, w: 1000 },
};
