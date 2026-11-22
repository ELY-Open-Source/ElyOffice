// Stroke smoothing at corners, simplification, direction, order.

const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);

function corners(pts, k) {
  const ang = pts.map((p, i) => {
    if (i < k || i >= pts.length - k) return 0;
    const a = [p[0] - pts[i - k][0], p[1] - pts[i - k][1]];
    const b = [pts[i + k][0] - p[0], pts[i + k][1] - p[1]];
    const d = Math.hypot(...a) * Math.hypot(...b) || 1;
    return Math.acos(Math.max(-1, Math.min(1, (a[0] * b[0] + a[1] * b[1]) / d)));
  });
  const out = [];
  for (let i = k; i < pts.length - k; i++) {
    if (ang[i] < 0.95) continue;
    let best = true;
    for (let j = Math.max(0, i - k); j <= Math.min(pts.length - 1, i + k); j++) if (ang[j] > ang[i] || (ang[j] === ang[i] && j < i)) best = false;
    if (best) out.push(i);
  }
  return out;
}

function blur(pts, closed, rounds) {
  let p = pts;
  const n = p.length;
  for (let r = 0; r < rounds; r++) {
    p = p.map((q, i) => {
      if (!closed && (i === 0 || i === n - 1)) return q;
      const a = p[(i - 1 + n) % n];
      const b = p[(i + 1) % n];
      return [(a[0] + 2 * q[0] + b[0]) / 4, (a[1] + 2 * q[1] + b[1]) / 4];
    });
  }
  return p;
}

function rdp(pts, eps) {
  if (pts.length < 3) return pts;
  const keep = new Uint8Array(pts.length);
  keep[0] = keep[pts.length - 1] = 1;
  const stack = [[0, pts.length - 1]];
  while (stack.length) {
    const [a, b] = stack.pop();
    const [ax, ay] = pts[a];
    const dx = pts[b][0] - ax;
    const dy = pts[b][1] - ay;
    const L = Math.hypot(dx, dy) || 1e-9;
    let worst = -1;
    let at = -1;
    for (let i = a + 1; i < b; i++) {
      const d = Math.abs((pts[i][0] - ax) * dy - (pts[i][1] - ay) * dx) / L;
      if (d > worst) {
        worst = d;
        at = i;
      }
    }
    if (worst > eps) {
      keep[at] = 1;
      stack.push([a, at], [at, b]);
    }
  }
  return pts.filter((_, i) => keep[i]);
}

function simplify(pts, eps) {
  if (pts.length < 4 || dist(pts[0], pts[pts.length - 1]) > 1e-6) return rdp(pts, eps);
  let far = 0;
  for (let i = 1; i < pts.length; i++) if (dist(pts[i], pts[0]) > dist(pts[far], pts[0])) far = i;
  return [...rdp(pts.slice(0, far + 1), eps), ...rdp(pts.slice(far), eps).slice(1)];
}

export function smooth(stroke, hw) {
  const { pts, closed } = stroke;
  if (pts.length < 3) return pts;
  const body = closed && dist(pts[0], pts[pts.length - 1]) < 1.5 ? pts.slice(0, -1) : pts;
  const cuts = corners(body, Math.max(3, Math.round(hw * 1.1)));
  let out;
  if (!cuts.length) out = blur(body, closed, 5);
  else {
    const bounds = [0, ...cuts, body.length - 1];
    out = [];
    for (let s = 0; s < bounds.length - 1; s++) {
      const piece = blur(body.slice(bounds[s], bounds[s + 1] + 1), false, 5);
      out.push(...(out.length ? piece.slice(1) : piece));
    }
  }
  if (closed) out.push(out[0]);
  return simplify(out, 0.3);
}

function area(pts) {
  let s = 0;
  for (let i = 0; i < pts.length - 1; i++) s += pts[i][0] * pts[i + 1][1] - pts[i + 1][0] * pts[i][1];
  return s;
}

export function direct(stroke) {
  let pts = stroke.pts;
  if (stroke.closed && pts.length > 3) {
    let top = 0;
    for (let i = 1; i < pts.length - 1; i++) if (pts[i][1] < pts[top][1] - 1e-6) top = i;
    const ring = [...pts.slice(top, -1), ...pts.slice(0, top), pts[top]];
    pts = area(ring) > 0 ? [...ring].reverse() : ring;
  } else if (pts.length > 1) {
    const a = pts[0];
    const b = pts[pts.length - 1];
    const flat = Math.abs(b[0] - a[0]) > Math.abs(b[1] - a[1]) * 1.15;
    if (flat ? b[0] < a[0] : b[1] < a[1]) pts = [...pts].reverse();
  }
  return { ...stroke, pts };
}

function crosses(p, q) {
  const side = (a, b, c) => (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
  for (let i = 1; i < p.length; i++) {
    for (let j = 1; j < q.length; j++) {
      const d1 = side(p[i - 1], p[i], q[j - 1]);
      const d2 = side(p[i - 1], p[i], q[j]);
      const d3 = side(q[j - 1], q[j], p[i - 1]);
      const d4 = side(q[j - 1], q[j], p[i]);
      if (d1 * d2 < 0 && d3 * d4 < 0) return true;
    }
  }
  return false;
}

function box(list) {
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const s of list) for (const [x, y] of s.pts) {
    x0 = Math.min(x0, x);
    y0 = Math.min(y0, y);
    x1 = Math.max(x1, x);
    y1 = Math.max(y1, y);
  }
  return { x0, y0, x1, y1 };
}

export function order(strokes, latin, em) {
  const root = strokes.map((_, i) => i);
  const top = (i) => (root[i] === i ? i : (root[i] = top(root[i])));
  for (let i = 0; i < strokes.length; i++) {
    for (let j = i + 1; j < strokes.length; j++) {
      if ([...strokes[i].nodes].some((n) => n >= 0 && strokes[j].nodes.has(n))) root[top(i)] = top(j);
    }
  }
  const groups = new Map();
  strokes.forEach((s, i) => groups.set(top(i), [...(groups.get(top(i)) ?? []), s]));
  const comps = [...groups.values()].map((list) => ({ list, b: box(list) }));
  comps.sort((p, q) => p.b.x0 - q.b.x0);
  const cols = [];
  for (const c of comps) {
    const col = cols[cols.length - 1];
    const w = Math.max(c.b.x1 - c.b.x0, em * 0.02);
    if (col && Math.min(col.x1, c.b.x1) - c.b.x0 > w * 0.3 - em * 0.04) {
      col.items.push(c);
      col.x1 = Math.max(col.x1, c.b.x1);
    } else cols.push({ x1: c.b.x1, items: [c] });
  }
  const tiny = (c) => Math.max(c.b.x1 - c.b.x0, c.b.y1 - c.b.y0) < em * 0.16;
  const out = [];
  for (const col of cols) {
    col.items.sort((p, q) => (latin ? tiny(p) - tiny(q) : 0) || (p.b.y0 + p.b.y1) - (q.b.y0 + q.b.y1));
    for (const c of col.items) {
      const list = c.list.map((s) => ({ s, k: s.pts[0][1] + 0.5 * s.pts[0][0] }));
      list.sort((p, q) => p.k - q.k);
      const seq = list.map((l) => l.s);
      for (let i = 0; i < seq.length; i++) {
        for (let j = i + 1; j < seq.length; j++) {
          const [a, b] = [seq[i].pts, seq[j].pts];
          const flatB = Math.abs(b[b.length - 1][0] - b[0][0]) > Math.abs(b[b.length - 1][1] - b[0][1]);
          const flatA = Math.abs(a[a.length - 1][0] - a[0][0]) > Math.abs(a[a.length - 1][1] - a[0][1]);
          if (!latin && flatB && !flatA && crosses(a, b)) seq.splice(i, 0, seq.splice(j, 1)[0]);
        }
      }
      out.push(...seq);
    }
  }
  return out;
}
