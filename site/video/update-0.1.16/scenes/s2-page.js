// What the apps show: ElyDoc's page, ElyTable's sheet.
import { span, lerp, ease, clamp } from '../lib/time.js';
import { rnd, signed } from '../lib/rand.js';
import { INK } from '../lib/stroke.js';
import { TAB, HALF, GREEN, GOLD, poly } from './s2-desk.js';

const LEFT = 300;
const RIGHT = 830;
const LINES = Array.from({ length: 9 }, (_, i) => 262 + 34 * i);
const WORDS = Array.from({ length: 46 }, (_, i) => 26 + Math.round(66 * rnd(0x52, i) ** 1.4));
const BREAK = 21;

function flow(hole) {
  const out = [];
  let row = 0;
  let x = LEFT + 28;
  WORDS.forEach((len, i) => {
    if (i === BREAK) {
      row++;
      x = LEFT + 28;
    }
    for (;;) {
      const y = LINES[row];
      if (y === undefined) throw new Error('s2: the page text overflows its window');
      if (hole && y > hole.y0 && y < hole.y1 && x < hole.x1 && x + len > hole.x0) x = hole.x1;
      if (x + len <= RIGHT) break;
      row++;
      x = LEFT;
    }
    out.push([x, LINES[row]]);
    x += len + 13;
  });
  return out;
}

let laid = null;
function layout(spot) {
  if (laid) return laid;
  const [hw, hh] = HALF;
  const hole = { x0: spot[0] - hw - 12, x1: spot[0] + hw + 14, y0: spot[1] - hh - 6, y1: spot[1] + hh + 16 };
  laid = { plain: flow(null), wrapped: flow(hole) };
  return laid;
}

function word(ink, x, y, len, i, upto) {
  const pts = [];
  const n = Math.max(3, Math.round(len / 7));
  for (let k = 0; k <= n; k++) {
    pts.push([x + (len * k) / n, y + 2.2 * Math.sin(k * 1.9 + i * 2.3) + 1.2 * signed(0x77, i, k)]);
  }
  ink.curve(pts, { id: `w${i}`, width: 3.4, alpha: 0.78, upto, overshoot: 2, taper: [0.12, 0.25], boil: 0.7, segs: 4 });
}

export function page(ink, t, o) {
  if (t < o.open) return;
  ink.text('Field notes', { x: LEFT, y: 214, size: 40, upto: span(t, o.open + 0.04, o.open + 0.3), alpha: 0.9 });
  const { plain, wrapped } = layout(o.spot);
  WORDS.forEach((len, i) => {
    const t0 = o.open + 0.16 + i * 0.011;
    const upto = span(t, t0, t0 + 0.09);
    if (upto <= 0) return;
    let [x, y] = plain[i];
    const cx = x + len / 2;
    if (t >= o.land) {
      const lag = Math.min(0.14, Math.hypot(cx - o.spot[0], y - o.spot[1]) / 2600);
      const k = ease.back(span(t, o.land + lag, o.land + lag + 0.32));
      x = lerp(x, wrapped[i][0], k);
      y = lerp(y, wrapped[i][1], k);
    }
    if (t >= o.hit) {
      const dx = cx - o.bump[0];
      const dy = y - o.bump[1];
      const d = Math.hypot(dx, dy) || 1;
      const push = 16 * Math.exp(-(t - o.hit) * 7) * Math.max(0, 1 - d / 300) * Math.cos((t - o.hit) * 24);
      x += (dx / d) * push;
      y += (dy / d) * push;
    }
    word(ink, x, y, len, i, upto);
  });
}

const COLS = [1124, 1290, 1430, 1560];
const ROWS = [262, 308, 354, 400, 446, 492, 538, 584];
const CELLS = [['Month', 'Spend'], ['Jan', '120'], ['Feb', '340'], ['Mar', '280'], ['Apr', '410'], ['May', '365'],
  ['Jun', '450'], ['Total', '1965']];

function pop(ink, t, t0, [cx, cy], paint) {
  const u = span(t, t0, t0 + 0.12);
  if (u <= 0) return;
  const s = 1 + 0.35 * (1 - ease.out(u));
  const ctx = ink.ctx;
  ctx.save();
  ctx.translate(cx, cy);
  ctx.scale(s, s);
  ctx.translate(-cx, -cy);
  paint();
  ctx.restore();
}

export function sheetView(ink, t, open) {
  if (t < open) return;
  const { x0, x1, y1, bar } = TAB;
  const head = span(t, open + 0.05, open + 0.3);
  if (head > 0) {
    ink.wash([[[x0 + 3, bar + 2], [x1 - 3, bar + 2], [x1 - 3, ROWS[0]], [x0 + 3, ROWS[0]]]], { id: 'thead', color: GREEN, alpha: 0.28 * head, bleed: 4 });
  }
  const gold = span(t, open + 0.62, open + 0.8);
  if (gold > 0) {
    ink.wash([[[COLS[1], ROWS[7]], [COLS[2], ROWS[7]], [COLS[2], ROWS[7] + 46], [COLS[1], ROWS[7] + 46]]], { id: 'tgold', color: GOLD, alpha: 0.55 * gold, bleed: 3 });
  }
  const grid = (k) => ({ id: `g${k}`, width: 1.5, alpha: 0.42, style: 'charcoal', overshoot: 5, boil: 1, upto: span(t, open + k * 0.018, open + k * 0.018 + 0.14) });
  COLS.forEach((x, k) => ink.line([[x, bar], [x, y1]], grid(k)));
  ROWS.forEach((y, k) => ink.line([[x0, y], [x1, y]], grid(k + 4)));
  const mid = (a, b) => (a + b) / 2;
  const centres = [mid(COLS[0], COLS[1]), mid(COLS[1], COLS[2]), mid(COLS[2], COLS[3]), mid(COLS[3], x1)];
  ['A', 'B', 'C', 'D'].forEach((ch, k) => pop(ink, t, open + 0.1 + k * 0.03, [centres[k], 242], () =>
    ink.text(ch, { x: centres[k], y: 250, size: 22, align: 'center', alpha: 0.7 })));
  CELLS.forEach(([a, b], r) => {
    const y = ROWS[r] + 31;
    const t0 = open + 0.18 + r * 0.055;
    pop(ink, t, t0, [mid(x0, COLS[0]), y - 8], () => ink.text(String(r + 1), { x: mid(x0, COLS[0]), y, size: 20, align: 'center', alpha: 0.6 }));
    pop(ink, t, t0 + 0.02, [COLS[0] + 40, y - 8], () => ink.text(a, { x: COLS[0] + 12, y, size: 24, alpha: 0.9 }));
    pop(ink, t, t0 + 0.04, [COLS[2] - 40, y - 8], () => ink.text(b, { x: COLS[2] - 12, y, size: 24, align: 'right', alpha: 0.9 }));
  });
}

const DROPS = Array.from({ length: 44 }, (_, i) => {
  const a = (i / 44) * Math.PI * 2 + 0.35 * signed(0x5a, i);
  const tint = i % 5 === 1 ? [92, 140, 196] : i % 5 === 3 ? [58, 116, 60] : INK;
  return { a, reach: 14 + 170 * rnd(0x5b, i) ** 1.5, r: 2.6 + 9 * rnd(0x5c, i) ** 2.2, tint, sat: rnd(0x5d, i) > 0.45 };
});

function blot(ink, x, y, r, color, alpha, id) {
  const s = ink.seed();
  const pts = [];
  for (let j = 0; j < 10; j++) {
    const a = (j / 10) * Math.PI * 2;
    const k = 1 + 0.28 * signed(id, j) + 0.06 * signed(s, j);
    pts.push([x + Math.cos(a) * r * k, y + Math.sin(a) * r * k * 0.88]);
  }
  poly(ink.ctx, pts, color, alpha);
}

export function splash(ink, t, land, spot) {
  if (t < land) return;
  const u = ease.out(span(t, land, land + 0.25));
  const [hw, hh] = HALF;
  ink.smudge(spot[0], spot[1] + hh * 0.6, hw * 1.6, { id: 'dust', color: INK, alpha: 0.12 * u });
  DROPS.forEach((d, i) => {
    const c = Math.cos(d.a);
    const s = Math.sin(d.a);
    const k = Math.min(hw / Math.max(Math.abs(c), 1e-3), hh / Math.max(Math.abs(s), 1e-3));
    const ox = spot[0] + c * (k - 6);
    const oy = spot[1] + s * (k - 6);
    const reach = s < 0 ? Math.min(d.reach, (oy - 236) / -s) : d.reach;
    const x = ox + c * reach * u;
    const y = oy + s * reach * u;
    const alpha = d.tint === INK ? 0.9 : 0.8;
    const tail = clamp(0.25 + 0.75 * (1 - u));
    ink.line([[ox + c * reach * u * (1 - tail), oy + s * reach * u * (1 - tail)], [x, y]], {
      id: `drop${i}`, width: d.r * 1.5, color: d.tint, alpha, taper: [0.95, 0.02], overshoot: 0, boil: 0.5 });
    blot(ink, x, y, d.r * (0.7 + 0.3 * u), d.tint, alpha, 0x5e + i);
    if (d.sat) blot(ink, x + c * (d.r * 3 + 8) * u, y + s * (d.r * 3 + 8) * u, d.r * 0.35, d.tint, alpha, 0x9e + i);
  });
}
