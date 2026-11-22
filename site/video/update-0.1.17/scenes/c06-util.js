// Shared helpers for c06: reveals, springs, clipping, small glyphs.
import { kit as k } from '../lib/kit/index.js';

export const hide = (s) => ({ ...s, show: [0, 0] });

// Draws strokes on in order, u of their total length.
export function reveal(strokes, u) {
  const lens = strokes.map((s) => k.length(s));
  let left = k.clamp(u) * lens.reduce((a, b) => a + b, 0);
  return strokes.map((s, i) => {
    const take = Math.min(left, lens[i]);
    left -= take;
    return { ...s, show: take > 1e-3 ? [0, take / lens[i]] : [0, 0] };
  });
}

// Step response: 0 to 1, overshoots, settles.
export function settle(tau, freq = 22, damp = 9) {
  if (tau <= 0) return 0;
  return 1 - Math.exp(-damp * tau) * (Math.cos(freq * tau) + (damp / freq) * Math.sin(freq * tau));
}

// Restarts a closed loop at its vertex nearest p.
export function nearest(stroke, p) {
  const ring = stroke.pts.slice(0, -1);
  let best = 0;
  ring.forEach((q, i) => {
    if (Math.hypot(q[0] - p[0], q[1] - p[1]) < Math.hypot(ring[best][0] - p[0], ring[best][1] - p[1])) best = i;
  });
  return { ...stroke, pts: [...ring.slice(best), ...ring.slice(0, best), ring[best]] };
}

// Faint landing before a glyph keeps its inbound tail light.
export function pad(stroke, alpha) {
  const p = stroke.pts;
  const at = p.find((q) => Math.hypot(q[0] - p[0][0], q[1] - p[0][1]) >= 6) ?? p[p.length - 1];
  const d = Math.hypot(at[0] - p[0][0], at[1] - p[0][1]) || 1;
  const from = [p[0][0] - ((at[0] - p[0][0]) / d) * 4, p[0][1] - ((at[1] - p[0][1]) / d) * 4];
  const on = stroke.show[1] - stroke.show[0] > 1e-6;
  return { pts: [from, p[0]], w: 0.35, alpha, show: on ? [0, 1] : [0, 0], app: stroke.app, link: stroke.link };
}

export function shift(strokes, dx, dy) {
  return strokes.map((s) => ({ ...s, pts: s.pts.map(([x, y]) => [x + dx, y + dy]) }));
}

export function scale(strokes, s, ox, oy) {
  return strokes.map((st) => ({ ...st, pts: st.pts.map(([x, y]) => [ox + (x - ox) * s, oy + (y - oy) * s]) }));
}

function cross(a, b, y) {
  const u = (y - a[1]) / (b[1] - a[1]);
  return [a[0] + (b[0] - a[0]) * u, y];
}

// Closed loop cut to band lo..hi, as one run.
export function band(pts, lo, hi) {
  const inside = (p) => p[1] >= lo && p[1] <= hi;
  const ring = pts.slice(0, -1);
  const out = ring.findIndex((p) => !inside(p));
  if (out < 0) return pts;
  const seq = [...ring.slice(out), ...ring.slice(0, out), ring[out]];
  const run = [];
  for (let i = 1; i < seq.length; i++) {
    const a = seq[i - 1];
    const b = seq[i];
    const ys = b[1] > a[1] ? [lo, hi] : [hi, lo];
    for (const y of ys) {
      if ((a[1] - y) * (b[1] - y) < 0) run.push(cross(a, b, y));
    }
    if (inside(b) && i < seq.length - 1) run.push(b);
  }
  return run.length > 1 ? run : null;
}

// Fixed stroke slots for text whose characters change.
export function ticker(chars, slots, o) {
  const per = Math.max(...chars.map((c) => k.text(c, o).length));
  return (str, x, y) => {
    const out = [];
    const glyphs = [...str.padStart(slots)];
    const w = k.measure(str, o).w;
    let cx = x - (o.align === 'right' ? w : 0);
    for (const ch of glyphs) {
      const sw = ch === ' ' ? 0 : k.measure(ch, o).w;
      const g = ch === ' ' ? [] : k.text(ch, { ...o, x: cx, y, align: 'left' });
      for (let i = 0; i < per; i++) out.push(g[i] ?? { ...k.line(x, y, x + 1, y), show: [0, 0], g: -1 });
      cx += sw;
    }
    return out;
  };
}

export function chevron(x, y, r, angle, o) {
  const c = Math.cos(angle);
  const s = Math.sin(angle);
  const at = ([px, py]) => [x + c * px - s * py, y + s * px + c * py];
  return k.stroke([[-r * 0.5, -r], [r * 0.5, 0], [-r * 0.5, r]].map(at), o);
}

export function folder(x, y, o) {
  return k.poly([[x, y + 22], [x, y], [x + 10, y], [x + 14, y + 5], [x + 30, y + 5], [x + 30, y + 22]], { ...o, closed: true });
}

export function page(x, y, o) {
  return k.poly([[x + 12, y], [x, y], [x, y + 26], [x + 20, y + 26], [x + 20, y + 8], [x + 12, y], [x + 12, y + 8], [x + 20, y + 8]], o);
}

export function capsule(x, y0, y1, half, o) {
  const r = Math.max(0.5, half);
  return k.rect(x - r, y0, 2 * r, Math.max(2 * r, y1 - y0), { ...o, r });
}

// Counter-clockwise reset arrow, head first, spun by a.
export function resetArrow(x, y, r, a, o) {
  const from = (155 / 180) * Math.PI + a;
  const to = from - (300 / 180) * Math.PI;
  const arc = k.arc(x, y, r, to, from, o).pts;
  const tip = arc[0];
  const tan = [Math.sin(to), -Math.cos(to)];
  const nrm = [Math.cos(to), Math.sin(to)];
  const h = r * 0.55;
  const barb = (sign) => [tip[0] + (-tan[0] + sign * nrm[0]) * h, tip[1] + (-tan[1] + sign * nrm[1]) * h];
  return k.stroke([barb(1), tip, barb(-1), tip, ...arc.slice(1)], o);
}
