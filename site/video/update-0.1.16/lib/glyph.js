// Glyph outlines from LXGW WenKai, flattened to polylines.
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import opentype from 'opentype.js';

const FONT = fileURLToPath(new URL('../assets/fonts/LXGWWenKai-Regular.ttf', import.meta.url));
let font = null;
const memo = new Map();

function load() {
  if (font) return font;
  if (!fs.existsSync(FONT)) throw new Error(`missing font ${FONT}; see ENGINE.md`);
  const b = fs.readFileSync(FONT);
  font = opentype.parse(b.buffer.slice(b.byteOffset, b.byteOffset + b.length));
  return font;
}

function flatten(commands) {
  const out = [];
  let cur = null;
  let x = 0;
  let y = 0;
  const steps = 7;
  for (const c of commands) {
    if (c.type === 'M') {
      if (cur && cur.length > 2) out.push(cur);
      cur = [[c.x, c.y]];
    } else if (c.type === 'L') cur.push([c.x, c.y]);
    else if (c.type === 'Q') {
      for (let i = 1; i <= steps; i++) {
        const u = i / steps;
        const v = 1 - u;
        cur.push([v * v * x + 2 * v * u * c.x1 + u * u * c.x, v * v * y + 2 * v * u * c.y1 + u * u * c.y]);
      }
    } else if (c.type === 'C') {
      for (let i = 1; i <= steps; i++) {
        const u = i / steps;
        const v = 1 - u;
        cur.push([
          v * v * v * x + 3 * v * v * u * c.x1 + 3 * v * u * u * c.x2 + u * u * u * c.x,
          v * v * v * y + 3 * v * v * u * c.y1 + 3 * v * u * u * c.y2 + u * u * u * c.y,
        ]);
      }
    } else if (c.type === 'Z') {
      if (cur && cur.length > 2) out.push(cur);
      cur = null;
    }
    if (c.x !== undefined) {
      x = c.x;
      y = c.y;
    }
  }
  if (cur && cur.length > 2) out.push(cur);
  return out;
}

function bounds(contours) {
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const c of contours) for (const [x, y] of c) {
    x0 = Math.min(x0, x);
    y0 = Math.min(y0, y);
    x1 = Math.max(x1, x);
    y1 = Math.max(y1, y);
  }
  return { x0, y0, x1, y1 };
}

function shaped(text, size) {
  const key = `${size}|${text}`;
  if (memo.has(key)) return memo.get(key);
  const f = load();
  const opts = { features: { liga: false, rlig: false } };
  const paths = f.getPaths(text, 0, 0, size, opts);
  const width = f.getAdvanceWidth(text, size, opts);
  const chars = [...text];
  const list = paths.map((p, i) => {
    const contours = flatten(p.commands);
    return { ch: chars[i] ?? '', contours, box: contours.length ? bounds(contours) : null };
  });
  const done = { list, width };
  memo.set(key, done);
  return done;
}

export function glyphs(text, o) {
  const { list, width } = shaped(text, o.size);
  const dx = o.x - (o.align === 'center' ? width / 2 : o.align === 'right' ? width : 0);
  const dy = o.y;
  return list.map((g) => ({
    ch: g.ch,
    contours: g.contours.map((c) => c.map(([x, y]) => [x + dx, y + dy])),
    box: g.box && { x0: g.box.x0 + dx, y0: g.box.y0 + dy, x1: g.box.x1 + dx, y1: g.box.y1 + dy },
  }));
}

export function advance(text, size) {
  return shaped(text, size).width;
}
