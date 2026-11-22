// Junctions re-centred from incoming tangents; stroke ends extended.
import { degree } from './graph.js';

const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);

function outward(pts, from) {
  return from ? pts : [...pts].reverse();
}

function solve(lines, fallback, reach) {
  let a = 0;
  let b = 0;
  let c = 0;
  let px = 0;
  let py = 0;
  for (const { p, d } of lines) {
    const m00 = 1 - d[0] * d[0];
    const m01 = -d[0] * d[1];
    const m11 = 1 - d[1] * d[1];
    a += m00;
    b += m01;
    c += m11;
    px += m00 * p[0] + m01 * p[1];
    py += m01 * p[0] + m11 * p[1];
  }
  const det = a * c - b * b;
  if (Math.abs(det) < 0.05) return fallback;
  const x = [(c * px - b * py) / det, (a * py - b * px) / det];
  return dist(x, fallback) <= reach ? x : fallback;
}

export function junctions(g, width) {
  for (let n = 0; n < g.nodes.length; n++) {
    if (degree(g, n) < 3) continue;
    const node = g.nodes[n];
    const R = width(node.at) * 1.3 + 1;
    const ends = [];
    for (const e of g.edges) {
      if (e.a === n) ends.push({ e, start: true });
      if (e.b === n) ends.push({ e, start: false });
    }
    const lines = [];
    for (const end of ends) {
      const pts = outward(end.e.pts, end.start);
      const k = pts.findIndex((p) => dist(p, node.at) > R);
      if (k < 0) continue;
      let m = k;
      while (m < pts.length - 1 && dist(pts[m], pts[k]) < R * 1.2) m++;
      if (m === k) continue;
      const len = dist(pts[m], pts[k]);
      end.k = k;
      lines.push({ p: pts[k], d: [(pts[k][0] - pts[m][0]) / len, (pts[k][1] - pts[m][1]) / len] });
    }
    const J = lines.length >= 2 ? solve(lines, node.at, R * 1.6) : node.at;
    node.at = J;
    for (const end of ends) {
      if (end.k === undefined) continue;
      const pts = outward(end.e.pts, end.start);
      const cut = [J, ...pts.slice(end.k)];
      end.e.pts = end.start ? cut : cut.reverse();
    }
  }
}

export function extend(g, bin, w, width) {
  const inside = (x, y) => bin[Math.round(y) * w + Math.round(x)] === 1;
  for (let n = 0; n < g.nodes.length; n++) {
    if (!g.nodes[n].end || degree(g, n) !== 1) continue;
    const e = g.edges.find((f) => f.a === n || f.b === n);
    const pts = outward(e.pts, e.a === n);
    const tip = pts[0];
    const hw = width(tip);
    let m = 1;
    while (m < pts.length - 1 && dist(pts[m], tip) < hw * 1.5 + 2) m++;
    const len = dist(pts[m], tip);
    if (!(len > 0)) continue;
    const d = [(tip[0] - pts[m][0]) / len, (tip[1] - pts[m][1]) / len];
    let s = 0;
    while (s < hw * 3 && inside(tip[0] + d[0] * (s + 0.5), tip[1] + d[1] * (s + 0.5))) s += 0.5;
    const grow = Math.max(0, s - hw * 0.45);
    const out = [[tip[0] + d[0] * grow, tip[1] + d[1] * grow], ...pts.slice(1)];
    e.pts = e.a === n ? out : out.reverse();
  }
}
