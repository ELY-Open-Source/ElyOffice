// Report.docx in line: window, banners, page, status bar.
import { kit } from '../lib/kit/index.js';

export const SIZE = 28;
export const WIN = { x: 400, y: 150, w: 1120, h: 780 };
export const STRIP = { x: 414, w: 1092, h: 64, r: 8 };
export const ROW0 = 214;
export const PITCH = 72;
export const BTN = 48;
export const PAGE = { x: 640, w: 640, foot: 866, clip: 846, top: 522 };
export const RULE = 874;
export const BASE = 912;
export const DIALOG = { x: 580, y: 436, w: 760, h: 232, r: 14 };

export const SHAPE = 1.35;
const TEXT = 1.2;

export const rowTop = (i) => ROW0 + PITCH * i;

export function words(str, x, y, o = {}) {
  return kit.text(str, { x, y, size: o.size ?? SIZE, w: o.w ?? TEXT, align: o.align ?? 'left' });
}

function warn(cx, cy) {
  return [
    kit.poly([[cx, cy - 12], [cx + 13, cy + 10], [cx - 13, cy + 10]], { closed: true, w: SHAPE }),
    kit.line(cx, cy - 4, cx, cy + 2, { w: SHAPE }),
    kit.circle(cx, cy + 6, 1.2, { w: SHAPE }),
  ];
}

function ring(cx, cy, bang) {
  const stem = bang ? kit.line(cx, cy - 6, cx, cy + 2, { w: SHAPE }) : kit.line(cx, cy - 1, cx, cy + 6, { w: SHAPE });
  const dot = kit.circle(cx, bang ? cy + 6 : cy - 5.5, 1.2, { w: SHAPE });
  return [kit.circle(cx, cy, 12, { w: SHAPE }), ...(bang ? [stem, dot] : [dot, stem])];
}

const ICONS = { warn, info: (x, y) => ring(x, y, false), stop: (x, y) => ring(x, y, true) };

export function buttonAt(label, x, y) {
  const w = kit.measure(label, { size: SIZE }).w + 40;
  return { w, rect: kit.rect(x, y, w, BTN, { r: 8, w: SHAPE }), text: words(label, x + w / 2, y + 34, { align: 'center' }) };
}

export function banner(i, label, icon, button) {
  const top = rowTop(i);
  const out = {
    top,
    strip: kit.rect(STRIP.x, top, STRIP.w, STRIP.h, { r: STRIP.r, w: SHAPE }),
    icon: ICONS[icon](STRIP.x + 32, top + 32),
    text: words(label, STRIP.x + 64, top + 42),
  };
  if (button) {
    const w = kit.measure(button, { size: SIZE }).w + 40;
    out.button = buttonAt(button, STRIP.x + STRIP.w - 8 - w, top + 8);
  }
  return out;
}

export function head(group, link) {
  return group.map((s, i) => (i ? s : { ...s, link }));
}

export const LINES = [
  [64, 300, 1.7],
  [110, 512], [144, 496], [178, 512], [212, 448], [246, 268],
  [300, 512], [334, 484], [368, 512], [402, 392],
  [456, 512], [490, 506], [524, 330],
];

const frame = kit.window(WIN.x, WIN.y, WIN.w, WIN.h, { chrome: 'windows', title: 'Report.docx', w: 1.5 });
const bar = frame[1];
export const chrome = [frame[0], ...frame.slice(6), ...frame.slice(2, 6), { ...bar, pts: [...bar.pts].reverse() }];

export const fonts = banner(0, 'Fonts not on this computer: Garamond. Lines and pages may move.', 'warn');
export const links = banner(1, 'Links not kept', 'info');
export const risk = banner(2, 'Saving over Report.docx may lose something', 'warn', 'Save a copy…');
export const stopped = banner(3, 'The document engine stopped', 'stop', 'Reopen documents');

export const pageNo = words('Page 1 of 4', WIN.x + WIN.w - 24, BASE, { align: 'right' });

export const linksAtFoot = { dx: WIN.x + 24 - (STRIP.x + 64), dy: BASE - (links.top + 42) };

export const reversed = (list) => list.map((s) => ({ ...s, pts: [...s.pts].reverse() })).reverse();
