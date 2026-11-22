// Polyline arithmetic: lengths, sampling, slices, tangents.

export function length(pts) {
  let s = 0;
  for (let i = 1; i < pts.length; i++) s += Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
  return s;
}

export function cumulative(pts) {
  const c = new Float64Array(pts.length);
  for (let i = 1; i < pts.length; i++) c[i] = c[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
  return c;
}

function find(c, d) {
  let lo = 0;
  let hi = c.length - 1;
  while (hi - lo > 1) {
    const m = (lo + hi) >> 1;
    if (c[m] <= d) lo = m;
    else hi = m;
  }
  return lo;
}

export function at(pts, c, d) {
  if (d <= 0) return pts[0].slice(0, 2);
  const L = c[c.length - 1];
  if (d >= L) return pts[pts.length - 1].slice(0, 2);
  const i = find(c, d);
  const seg = c[i + 1] - c[i];
  const u = seg > 0 ? (d - c[i]) / seg : 0;
  return [pts[i][0] + (pts[i + 1][0] - pts[i][0]) * u, pts[i][1] + (pts[i + 1][1] - pts[i][1]) * u];
}

export function slice(pts, a, b, c = cumulative(pts)) {
  const L = c[c.length - 1];
  const da = a * L;
  const db = b * L;
  if (!(db > da)) return [at(pts, c, da)];
  const out = [at(pts, c, da)];
  for (let i = 1; i < pts.length - 1; i++) if (c[i] > da && c[i] < db) out.push([pts[i][0], pts[i][1]]);
  out.push(at(pts, c, db));
  return out;
}

export function resample(pts, n) {
  const c = cumulative(pts);
  const L = c[c.length - 1];
  const out = [];
  for (let i = 0; i < n; i++) out.push(at(pts, c, (L * i) / (n - 1)));
  return out;
}

function unit(dx, dy) {
  const d = Math.hypot(dx, dy);
  return d > 1e-9 ? [dx / d, dy / d] : null;
}

export function startTangent(pts, reach = 6) {
  for (let i = 1; i < pts.length; i++) {
    const v = unit(pts[i][0] - pts[0][0], pts[i][1] - pts[0][1]);
    if (v && (Math.hypot(pts[i][0] - pts[0][0], pts[i][1] - pts[0][1]) >= reach || i === pts.length - 1)) return v;
  }
  return [1, 0];
}

export function endTangent(pts, reach = 6) {
  const n = pts.length - 1;
  for (let i = n - 1; i >= 0; i--) {
    const v = unit(pts[n][0] - pts[i][0], pts[n][1] - pts[i][1]);
    if (v && (Math.hypot(pts[n][0] - pts[i][0], pts[n][1] - pts[i][1]) >= reach || i === 0)) return v;
  }
  return [1, 0];
}

export function cubic(p0, p1, p2, p3, step = 6) {
  const est = Math.hypot(p1[0] - p0[0], p1[1] - p0[1]) + Math.hypot(p2[0] - p1[0], p2[1] - p1[1])
    + Math.hypot(p3[0] - p2[0], p3[1] - p2[1]);
  const n = Math.max(2, Math.min(400, Math.ceil(est / step)));
  const out = [];
  for (let i = 0; i <= n; i++) {
    const u = i / n;
    const v = 1 - u;
    const a = v * v * v;
    const b = 3 * v * v * u;
    const c = 3 * v * u * u;
    const d = u * u * u;
    out.push([a * p0[0] + b * p1[0] + c * p2[0] + d * p3[0], a * p0[1] + b * p1[1] + c * p2[1] + d * p3[1]]);
  }
  return out;
}

export function bridge(p0, t0, p1, t1, step = 6, land = 900) {
  const d = Math.hypot(p1[0] - p0[0], p1[1] - p0[1]);
  const h = Math.min(Math.max(d * 0.38, 3), 900);
  const g = Math.min(Math.max(d * 0.38, 3), land);
  return cubic(p0, [p0[0] + t0[0] * h, p0[1] + t0[1] * h], [p1[0] - t1[0] * g, p1[1] - t1[1] * g], p1, step);
}

export function densify(pts, step) {
  const out = [pts[0]];
  for (let i = 1; i < pts.length; i++) {
    const [x0, y0] = pts[i - 1];
    const [x1, y1] = pts[i];
    const n = Math.ceil(Math.hypot(x1 - x0, y1 - y0) / step);
    for (let k = 1; k < n; k++) out.push([x0 + ((x1 - x0) * k) / n, y0 + ((y1 - y0) * k) / n]);
    out.push(pts[i]);
  }
  return out;
}
