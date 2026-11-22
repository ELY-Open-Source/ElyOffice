// Skeleton glyphs in em units, cached under .work/glyphs.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { EM, raster, advance } from './raster.js';
import { zhangSuen, tidy, distance, blobs } from './thin.js';
import { trace, prune } from './graph.js';
import { junctions, extend } from './refine.js';
import { assemble } from './assemble.js';
import { smooth, direct, order } from './shape.js';

const CACHE = fileURLToPath(new URL('../../.work/glyphs/v4', import.meta.url));
const memo = new Map();

function length(pts) {
  let s = 0;
  for (let i = 1; i < pts.length; i++) s += Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
  return s;
}

function build(font, ch) {
  const r = raster(font, ch);
  const { w, h } = r;
  const dt = distance(r.bin, w, h);
  const img = blobs(r.bin, tidy(zhangSuen(r.bin, w, h), w, h), dt, w);
  const width = ([x, y]) => {
    let m = 0;
    const cx = Math.round(x);
    const cy = Math.round(y);
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) m = Math.max(m, dt[(cy + dy) * w + cx + dx] || 0);
    return m;
  };
  const g = prune(trace(img, w), width);
  junctions(g, width);
  extend(g, r.bin, w, width);
  const widths = [];
  for (let i = 0; i < img.length; i++) if (img[i]) widths.push(dt[i]);
  widths.sort((a, b) => a - b);
  const hw = widths.length ? widths[widths.length >> 1] : 1;
  const strokes = assemble(g, width).map((s) => {
    const pts = smooth(s, hw);
    const alone = [...s.nodes].every((n) => n < 0 || g.nodes[n].end);
    if (!alone || length(pts) >= hw * 1.6) return { ...s, pts };
    const c = pts.reduce((a, p) => [a[0] + p[0] / pts.length, a[1] + p[1] / pts.length], [0, 0]);
    const r = hw * 0.3;
    const ring = [];
    for (let i = 0; i <= 12; i++) ring.push([c[0] + r * Math.sin((i * Math.PI) / 6), c[1] - r * Math.cos((i * Math.PI) / 6)]);
    return { ...s, closed: true, pts: ring };
  });
  const latin = ch.codePointAt(0) < 0x2e80;
  const list = order(strokes.map(direct), latin, EM);
  const em = (v, o) => Math.round(((v - o) / EM) * 1e4) / 1e4;
  return {
    adv: advance(font, ch),
    strokes: list.map((s) => s.pts.map(([x, y]) => [em(x, r.ox), em(y, r.oy)])),
  };
}

export function glyph(font, ch) {
  const key = `${font}|${ch}`;
  if (memo.has(key)) return memo.get(key);
  const file = path.join(CACHE, font, `${[...ch].map((c) => c.codePointAt(0).toString(16)).join('-')}.json`);
  let g;
  if (fs.existsSync(file)) g = JSON.parse(fs.readFileSync(file, 'utf8'));
  else {
    g = /\s/.test(ch) ? { adv: advance(font, ch), strokes: [] } : build(font, ch);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    const tmp = `${file}.${process.pid}`;
    fs.writeFileSync(tmp, JSON.stringify(g));
    fs.renameSync(tmp, file);
  }
  memo.set(key, g);
  return g;
}

export { advance };
