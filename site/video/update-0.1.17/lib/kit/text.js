// Text as skeleton strokes, laid out with the font's advances.
import { glyph, advance } from '../glyph/index.js';

const offsets = new Map();

function places(font, line) {
  const key = `${font}|${line}`;
  if (!offsets.has(key)) {
    const chars = [...line];
    const xs = chars.map((_, i) => advance(font, chars.slice(0, i).join('')));
    offsets.set(key, { chars, xs, w: advance(font, line) });
  }
  return offsets.get(key);
}

const plain = (str) => String(str).replace(/…/g, '...');

const weight = (size) => Math.min(1.1, 0.62 + size / 200);

export function measure(str, o = {}) {
  const size = o.size ?? 32;
  const lines = plain(str).split('\n');
  const w = Math.max(...lines.map((l) => places(o.font ?? 'sans', l).w)) * size;
  return { w, h: size * (1 + (lines.length - 1) * (o.leading ?? 1.35)) };
}

export function text(str, o = {}) {
  const size = o.size ?? 32;
  const font = o.font ?? 'sans';
  const align = o.align ?? 'left';
  if (!['left', 'center', 'right'].includes(align)) throw new Error(`text align ${align}; use left, center or right`);
  const out = [];
  let g = 0;
  plain(str).split('\n').forEach((line, row) => {
    const { chars, xs, w } = places(font, line);
    const x0 = (o.x ?? 0) - (align === 'center' ? w * size / 2 : align === 'right' ? w * size : 0);
    const y0 = (o.y ?? 0) + row * size * (o.leading ?? 1.35);
    chars.forEach((ch, i) => {
      for (const s of glyph(font, ch).strokes) {
        out.push({
          pts: s.map(([gx, gy]) => [x0 + (xs[i] + gx) * size, y0 + gy * size]),
          w: o.w ?? weight(size),
          alpha: o.alpha ?? 1,
          show: [0, 1],
          app: o.app ?? null,
          link: out.length ? o.link ?? 0.5 : 1,
          g,
          ch,
        });
      }
      g++;
    });
  });
  return seat(out);
}

const gap = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);

function seat(list) {
  for (let i = 0; i < list.length; i++) {
    const pts = list[i].pts;
    if (pts.length < 4 || gap(pts[0], pts[pts.length - 1]) > 1e-3) continue;
    const prev = list[i - 1]?.pts.at(-1);
    const next = list[i + 1]?.pts[0];
    let best = 0;
    let cost = Infinity;
    for (let j = 0; j < pts.length - 1; j++) {
      const c = (prev ? gap(prev, pts[j]) : 0) + (next ? gap(pts[j], next) : 0);
      if (c < cost - 1e-9) {
        cost = c;
        best = j;
      }
    }
    list[i].pts = [...pts.slice(best, -1), ...pts.slice(0, best), pts[best]];
  }
  return list;
}
