// ElyTable: the format panel sets a cell's font and size.
import { kit as k } from '../lib/kit/index.js';
import { reveal, settle, shift, chevron, nearest } from './c06-util.js';

const X = 1000;
const Y = 70;
const WD = 840;
const HT = 430;
const TOP = Y + 48;
const HEAD = TOP + 36;
const ROW = 46;
const RULES = [1050, 1180, 1310, 1440];
const PANEL = 1580;
const GROW = 14;
const T = { font: 47.625, size: 47.8125, cell: 48.0 };
const thin = { w: 0.42 };
const ui = { w: 0.75 };

const head = [
  ...['A', 'B', 'C', 'D'].flatMap((c, i) => k.text(c, { x: (RULES[i] + (RULES[i + 1] ?? PANEL)) / 2, y: TOP + 26, size: 18, align: 'center', font: 'mono', w: 0.6 })),
];
const rowNums = [1, 2, 3, 4, 5, 6, 7].map((n) => k.text(String(n), { x: X + 25, y: HEAD + ROW * (n - 1) + 30, size: 18, align: 'center', font: 'mono', w: 0.6 }));
const cells = [
  ['Q1', '1,280'], ['Q2', '940'], ['Q3', '1,615'], ['Q4', '2,070'],
].map(([a, b], i) => [
  k.text(a, { x: RULES[0] + 14, y: HEAD + ROW * (i + 1) + 31, size: 20, w: 0.7 }),
  k.text(b, { x: RULES[2] - 14, y: HEAD + ROW * (i + 1) + 31, size: 20, align: 'right', w: 0.7 }),
]);

const field = (x, w) => k.rect(x, 0, w, 44, { ...ui, r: 6 });
const fontBox = k.move([field(1604, 212)], { y: 150 });
const sizeBox = k.move([field(1604, 92)], { y: 212 });
const marks = ['B', 'I', 'U'].map((c, i) => k.text(c, { x: 1620 + i * 46, y: 312, size: 22, w: 0.75 }));
const fontName = (name, font) => k.text(name, { x: 1618, y: 180, size: 20, font, w: 0.75 });
const oldName = fontName('Noto Sans', 'sans');
const newName = fontName('Noto Serif', 'serif');
const oldSize = k.text('11', { x: 1618, y: 242, size: 20, w: 0.75 });
const newSize = k.text('18', { x: 1618, y: 242, size: 20, w: 0.75 });
const oldCell = k.text('Budget', { x: RULES[0] + 14, y: HEAD + 31, size: 20, w: 0.75 });
const newCell = k.text('Budget', { x: RULES[0] + 12, y: HEAD + ROW + GROW - 13, size: 31, font: 'serif', w: 0.95 });

const frame = k.window(X, Y, WD, HT, { chrome: 'windows', title: 'Budget.xlsx', titleSize: 20, bar: 48, w: 0.8 });
const chrome = [nearest(frame[0], [X + WD, Y + HT / 2]), ...frame.slice(1)];

// Rows then columns, each one serpentine along the frame.
function grid(g) {
  const rows = [];
  for (let i = 0; i <= 7; i++) {
    const y = HEAD + ROW * i + (i ? g : 0);
    rows.push(...(i % 2 ? [[PANEL, y], [X, y]] : [[X, y], [PANEL, y]]));
  }
  const cols = [];
  [...RULES, PANEL].forEach((x, i) => cols.push(...(i % 2 ? [[x, Y + HT], [x, TOP]] : [[x, TOP], [x, Y + HT]])));
  return [k.stroke(rows, thin), k.stroke(cols, thin)];
}

function selection(g) {
  const x = RULES[0];
  const y = HEAD;
  const h = ROW + g;
  return [k.rect(x, y, RULES[1] - x, h, { w: 1.35 }), k.rect(RULES[1] - 4, y + h - 4, 8, 8, { w: 1.1 })];
}

function swap(old, next, at, u0 = 0.0625, u1 = 0.1875) {
  return (t) => [k.leave(old, k.span(t, at - u0, at)), reveal(next, k.ease.out(k.span(t, at, at + u1)))];
}

const nameSwap = swap(oldName, newName, T.font);
const sizeSwap = swap(oldSize, newSize, T.size);
const cellSwap = swap(oldCell, newCell, T.cell, 0.0625, 0.25);

export const table = {
  t0: 46.75,
  t1: 47.5625,
  strokes(t) {
    const g = GROW * settle(t - T.cell, 20, 8);
    const [oN, nN] = nameSwap(t);
    const [oS, nS] = sizeSwap(t);
    const [oC, nC] = cellSwap(t);
    const below = (list) => shift(list, 0, g);
    return [
      ...chrome, ...grid(g), ...head, ...rowNums[0], ...rowNums.slice(1).flatMap(below), ...cells.flat().flatMap(below),
      ...fontBox, chevron(1796, 172, 6, Math.PI / 2, ui), ...sizeBox, chevron(1678, 234, 6, Math.PI / 2, ui), ...marks.flat(),
      ...selection(g), ...oC, ...oS, ...oN, ...nN, ...nS, ...shift(nC, 0, g - GROW),
    ];
  },
  // Pointer: font field, size field, then back to the cell.
  cursor(t, tip, to) {
    if (t < this.t1) return null;
    return k.keys(t, [
      [this.t1, tip], [T.font - 0.0625, [1760, 176], 'out'], [T.size - 0.0625, [1664, 236], 'inOut'],
      [T.cell + 0.125, [1664, 236]], [48.5, to, 'inOut'],
    ]);
  },
  clicks: [[T.font, 1760, 176], [T.size, 1664, 236]],
  cam: { x: X + WD / 2, y: Y + HT / 2, w: 1000 },
};
