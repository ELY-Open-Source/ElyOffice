// c11 drawings: dialog, graphics card, engines, process tree, mail.
import { kit } from '../lib/kit/index.js';

const { rect, line, poly, circle, text, stroke, measure } = kit;

export const LINE = 0.9;
export const INK = 0.55;
const SIZE = 21;

// A: the failed save.
export function dialog() {
  const o = { app: 'doc' };
  const t = { ...o, w: INK };
  const body = [
    rect(170, 140, 600, 280, { ...o, r: 14, w: LINE }),
    ...text("Q3 Report.docx wasn't saved.", { ...t, x: 214, y: 218, size: 26 }),
    ...text('The disk is full.', { ...t, x: 214, y: 266, size: 22 }),
    ...text('Cancel', { ...t, x: 512, y: 375, size: 22, align: 'center' }),
  ];
  const save = [
    rect(590, 342, 152, 50, { ...o, r: 10, w: LINE * 1.25 }),
    ...text('Save As…', { ...t, x: 666, y: 375, size: 22, align: 'center' }),
  ];
  return { body, save, saveAt: [666, 367] };
}

// B: a graphics card, struck through; the document to open.
export function card() {
  const o = { app: 'doc', w: LINE };
  const pins = [];
  for (let x = 194; x < 346; x += 16) pins.push([x, 720], [x, 733], [x + 8, 733], [x + 8, 720]);
  return [
    rect(150, 610, 240, 110, { ...o, r: 10 }),
    circle(225, 665, 34, o),
    circle(225, 665, 9, o),
    circle(315, 665, 34, o),
    circle(315, 665, 9, o),
    poly([[150, 598], [134, 598], [134, 752], [146, 752]], o),
    stroke(pins, { ...o, w: LINE * 0.8 }),
    line(116, 586, 420, 764, { ...o, w: LINE * 1.5 }),
  ];
}

export const FILE_AT = [272, 868];
export const fileIcon = () => kit.icon('doc', FILE_AT[0], FILE_AT[1], 128, { app: 'doc', w: LINE });

export const WIN_AT = [600, 790];
export function docWindow() {
  const o = { app: 'doc', w: LINE };
  const out = [...kit.window(420, 610, 360, 360, { chrome: 'windows', bar: 44, ...o }), rect(500, 676, 200, 274, { ...o, r: 2 })];
  out.push(line(524, 716, 630, 716, { ...o, w: LINE * 1.3 }));
  [152, 146, 152, 124, 76].forEach((w, i) => out.push(line(524, 754 + i * 24, 524 + w, 754 + i * 24, { ...o, w: INK })));
  [152, 98].forEach((w, i) => out.push(line(524, 898 + i * 24, 524 + w, 898 + i * 24, { ...o, w: INK })));
  return out;
}

// C: AppImage, sign-in, package, three engines.
export const SOURCES = { image: [1093, 300], pkg: [1640, 300] };
export const GEARS = [[1255, 400, 'doc'], [1365, 400, 'table'], [1475, 400, 'show']];
export const LOCK_AT = [1365, 200];

export function sources() {
  const o = { w: LINE };
  const t = { w: INK };
  const hex = [-90, -30, 30, 90, 150, 210].map((a) => [1640 + 56 * Math.cos((a * Math.PI) / 180), 184 + 56 * Math.sin((a * Math.PI) / 180)]);
  return {
    image: [
      poly([[1052, 128], [1108, 128], [1134, 154], [1134, 236], [1052, 236]], { ...o, closed: true }),
      poly([[1108, 128], [1108, 154], [1134, 154]], o),
      ...text('ElyOffice.AppImage', { ...t, x: 1093, y: 280, size: SIZE, align: 'center' }),
    ],
    pkg: [
      poly(hex, { ...o, closed: true }),
      poly([hex[5], [1640, 184], hex[1]], o),
      line(1640, 184, 1640, 240, o),
      ...text('ely-office', { ...t, x: 1640, y: 280, size: SIZE, align: 'center' }),
    ],
  };
}

export function lock(open) {
  const o = { w: LINE };
  const lift = -14 * open;
  const arcPts = [];
  for (let i = 0; i <= 24; i++) {
    const a = Math.PI + (Math.PI * i) / 24;
    arcPts.push([1365 + 16 * Math.cos(a), 166 + lift + 16 * Math.sin(a)]);
  }
  return [
    stroke([[1349, 178 + lift * 0.4], ...arcPts, [1381, 166 + lift], [1381, 178 + lift]], o),
    rect(1338, 178, 54, 44, { ...o, r: 8 }),
    circle(1365, 197, 5, o),
  ];
}

export function gear(cx, cy, r, spin, app) {
  const n = 8;
  const pts = [];
  for (let i = 0; i < n; i++) {
    const a = -Math.PI / 2 + spin + ((i - 0.5) * 2 * Math.PI) / n;
    const q = (2 * Math.PI) / n;
    for (const [da, rr] of [[0.06, r - 6], [0.2, r + 1], [0.8, r + 1], [0.94, r - 6]]) {
      pts.push([cx + rr * Math.cos(a + da * q), cy + rr * Math.sin(a + da * q)]);
    }
  }
  return poly(pts, { closed: true, w: LINE, app });
}

export const cable = (from, to, slack) => stroke(bend(from, to, slack), { w: INK });

function bend(a, b, slack) {
  const out = [];
  for (let i = 0; i <= 24; i++) {
    const u = i / 24;
    const v = 1 - u;
    const p1 = [a[0], a[1] - slack * 0.6];
    const p2 = [b[0], b[1] + slack];
    out.push([
      v * v * v * a[0] + 3 * v * v * u * p1[0] + 3 * v * u * u * p2[0] + u * u * u * b[0],
      v * v * v * a[1] + 3 * v * v * u * p1[1] + 3 * v * u * u * p2[1] + u * u * u * b[1],
    ]);
  }
  return out;
}

// D: explorer, ElyTree, Git and its helpers.
export const STOP_AT = [1148, 763];
export function processes() {
  const t = { app: 'tree', w: INK };
  const o = { app: 'tree', w: LINE * 0.7 };
  const name = (s, x, y) => text(s, { ...t, x, y, size: SIZE });
  const elbow = (x, y0, y1, x1) => poly([[x, y0], [x, y1], [x1, y1]], o);
  const keep = [...name('explorer.exe', 960, 640), elbow(971, 652, 699, 992), ...name('ely-tree.exe', 1000, 705)];
  const end = [
    elbow(1011, 717, 764, 1032), ...name('git.exe', 1040, 770),
    circle(STOP_AT[0], STOP_AT[1], 17, { app: 'tree', w: LINE }),
    rect(STOP_AT[0] - 5.5, STOP_AT[1] - 5.5, 11, 11, { app: 'tree', w: LINE }),
    elbow(1051, 782, 829, 1072), ...name('git-remote-https.exe', 1080, 835),
    elbow(1051, 829, 894, 1072), ...name('git-lfs.exe', 1080, 900),
  ];
  return { keep, end };
}

// E: ElyMail with a sidebar that narrows.
export const MAIL = { x: 1380, y: 590, w: 460, h: 380, bar: 44 };
export const DIVIDER = { from: 1610, to: 1520 };
const ACCOUNTS = [['Outlook', '12'], ['Gmail', '3'], ['Personal', '128']];
const DOTS = text('…', { size: SIZE, w: INK, app: 'mail' });
const DOTS_W = measure('…', { size: SIZE }).w;

const rows = ACCOUNTS.map(([nm, n], i) => {
  const y = 678 + i * 52;
  const chars = [...nm];
  return {
    y,
    name: text(nm, { x: MAIL.x + 20, y, size: SIZE, w: INK, app: 'mail' }),
    widths: chars.map((_, j) => measure(chars.slice(0, j + 1).join(''), { size: SIZE }).w),
    count: text(n, { x: 0, y, size: SIZE, w: INK * 1.6, app: 'mail', align: 'right' }),
    countW: measure(n, { size: SIZE }).w,
  };
});

export function mailFrame() {
  return kit.window(MAIL.x, MAIL.y, MAIL.w, MAIL.h, { chrome: 'mac', bar: MAIL.bar, app: 'mail', w: LINE });
}

export function sidebar(div) {
  const out = [];
  const x0 = MAIL.x + 20;
  for (const r of rows) {
    const room = div - MAIL.x - 20 - 18 - r.countW - 12;
    const fits = r.widths.at(-1) <= room;
    let n = r.widths.length;
    if (!fits) while (n > 1 && r.widths[n - 1] + DOTS_W > room) n--;
    out.push(...r.name.map((s) => (s.g < n ? s : { ...s, show: [0, 0] })));
    const dx = x0 + (r.widths[n - 1] ?? 0);
    out.push(...DOTS.map((s) => ({ ...s, pts: s.pts.map(([x, y]) => [x + dx, y + r.y]), show: fits ? [0, 0] : [0, 1] })));
    out.push(...kit.move(r.count, { x: div - 18 }));
  }
  return out;
}

export function list(div) {
  const out = [];
  for (let i = 0; i < 4; i++) {
    const y = 672 + i * 72;
    out.push(line(div + 26, y, div + 136, y, { app: 'mail', w: LINE * 0.9 }));
    out.push(line(div + 26, y + 22, div + 26 + [180, 140, 168, 116][i], y + 22, { app: 'mail', w: INK }));
  }
  return out;
}

export const divider = (div) => line(div, MAIL.y + MAIL.bar, div, MAIL.y + MAIL.h, { app: 'mail', w: LINE });
