// The drawing kit a scene receives, seeded per drawing.
import { draw, boil, INK, CHARCOAL, rgba } from './stroke.js';
import { hatch, wash, smudge } from './fill.js';
import { glyphs } from './glyph.js';
import { hash, signed } from './rand.js';
import { clamp, W, H } from './time.js';
import { createCanvas } from '@napi-rs/canvas';

export function spline(pts, segs = 10, closed = false) {
  const n = pts.length;
  if (n < 3) return pts;
  const at = (i) => pts[closed ? (i + n) % n : clamp(i, 0, n - 1)];
  const out = [];
  const last = closed ? n : n - 1;
  for (let i = 0; i < last; i++) {
    const [p0, p1, p2, p3] = [at(i - 1), at(i), at(i + 1), at(i + 2)];
    for (let k = 0; k < segs; k++) {
      const u = k / segs;
      const u2 = u * u;
      const u3 = u2 * u;
      out.push([0, 1].map((c) => 0.5 * (2 * p1[c] + (-p0[c] + p2[c]) * u
        + (2 * p0[c] - 5 * p1[c] + 4 * p2[c] - p3[c]) * u2 + (-p0[c] + 3 * p1[c] - 3 * p2[c] + p3[c]) * u3)));
    }
  }
  if (!closed) out.push(pts[n - 1]);
  return out;
}

function jitter(contour, amp, seed) {
  const tx = signed(seed, 1) * amp;
  const ty = signed(seed, 2) * amp;
  return contour.map(([x, y], i) => [x + tx + signed(seed, 3, i >> 2) * amp * 0.5, y + ty + signed(seed, 4, i >> 2) * amp * 0.5]);
}

function path(ctx, contours) {
  ctx.beginPath();
  for (const c of contours) {
    c.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
    ctx.closePath();
  }
}

let layers = null;
function scratch() {
  layers ??= [createCanvas(W, H), createCanvas(W, H)].map((c) => c.getContext('2d'));
  for (const c of layers) c.clearRect(0, 0, W, H);
  return layers;
}

export function kit(ctx, drawing, light) {
  let count = 0;
  let lifts = 0;
  const seed = (id) => (id === undefined ? hash(drawing, 0x5eed, count++) : hash(drawing, id));
  const ink = {
    ctx,
    drawing,
    light,
    seed,
    INK,
    CHARCOAL,
    line: (pts, o = {}) => draw(ctx, pts, { width: 3, ...o }, seed(o.id)),
    curve: (pts, o = {}) => draw(ctx, spline(pts, o.segs ?? 10, o.closed), { width: 3, ...o }, seed(o.id)),
    boiled: (pts, o = {}) => boil(pts, { width: 3, ...o }, seed(o.id)),
    shape: (contours, o = {}) => contours.forEach((c) => draw(ctx, c, { width: 2.4, closed: true, ...o }, seed())),
    hatch: (shape, o = {}) => hatch(ctx, shape, o, seed(o.id)),
    wash: (shape, o = {}) => wash(ctx, shape, o, seed(o.id)),
    smudge: (x, y, r, o = {}) => smudge(ctx, x, y, r, o, seed(o.id)),
    glyphs,
    lift(height, [ax, ay], paint) {
      const [layer, shade] = scratch();
      paint(kit(layer, hash(drawing, 0x11f7, lifts++), light));
      if (light && light.i > 0.01) {
        const dx = ax - light.x;
        const dy = ay - light.y;
        const dist = Math.hypot(dx, dy) || 1;
        const reach = Math.min(90, (height * dist) / 140);
        shade.drawImage(layer.canvas, 0, 0);
        shade.globalCompositeOperation = 'source-in';
        shade.fillStyle = 'rgb(60,40,30)';
        shade.fillRect(0, 0, W, H);
        shade.globalCompositeOperation = 'source-over';
        ctx.save();
        ctx.globalCompositeOperation = 'multiply';
        ctx.globalAlpha = 0.55 * Math.min(1, light.i);
        ctx.filter = `blur(${(1.5 + reach * 0.12).toFixed(1)}px)`;
        ctx.drawImage(shade.canvas, (dx / dist) * reach, (dy / dist) * reach);
        ctx.restore();
      }
      ctx.drawImage(layer.canvas, 0, 0);
    },
    text: (str, o) => ink.write(glyphs(str, o), o),
    write(list, o) {
      const size = o.size;
      const small = size < 44;
      const reveal = (o.upto ?? 1) * list.length;
      list.forEach((g, i) => {
        const part = clamp(reveal - i);
        if (part <= 0 || !g.contours.length) return;
        const s = seed();
        const color = o.color ?? (o.style === 'charcoal' ? CHARCOAL : INK);
        const amp = o.boil ?? Math.min(1.4, 0.25 + size * 0.006);
        if (o.fill !== 'none') {
          const shape = g.contours.map((c, k) => jitter(c, amp, hash(s, k)));
          if (o.fill === 'hatch') {
            hatch(ctx, shape, { gap: o.gap ?? Math.max(3, size * 0.04), width: Math.max(1, size * 0.013),
              color, alpha: (o.alpha ?? 0.8) * part, angle: o.angle ?? -0.95, style: o.style }, hash(s, 9));
          } else {
            path(ctx, shape);
            ctx.fillStyle = rgba(color, (o.alpha ?? 0.92) * part * (small ? 1 : 0.86));
            ctx.fill('nonzero');
          }
        }
        if (small && o.outline !== true) return;
        const width = o.width ?? Math.max(1, size * 0.016);
        g.contours.forEach((c, k) => draw(ctx, c, {
          width, style: o.style, color, alpha: o.alpha ?? 0.9, closed: true, upto: part,
          boil: amp, overshoot: width * 2.5, taper: [0.04, 0.08], pressure: 0.35,
        }, hash(s, 20, k)));
      });
      return list;
    },
  };
  return ink;
}
