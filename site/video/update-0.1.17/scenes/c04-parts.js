// c04 geometry: engine, start scene, page, settings, tabs.
import { kit as k } from '../lib/kit/index.js';
import { resample, length } from '../lib/geom.js';

export const WIN = { x: 400, y: 130, w: 900, h: 820 };
export const MID = { x: WIN.x + WIN.w / 2, y: 505 };
export const PAGE = { x: MID.x - 226, y: 272, w: 452, h: 640 };
export const TRACK = { x: MID.x - 138, y: 722, w: 276 };
export const ENGINE = { x: 220, y: 800, r: 58 };
export const SET = { x: 1370, y: 236, w: 480, h: 290 };
export const SEG = { x: SET.x + 36, y: 410, h: 60 };
export const TABS = { y: 140, base: WIN.y + 56, x: WIN.x + 22 };
export const CLOSE = [WIN.x + WIN.w - 36, WIN.y + 28];

const ICON = 220;
const BAR = 0.3;
const flip = (s, on) => (on ? { ...s, pts: [...s.pts].reverse() } : s);

export function icon(scale) {
  return k.icon('doc', MID.x, MID.y, ICON * scale, { w: 1.15 }).map((s, i) => flip(s, i === 3 || i === 5));
}

const KEYS = [[-64, -92], [18, -92], [73, -37], [73, 83], [64, 92], [-64, 92], [-73, 83], [-73, -82], [-64, -92]];
const CUTS = (() => {
  const pts = icon(1)[0].pts;
  const near = ([x, y], from) => {
    let best = from;
    for (let i = from; i < pts.length; i++) {
      const d = Math.hypot(pts[i][0] - MID.x - x, pts[i][1] - MID.y - y);
      if (d < Math.hypot(pts[best][0] - MID.x - x, pts[best][1] - MID.y - y) - 1e-9) best = i;
    }
    return best;
  };
  const out = [0];
  for (const key of KEYS.slice(1, -1)) out.push(near(key, out.at(-1) + 1));
  return [...out, pts.length - 1];
})();

function pageSegs() {
  const { x, y, w, h } = PAGE;
  const r = 8;
  const q = Math.PI / 2;
  const arc = (cx, cy, a0) => k.arc(cx, cy, r, a0, a0 + q).pts;
  return [[[x + r, y], [x + w - 3, y]], [[x + w - 3, y], [x + w, y + 3]], [[x + w, y + 3], [x + w, y + h - r]],
    arc(x + w - r, y + h - r, 0), [[x + w - r, y + h], [x + r, y + h]], arc(x + r, y + h - r, q),
    [[x, y + h - r], [x, y + r]], arc(x + r, y + r, 2 * q)];
}

function outline(from, u) {
  const segs = pageSegs();
  const pts = [];
  for (let i = 0; i < segs.length; i++) {
    const a = from.pts.slice(CUTS[i], CUTS[i + 1] + 1);
    const n = Math.max(2, Math.ceil(Math.max(length(a), length(segs[i])) / 4));
    const pa = resample(a, n);
    const pb = resample(segs[i], n);
    pts.push(...pa.map((p, j) => [k.lerp(p[0], pb[j][0], u), k.lerp(p[1], pb[j][1], u)]).slice(i ? 1 : 0));
  }
  return { ...from, pts };
}

const LINES = [[0, 240, 1.5], [1, 340], [2, 340], [3, 340], [4, 200], [6, 340], [7, 340], [8, 300], [9, 340], [10, 180]];

function lines() {
  return LINES.map(([row, span, w = 0.95], i) => {
    const y = PAGE.y + 86 + row * 40;
    const a = [PAGE.x + 56, y];
    const b = [PAGE.x + 56 + span, y];
    return k.stroke(i % 2 ? [b, a] : [a, b], { w });
  });
}

export function pageFrom(scale, u) {
  const a = icon(scale);
  const b = lines();
  const seed = a[a.length - 1].pts.at(-1);
  const tick = k.stroke([[PAGE.x + PAGE.w - 2, PAGE.y], [PAGE.x + PAGE.w, PAGE.y + 2]]);
  const out = [{ ...outline(a[0], u), w: 1.15 }, { ...k.morph(a[1], tick, u), show: u < 1 ? [0, 1] : [0, 0] }];
  b.forEach((target, i) => {
    const from = a[i + 2] ?? k.stroke([seed, [seed[0] + (i % 2 ? -1 : 1), seed[1]]]);
    out.push({ ...k.morph(from, target, u), w: k.lerp(from.w, target.w, u) });
  });
  if (u <= 0) return out.map((s, i) => (i >= 7 ? { ...s, show: [0, 0] } : s));
  return out;
}

export function name() {
  return k.text('ElyDoc', { x: MID.x, y: 676, size: 44, align: 'center' });
}

export function track(share) {
  const x0 = TRACK.x + TRACK.w * share;
  return [k.line(TRACK.x + TRACK.w, TRACK.y, TRACK.x, TRACK.y, { w: 0.4 }),
    { ...k.line(x0, TRACK.y, x0 + TRACK.w * BAR, TRACK.y, { w: 1.8 }), link: 0.2 }];
}

function gearPts(cx, cy, r, turn) {
  const n = 8 * 48;
  const pts = [];
  for (let i = 0; i <= n; i++) {
    const a = -0.75 * Math.PI + (i / n) * Math.PI * 2;
    const ph = ((((a - turn) / (Math.PI * 2)) * 192) % 24 + 24) % 24;
    const lift = ph < 4 ? ph / 4 : ph < 10 ? 1 : ph < 14 ? (14 - ph) / 4 : 0;
    pts.push([cx + r * (0.8 + 0.2 * lift) * Math.cos(a), cy + r * (0.8 + 0.2 * lift) * Math.sin(a)]);
  }
  return pts;
}

export function engine(turn) {
  const { x, y, r } = ENGINE;
  return [k.stroke(gearPts(x, y, r, turn), { w: 1.15 }), { ...k.circle(x, y, r * 0.3, { from: -0.75 * Math.PI }), link: 0.3 }];
}

export function ring(fill) {
  const { x, y, r } = ENGINE;
  const work = k.circle(x, y, r * 1.5, { w: 0.6, from: -Math.PI / 2, to: Math.PI * 1.5 });
  return { ...work, show: fill > 0 ? [0, fill] : [0, 0], link: 0.3 };
}

const LABEL = 32;
const SEGS = ['System', 'Reduced', 'Full'].map((label) => ({ label, w: k.measure(label, { size: LABEL }).w + 40 }));
SEGS.reduce((x, s) => {
  s.x = x;
  return x + s.w;
}, SEG.x);
const SEG_W = SEGS.reduce((a, s) => a + s.w, 0);

export const segment = (i) => SEGS[i];

export function settings(pick, click) {
  const { x, y, w, h } = SET;
  const bar = 56;
  const mid = y + bar / 2;
  const g = 5;
  const cx = x + w - 36;
  const out = [{ ...k.rect(x, y, w, h, { r: 12, w: 1.15 }), link: 0.3 }, ...k.text('Settings', { x: x + w / 2, y: mid + 10, size: 28, align: 'center' }),
    k.circle(cx, mid, 13), k.line(cx - g, mid - g, cx + g, mid + g), k.line(cx + g, mid - g, cx - g, mid + g),
    k.line(x + w, y + bar, x, y + bar, { w: 1.15 })];
  out.push(...k.text('Motion', { x: SEG.x, y: SEG.y - 30, size: LABEL }));
  for (const s of SEGS) out.push(...k.text(s.label, { x: s.x + s.w / 2, y: SEG.y + 41, size: LABEL, align: 'center' }));
  out.push({ ...k.rect(SEG.x, SEG.y, SEG_W, SEG.h, { r: 14 }), link: 0.15 });
  const a = SEGS[Math.floor(pick)];
  const b = SEGS[Math.min(SEGS.length - 1, Math.floor(pick) + 1)];
  const u = pick - Math.floor(pick);
  out.push({ ...k.rect(k.lerp(a.x, b.x, u) + 6, SEG.y + 6, k.lerp(a.w, b.w, u) - 12, SEG.h - 12, { r: 10, w: 1.35 }), link: 0.3 });
  out.push(k.ripple(SEGS[1].x + SEGS[1].w / 2, SEG.y + SEG.h / 2, click));
  return out;
}

const NAMES = ['Report.docx', 'Notes.docx', 'Plan.docx'];
const TAB_W = NAMES.map((n) => k.measure(n, { size: 28 }).w + 66);
export const SHUT = -(8 + TAB_W[1]);

function tab(x, i, lead) {
  const { y, base } = TABS;
  const w = TAB_W[i];
  const r = 10;
  const q = Math.PI / 2;
  const edge = [...lead, [x, base], ...k.arc(x + r, y + r, r, Math.PI, Math.PI + q).pts, ...k.arc(x + w - r, y + r, r, -q, 0).pts, [x + w, base]];
  const cx = x + w - 24;
  const cy = (y + base) / 2 + 1;
  const g = 7;
  return [
    k.stroke(edge, { w: i ? 0.95 : 1.2 }),
    k.line(cx - g, cy - g, cx + g, cy + g, { w: 0.95 }),
    k.line(cx + g, cy - g, cx - g, cy + g, { w: 0.95 }),
    ...k.text(NAMES[i], { x: x + 18, y: cy + 10, size: 28 }),
  ];
}

export function tabs(slide, gone) {
  const x1 = TABS.x + TAB_W[0] + 8;
  const x2 = x1 + TAB_W[1] + 8 + slide;
  const px = x2 + TAB_W[2] + 34;
  const py = (TABS.y + TABS.base) / 2 + 1;
  const notes = tab(x1, 1, []);
  return [
    ...tab(TABS.x, 0, [[WIN.x, TABS.base]]),
    ...(gone ? notes.map((s) => ({ ...s, show: [0, 0] })) : notes),
    ...tab(x2, 2, []),
    k.line(px - 9, py, px + 9, py, { w: 0.95 }),
    k.line(px, py - 9, px, py + 9, { w: 0.95 }),
    k.line(x2 + TAB_W[2], TABS.base, WIN.x + WIN.w, TABS.base),
  ];
}

export const notesClose = () => [TABS.x + TAB_W[0] + 8 + TAB_W[1] - 24, (TABS.y + TABS.base) / 2 + 1];

export function frame() {
  const all = k.window(WIN.x, WIN.y, WIN.w, WIN.h, { chrome: 'linux', w: 1.25 });
  return { rect: all[0], bar: all[1], controls: all.slice(2) };
}
