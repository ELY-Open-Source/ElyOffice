// Text and PDF: the line settles, then four PDF fixes.
import { kit } from '../lib/kit/index.js';
import { hero, writing } from './c09-type.js';
import { pdf, glyphs, copy, page, SAVE, EXPORT, COPY, SEL, CARET, DROP } from './c09-ui.js';
import { plan, reveal, travel, tangentIn, penTable, track } from './c09-draw.js';

const START = 78;
const END = 88;
const WRITE = writing(START + 0.75, 79.625);

const GROUPS = [
  { build: pdf, travel: [80.6875, 80.875], draw: [80.875, 81.3125], out: [0.35, 1] },
  { build: glyphs, travel: [82.25, 82.375], draw: [82.375, 83.375], from: SAVE.at, out: [0, 1], even: true },
  { build: copy, travel: [84.125, 84.25], draw: [84.25, 84.8125], from: [906, 842], out: [1, -0.3] },
  { build: page, travel: [85.625, 85.75], draw: [85.75, 86.625], from: CARET, out: [0.2, 1] },
].map((g) => {
  const first = g.build(g.draw[0]).draw;
  const times = g.even ? plan(first, ...g.draw, 0, 1) : plan(first, ...g.draw);
  return { ...g, times, entry: first[0].pts[0], tin: tangentIn(first[0]) };
});

const endOf = (list) => {
  for (let i = list.length - 1; i >= 0; i--) {
    const s = list[i];
    if (s.show[1] - s.show[0] > 1e-6 && kit.length(s) > 0.5) return s.pts.at(-1);
  }
  return list[0].pts[0];
};

function lay(t, trim) {
  const top = hero(t, WRITE);
  if (!trim) top[0] = { ...top[0], show: [0, 1] };
  const out = [...top];
  let last = endOf(top);
  for (const g of GROUPS) {
    const { draw, extra } = g.build(t);
    const lead = travel(g.from ?? last, g.entry, g.tin, g.out);
    const u = kit.ease.inOut(kit.span(t, ...g.travel));
    lead.show = u > 0 ? [0, u] : [0, 0];
    out.push(lead, ...reveal(draw, g.times, t, g.even ? 'linear' : 'sine'), ...extra);
    last = endOf(draw);
  }
  return out;
}

const finish = (g) => endOf(g.build(g.draw[1] + 1e-3).draw);
const [P2, P3, P4, P5] = GROUPS.map(finish);
const mid = SEL.y + SEL.h / 2;
const HOVER = [1806, 924];
// Two neighbours per shot, edges in the gaps.
const PAIRS = [[505, 784, 910], [980, 784, 910], [1372, 784, 910]];

const SPANS = [
  { from: 81.3125, to: 82.25, keys: [[81.3125, P2], [81.4375, SAVE.at], [81.625, SAVE.at], [81.8125, [SAVE.at[0] + 18, SAVE.at[1] + 12]], [81.9375, SAVE.at]] },
  { from: 83.375, to: 84.125, keys: [[83.375, P3], [83.4375, EXPORT.at, 'out'], [83.625, EXPORT.at], [84.0, [906, 842], 'out'], [84.125, [906, 842]]] },
  { from: 84.8125, to: 85.625, keys: [[84.8125, P4], [84.875, [SEL.x0 + 4, mid], 'out'], [COPY.release, [SEL.x1 - 4, mid]], [85.25, [SEL.x1 - 4, mid]], [85.3125, CARET], [85.625, CARET]] },
  { from: 86.625, to: END + 1, keys: [[86.625, P5], [86.875, HOVER, 'out'], [87.375, HOVER], [END, [1914, 884], 'in']] },
];

export default {
  id: 'c09', start: START, end: END, app: 'pdf',
  strokes: (t) => lay(t, false),
  pen: penTable('c09', START, END, (t) => kit.marks(lay(t, true)).total),
  cursor: (t) => track(SPANS, t),
  camera(t, k) {
    const keys = [
      [78.75, [960, 318, 1500]], [80.75, [960, 312, 1400], 'sine'], [81.3125, [345, 760, 1150]],
      [82.25, [345, 760, 1150]], [82.75, PAIRS[0]], [84.125, PAIRS[0]], [84.625, PAIRS[1]],
      [85.625, PAIRS[1]], [86.125, PAIRS[2]], [DROP.close, PAIRS[2]], [END - 0.25, [960, 540, 1920]],
    ];
    const [x, y, w] = k.keys(t, keys);
    return { x, y, w };
  },
};
