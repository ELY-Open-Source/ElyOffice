// c10 drawings: the document, the address bar, the engine channel.
import { kit } from '../lib/kit/index.js';

export const WIN = { x: 150, y: 110, w: 720, h: 500, bar: 56 };
export const ROW = [236, 336, 436];
export const STATUS = 550;
export const PILL = { x: 930, y: 190, w: 840, h: 68 };
export const LINE = 860;
export const ICON = [270, 860];
export const GEAR = { x: 1660, y: 860, r: 80 };
export const LOCK = { x: 965, top: LINE + 52 };
export const LENS = { x: 1250, r: 46 };
export const MEET = 965;
export const VER = 1024;

const LEFT = WIN.x + 36;
const BODY = { size: 30, w: 1 };
const CODE = 1.5;

export const URL_IN = 'https://example.com/Q3 plan "final".pdf';
export const URL_OUT = 'https://example.com/Q3%20plan%20%22final%22.pdf';
export const BROKEN = 'https://exa mple.com/';
export const MAILTO = 'mailto:ann@example.com?attach=notes.txt';
export const SAYS = "This link's address is broken, so Ely can't open it.";

const at = (str, n) => kit.measure(str.slice(0, n), BODY).w;
export const xAt = (str, n) => LEFT + at(str, n);

export function windowArt() {
  const { x, y, w, h, bar } = WIN;
  const mid = y + bar / 2;
  return [
    kit.rect(x, y, w, h, { r: 14 }),
    ...[0, 1, 2].map((i) => kit.circle(x + 30 + i * 26, mid, 7.5, { w: 0.8 })),
    ...kit.text('Q3 review.docx', { size: 28, x: x + w / 2, y: mid + 10, align: 'center', w: 0.85 }),
    kit.line(x + w, y + bar, x, y + bar, { w: 0.8 }),
  ];
}

export const rule = () => [kit.line(WIN.x + WIN.w, STATUS, WIN.x, STATUS, { w: 0.8 })];
export const link1 = () => kit.text(URL_IN, { ...BODY, x: LEFT, y: ROW[0] });
export const link2 = () => kit.text(BROKEN, { ...BODY, x: LEFT, y: ROW[1] });
export const link3 = () => kit.text(MAILTO, { ...BODY, x: LEFT, y: ROW[2], app: 'mail' });
export const says = () => kit.text(SAYS, { size: 28, x: LEFT - 10, y: STATUS + 39, w: 0.9 });

export const pill = () => [kit.rect(PILL.x, PILL.y, PILL.w, PILL.h, { r: PILL.h / 2, w: 0.9 })];

const codes = new Set();
for (let i = URL_OUT.indexOf('%'); i >= 0; i = URL_OUT.indexOf('%', i + 1)) [i, i + 1, i + 2].forEach((j) => codes.add(j));
export const isCode = (s) => s.ch !== undefined && codes.has(s.g);

export function url2() {
  const w = kit.measure(URL_OUT, BODY).w;
  const x = PILL.x + (PILL.w - w) / 2;
  return kit.text(URL_OUT, { ...BODY, x, y: ROW[0] }).map((s) => (isCode(s) ? { ...s, w: CODE } : s));
}

export function strike() {
  const x0 = xAt(MAILTO, MAILTO.indexOf('attach')) - 4;
  const x1 = xAt(MAILTO, MAILTO.length) + 6;
  const y = ROW[2] - 10;
  return [kit.stroke([[x0, y + 2], [(x0 + x1) / 2, y - 1], [x1, y - 3]], { w: 1.4, app: 'mail' })];
}

export const icon = () => kit.icon('doc', ICON[0], ICON[1], 150);

export const channel = () => [kit.line(ICON[0] + 70, LINE, GEAR.x - GEAR.r - 4, LINE, { w: 0.9 })];

export function gear(turn = 0) {
  const { x, y, r } = GEAR;
  const teeth = 10;
  const step = (Math.PI * 2) / teeth;
  const pts = [];
  for (let i = 0; i < teeth; i++) {
    const a = -Math.PI / 2 + i * step + turn;
    for (const [da, rr] of [[0.06, 0.8], [0.16, 1], [0.34, 1], [0.44, 0.8], [1, 0.8]]) {
      const b = a + step * (da - 0.25);
      pts.push([x + r * rr * Math.cos(b), y + r * rr * Math.sin(b)]);
    }
  }
  return [kit.poly(pts, { closed: true }), kit.circle(x, y, r * 0.3, { from: -Math.PI / 2 + turn })];
}

export function lock(lift, swing) {
  const { x, top } = LOCK;
  const cy = top - 32 - lift;
  const arc = kit.arc(x, cy, 20, Math.PI, Math.PI * 2).pts;
  const parts = [
    kit.rect(x - 32, top, 64, 56, { r: 10, w: 1.1 }),
    kit.circle(x, top + 22, 6, { w: 0.8 }),
    kit.line(x, top + 28, x, top + 40, { w: 0.8 }),
    kit.stroke([[x - 20, top - lift], ...arc, [x + 20, top]], { w: 1.2 }),
  ];
  return kit.move(parts, { r: swing, ox: x, oy: LINE });
}

export const version = (cx) => kit.text('0.1.17', { size: 30, x: cx, y: VER, align: 'center', w: 1 });

export const snippet = () => kit.text('<img src="data:…">', { size: 28, font: 'mono', x: ICON[0] + 110, y: LINE - 18, w: 0.95 });

export function lens(pulse = 0) {
  const { x, r } = LENS;
  const rr = r * (1 + pulse);
  const d = rr * Math.SQRT1_2;
  return [
    kit.circle(x, LINE, rr, { w: 1.1 }),
    kit.line(x + d, LINE + d, x + d + 40, LINE + d + 40, { w: 1.5 }),
  ];
}

export function picture() {
  const x = 1340;
  const y = 772;
  return [
    kit.rect(x, y, 112, 76, { r: 6 }),
    kit.poly([[x + 8, y + 68], [x + 38, y + 34], [x + 56, y + 54], [x + 72, y + 40], [x + 104, y + 68]]),
    kit.circle(x + 84, y + 20, 9, { w: 0.9 }),
  ];
}

export const tick = () => [kit.stroke([[LENS.x - 22, 772], [LENS.x - 7, 788], [LENS.x + 22, 754]], { w: 1.4 })];

export const click = (x, y, u) => [kit.ripple(x, y, u, { r: 12 })];
