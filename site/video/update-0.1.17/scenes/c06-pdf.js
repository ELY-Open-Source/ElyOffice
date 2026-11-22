// ElyPDF: the scroll bar drags straight through the document.
import { kit as k } from '../lib/kit/index.js';
import { densify } from '../lib/geom.js';
import { band, ticker, capsule, nearest } from './c06-util.js';

const X = 1240;
const Y = 580;
const WD = 600;
const HT = 430;
const LO = Y + 50;
const HI = Y + HT - 4;
const P = HI - LO;
const PH = P - 28;
const PW = Math.round(PH / 1.414);
const PX = 1526 - PW / 2;
const PAGES = 48;
const LINES = 10;
const TRACK = [LO + 12, HI - 12];
const THUMB = 40;
const T = { hover: 49.5, grab: 50.0, end: 51.0, let: 51.25 };
const A = 0.3;
const C = 1 / (1 - A / 2);
const hidden = { ...k.line(PX, LO, PX + 1, LO), show: [0, 0] };

const chrome = k.window(X, Y, WD, HT, { chrome: 'mac', title: 'Report.pdf', titleSize: 20, bar: 48, w: 0.8 });
const title = k.text('Annual Report', { x: PX + 26, y: 0, size: 24, font: 'serif', w: 0.8 });
const count = ticker(['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '/'], 7, { size: 18, font: 'mono', w: 0.7, align: 'right' });

// Drag: eases in, then runs flat out to the stop.
function drag(t) {
  const u = k.span(t, T.grab, T.end);
  return u < A ? (C * u * u) / (2 * A) : C * (u - A / 2);
}

function speed(t) {
  const u = k.span(t, T.grab, T.end);
  if (u <= 0 || u >= 1) return 0;
  return (C * Math.min(u / A, 1) * (PAGES - 1) * P) / (T.end - T.grab);
}

function scroll(t) {
  const tau = t - T.end;
  const bounce = tau > 0 ? 26 * Math.exp(-10 * tau) * Math.sin(20 * tau) : 0;
  return drag(t) * (PAGES - 1) * P + bounce;
}

function lineLen(i, n) {
  if (n % 4 === 3) return 0;
  const r = k.hash(6, i, n);
  return (n % 4 === 2 ? 0.35 + 0.4 * r : 0.82 + 0.18 * r) * (PW - 52);
}

function slot(i, top, streak) {
  const off = streak || i < 0 || i >= PAGES;
  const loop = off ? null : band(k.rect(PX, top, PW, PH).pts, LO, HI);
  const out = [loop ? k.stroke(loop, { w: 0.7 }) : hidden];
  const first = i === 0 ? 3 : 0;
  for (let n = 0; n < LINES; n++) {
    const y = top + 44 + n * 30 + (i === 0 ? 40 : 0);
    const len = lineLen(i, n);
    const on = !off && n >= first && len > 0 && y > LO + 2 && y < HI - 2 && y < top + PH - 20;
    const x0 = PX + 26;
    out.push(on ? { ...(n % 2 ? k.line(x0 + len, y, x0, y, { w: 0.5 }) : k.line(x0, y, x0 + len, y, { w: 0.5 })), link: 0.35 } : hidden);
  }
  const cover = !off && i === 0;
  for (const s of title) {
    const pts = s.pts.map(([x, y]) => [x, y + top + 66]);
    const inside = pts.every(([, y]) => y > LO && y < HI);
    out.push(cover && inside ? { ...s, pts } : { ...hidden, g: -1 });
  }
  return out;
}

function streaks(on, v) {
  const xs = [PX, PX + 40, PX + 96, PX + 150, PX + PW - 60, PX + PW];
  return xs.map((x, i) => {
    const edge = i === 0 || i === xs.length - 1;
    if (!on) return hidden;
    const s = i % 2 ? k.line(x, HI - 6, x, LO + 6) : k.line(x, LO + 6, x, HI - 6);
    return { ...s, w: edge ? 0.6 : 0.3, alpha: edge ? 1 : 0.1 + 0.5 * k.clamp(v / 15000), link: 0.3 };
  });
}

const wide = (t) => k.keys(t, [[T.hover, 3.5], [T.hover + 0.1875, 6.5, 'back'], [T.let, 6.5], [T.let + 0.25, 3.5]]);
const top = (t) => TRACK[0] + (TRACK[1] - TRACK[0] - THUMB) * k.clamp(drag(t));
const grip = (t) => [1822 + wide(t), top(t) + THUMB / 2];

function thumb(t) {
  const y0 = top(t);
  const squash = Math.max(0, k.spring(t - T.end, 5, 30, 12));
  const pill = capsule(1822, y0 + squash, y0 + THUMB, wide(t), { w: t >= T.grab && t < T.let ? 1.3 : 0.9 });
  return nearest({ ...pill, pts: densify(pill.pts, 3) }, grip(t));
}

export const pdf = {
  t0: 48.5,
  t1: 49.375,
  strokes(t) {
    const s = scroll(t);
    const streak = speed(t) > 1500;
    const base = Math.floor(s / P);
    const pages = [0, 1].flatMap((j) => slot(base + j, LO + 14 + (base + j) * P - s, streak));
    const page = Math.min(PAGES, Math.max(1, Math.round(s / P) + 1));
    return [
      ...chrome, { ...k.line(1822, TRACK[0], 1822, TRACK[1]), w: 0.35, alpha: 0.5 },
      ...pages, ...streaks(streak, speed(t)), ...count(`${page} / ${PAGES}`, X + WD - 22, Y + 31), thumb(t),
    ];
  },
  // Pointer grabs the thumb and rides it down.
  cursor(t, tip, to) {
    if (t < this.t1) return null;
    const [x, y] = grip(t);
    if (t < T.grab) return k.keys(t, [[this.t1, tip], [T.hover, grip(T.hover), 'inOut'], [T.grab, grip(T.grab)]]);
    const over = Math.max(0, k.spring(t - T.end, 10, 16, 7));
    const back = k.ease.inOut(k.span(t, T.let, 51.5));
    return [k.lerp(x, to[0], back), k.lerp(y + over, to[1], back)];
  },
  clicks: [[T.grab, ...grip(T.grab)]],
  cam: { x: X + WD / 2, y: Y + HT / 2, w: 1000 },
};
