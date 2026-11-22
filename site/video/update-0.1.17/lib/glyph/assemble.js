// Graph edges joined into pen strokes by good continuation.
import { degree } from './graph.js';

const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);

function leaving(pts, reach) {
  const p = pts[0];
  let m = 1;
  while (m < pts.length - 1 && dist(pts[m], p) < reach) m++;
  const len = dist(pts[m], p) || 1;
  return [(pts[m][0] - p[0]) / len, (pts[m][1] - p[1]) / len];
}

export function assemble(g, width) {
  const key = (i, side) => i * 2 + side;
  const pair = new Map();
  for (let n = 0; n < g.nodes.length; n++) {
    if (degree(g, n) < 3) continue;
    const reach = width(g.nodes[n].at) * 2.5 + 2;
    const ends = [];
    g.edges.forEach((e, i) => {
      if (e.a === n) ends.push({ id: key(i, 0), d: leaving(e.pts, reach) });
      if (e.b === n) ends.push({ id: key(i, 1), d: leaving([...e.pts].reverse(), reach) });
    });
    const cands = [];
    for (let i = 0; i < ends.length; i++) {
      for (let j = i + 1; j < ends.length; j++) {
        const dot = -(ends[i].d[0] * ends[j].d[0] + ends[i].d[1] * ends[j].d[1]);
        cands.push({ i, j, dev: Math.acos(Math.max(-1, Math.min(1, dot))) });
      }
    }
    cands.sort((p, q) => p.dev - q.dev);
    const taken = new Set();
    for (const c of cands) {
      if (c.dev > 0.75 || taken.has(c.i) || taken.has(c.j)) continue;
      taken.add(c.i);
      taken.add(c.j);
      pair.set(ends[c.i].id, ends[c.j].id);
      pair.set(ends[c.j].id, ends[c.i].id);
    }
  }
  const used = new Set();
  const strokes = [];
  const walk = (i, side) => {
    const pts = [];
    const nodes = new Set();
    let closed = false;
    for (;;) {
      used.add(i);
      const e = g.edges[i];
      const seg = side === 0 ? e.pts : [...e.pts].reverse();
      pts.push(...(pts.length ? seg.slice(1) : seg));
      nodes.add(e.a).add(e.b);
      const next = pair.get(key(i, 1 - side));
      if (next === undefined) break;
      const ni = next >> 1;
      if (used.has(ni)) {
        closed = true;
        break;
      }
      i = ni;
      side = next & 1;
    }
    strokes.push({ pts, nodes, closed: closed || !!g.edges[i].loop });
  };
  const starts = [];
  g.edges.forEach((e, i) => {
    for (const side of [0, 1]) {
      const n = side === 0 ? e.a : e.b;
      if (n >= 0 && !pair.has(key(i, side))) starts.push([i, side, g.nodes[n].end ? 0 : 1]);
    }
  });
  starts.sort((p, q) => p[2] - q[2]);
  for (const [i, side] of starts) if (!used.has(i)) walk(i, side);
  g.edges.forEach((e, i) => {
    if (!used.has(i)) walk(i, 0);
  });
  g.nodes.forEach((node, n) => {
    if (node.end && !node.dead && degree(g, n) === 0) strokes.push({ pts: [node.at], nodes: new Set([n]), closed: false });
  });
  return strokes;
}
