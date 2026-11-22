// ElyShow fixes: select, keys, undo, links, script; one window.
import { kit } from '../lib/kit/index.js';
import { FPS } from '../lib/time.js';
import * as ui from './c08-ui.js';
import { T, DRAW, KEYS, KEY_Y, LEAVE, slides, pop, press, pointer, camera } from './c08-plan.js';

const START = 68;
const END = 78;
const off = (s) => ({ ...s, show: [0, 0] });
const hidden = (list) => list.map(off);
const shift = (list, dx) => (dx ? kit.move(list, { x: dx }) : list);
const evenTime = (t) => (2 * Math.floor(Math.round(t * FPS) / 2)) / FPS;
const when = (on, list) => (on ? list : hidden(list));
// Faint entry into a group, so off-screen joins stay quiet.
const quiet = (list, link = 0.2) => list.map((s, i) => (i ? s : { ...s, link }));

const chrome = ui.chrome();
const menu = ui.menu();
const drop = ui.dropdown(menu.insert);
const canvas = ui.canvas(drop.right + 6);
const cut = 1 - (drop.right + 6 - Math.max(ui.SLIDE.x, drop.left - 4)) / kit.length(canvas);
const R = ui.S1_RECT;
const dragFrom = [R.x, R.y];
const dragTo = [R.x + R.w, R.y + R.h];
const rectAt = (x1, y1) => kit.rect(R.x, R.y, Math.max(0, x1 - R.x), Math.max(0, y1 - R.y), { r: 4, w: 1.25 });
const full = rectAt(...dragTo);
const title = kit.text('Launch.pptx', { x: 960, y: ui.WIN.y + 38, size: 28, align: 'center', w: 0.9 });
title[0].link = 1;
const diagram = ui.diagram();
const tilted = ui.tilted();
const thumbs = [
  ui.thumb(0), ...ui.mini([full], 0),
  ui.thumb(1), ...ui.mini(diagram.strokes, 1),
  ui.thumb(2), ...ui.mini(tilted.strokes, 2),
];
const box = ui.titleBox();
const words = kit.text('Launch day', { ...box.text, w: 1.05 });
const braces = ui.braces();
const select = ui.handles(R.x, R.y, R.x + R.w, R.y + R.h, 1).at(-1).pts.at(-1);
const linkB = [tilted.hit[0] - 30, tilted.hit[1] + 12];
// Leave straight down, clear of the slide.
const exit = () => [braces.at(-1).pts.at(-1)[0], 1020];

let cursorAt = null;
const spot = { shape: [R.x + R.w * 0.62, R.y + R.h * 0.7] };
const spots = () => ({
  canvasEnd: canvas.pts.at(-1), insert: [menu.insert.x + menu.insert.w * 0.55, ui.WIN.y + ui.WIN.bar + 26],
  glyph: drop.target, dragFrom, dragTo, select, thumbsEnd: thumbs.at(-1).pts.at(-1), textClick: box.click,
  typedEnd: words.at(-1).pts.at(-1), linkA: diagram.hit, linkB, shape: spot.shape,
  bracesEnd: braces.at(-1).pts.at(-1), exit: exit(),
});
const cursorFor = () => (cursorAt ??= pointer(spots()));

const ripple = (te, T0, [x, y]) => ({ ...kit.ripple(x, y, (te - T0) / 0.3125, { r: 12, w: 0.7 }), link: 0.4 });

function dropdown(t, te) {
  const open = kit.ease.out(kit.span(te, T.insert, T.insert + 0.0625));
  const unfurl = (list) => list.map((s) => ({ ...s, pts: s.pts.map(([x, y]) => [x, drop.top + (y - drop.top) * open]) }));
  const list = unfurl([...drop.strokes, te >= T.lit ? drop.lit : off(drop.lit)]);
  if (te < T.insert) return hidden(list);
  return kit.leave(list, kit.span(t, T.glyph, T.glyph + LEAVE));
}

function slideOne(t, te, k) {
  if (te < T.drag[0]) return off(full);
  if (t < T.drag[1]) {
    const c = cursorFor()(t) ?? { x: dragTo[0], y: dragTo[1] };
    return rectAt(c.x, c.y);
  }
  return full;
}

const grip = (te) => kit.keys(te - T.select, [[0, 0.4], [1 / 30, 1.35, 'out'], [0.1, 1, 'inOut']]);

// While handles pop, the pointer rides the clicked one.
function pointerAt(t, k) {
  const te = evenTime(t);
  if (te < T.select || t >= DRAW[1][0]) return cursorFor()(t);
  const [x, y] = ui.handles(R.x, R.y, R.x + R.w, R.y + R.h, grip(te)).at(-1).pts.at(-1);
  return { x, y };
}

function selection(t, te) {
  const list = ui.handles(R.x, R.y, R.x + R.w, R.y + R.h, grip(te));
  if (te < T.select) return hidden(list);
  return kit.leave(list, kit.span(t, 70.625, 70.6875));
}

function script(te) {
  const s = kit.keys(te - T.script, [[0, 0.3], [1 / 15, 1.18, 'out'], [0.2, 1, 'inOut']]);
  return kit.move(braces, { s, ox: R.x + R.w / 2, oy: R.y + R.h / 2 });
}

function typing(t, te, at) {
  const edit = at !== undefined && te < T.retype;
  const frame = edit && te >= T.text ? box.frame : off(box.frame);
  const ring = edit ? ripple(te, T.text, box.click) : off(ripple(te, T.text, box.click));
  if (!edit) return [frame, ring, ...hidden(words)];
  let list = words.map((s) => (te >= T.type + s.g * 0.0625 ? s : off(s)));
  if (te >= T.undo && te < T.redo) {
    const gone = kit.leave(list.filter((s) => s.g >= 7), kit.span(t, T.undo, T.undo + LEAVE));
    list = [...list.filter((s) => s.g < 7), ...gone];
  }
  return [shift([frame], at)[0], ring, ...shift(list, at)];
}

function keys(t, te) {
  return KEYS.flatMap((key) => {
    const live = key.shows.find(([a, b]) => te >= a && t < b + LEAVE);
    const x = key.x(t);
    const cap = ui.keycap(key.label, x, KEY_Y, key.w, press(key, t));
    cap[0] = { ...cap[0], link: 0.5 };
    if (!live) return hidden(cap);
    const grown = kit.move(cap, { s: pop(te, live[0]), ox: x + key.w / 2, oy: KEY_Y + ui.KEY_H / 2 });
    return kit.leave(grown, kit.span(t, live[1], live[1] + LEAVE));
  });
}

function raw(t, k) {
  const te = evenTime(t);
  const at = slides(t, te);
  const cur = pointerAt(t, k);
  const covered = te >= T.insert && t < T.glyph + LEAVE;
  const hand = ui.hand(cur?.x ?? 0, cur?.y ?? 0, 1.05);
  const one = at[1] !== undefined;
  const two = at[2] !== undefined;
  return [
    ...chrome, ...menu.strokes, covered ? { ...canvas, show: [0, cut] } : canvas,
    ...dropdown(t, te),
    ripple(te, T.glyph, drop.target),
    one && te < T.swap ? slideOne(t, te, k) : off(full),
    ...selection(t, te),
    ...when(te >= DRAW[1][0], [...title, ...thumbs]),
    ...quiet(one && te >= T.swap ? shift([full], at[1]) : [off(full)]),
    ...(one && te >= T.script ? shift(script(te), at[1]) : hidden(braces)),
    ...quiet(two && te >= T.retype ? shift(words, at[2]) : hidden(words)),
    ...quiet(two ? shift(diagram.strokes, at[2]) : hidden(diagram.strokes)),
    ...quiet(at[3] === undefined ? hidden(tilted.strokes) : shift(tilted.strokes, at[3])),
    ...typing(t, te, at[2]),
    ...keys(t, te),
    ripple(te, T.links[0], diagram.hit),
    ripple(te, T.links[1], linkB),
    ripple(te, T.script, spot.shape),
    ...when(te >= 74.8125 && t < 75.5625 && cur, hand),
  ];
}

// Drawn length per frame: eased in pen stretches, else whole.
let table = null;
const held = new Set();
function pens(k) {
  const f0 = Math.round(START * FPS);
  const f1 = Math.round(END * FPS);
  const win = DRAW.map(([a, b]) => [Math.round(a * FPS), Math.round(b * FPS)]);
  const v = new Float64Array(f1 - f0 + 1);
  let top = 0;
  let even = 0;
  let from = 0;
  for (let f = f0; f <= f1; f++) {
    const t = f / FPS;
    let len = t <= DRAW[0][0] ? 0 : kit.marks(raw(t, k)).total;
    if (f % 2 && f !== f1 && len > even + 0.5) {
      held.add(f);
      len = kit.marks(raw((f - 1) / FPS, k)).total;
    }
    const w = win.find(([a, b]) => f >= a && f <= b);
    if (w && f === w[0]) from = top;
    const goal = w ? kit.lerp(from, len, kit.ease.inOut((f - w[0]) / (w[1] - w[0]))) : len;
    top = Math.max(top, goal);
    v[f - f0] = top;
    if (f % 2 === 0) even = Math.max(even, len);
  }
  return v;
}

// Frames outrunning the pen repeat the frame before.
function strokes(t, k) {
  table ??= pens(k);
  const f = Math.round(t * FPS);
  return raw(held.has(f) ? (f - 1) / FPS : t, k);
}

export default {
  id: 'c08', start: START, end: END, app: 'show',
  strokes,
  pen(t, k) {
    table ??= pens(k);
    return table[Math.min(table.length - 1, Math.max(0, Math.round(t * FPS) - Math.round(START * FPS)))];
  },
  cursor: pointerAt,
  camera: (t) => camera(t),
};
