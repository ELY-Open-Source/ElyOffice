// Skeleton pixels to a graph: nodes, edges, spurs pruned.
import { NEIGHBOURS } from './thin.js';

const { DX, DY } = NEIGHBOURS;

function around(img, w, i) {
  const x = i % w;
  const y = (i - x) / w;
  const out = [];
  for (let k = 0; k < 8; k++) {
    const j = (y + DY[k]) * w + x + DX[k];
    if (img[j]) out.push(j);
  }
  return out;
}

function centre(pixels, w) {
  let x = 0;
  let y = 0;
  for (const i of pixels) {
    x += i % w;
    y += Math.floor(i / w);
  }
  return [x / pixels.length, y / pixels.length];
}

export function trace(img, w) {
  const deg = new Map();
  for (let i = 0; i < img.length; i++) if (img[i]) deg.set(i, around(img, w, i).length);
  const nodeOf = new Map();
  const nodes = [];
  for (const [i, d] of deg) {
    if (d === 2 || nodeOf.has(i)) continue;
    const pixels = [i];
    nodeOf.set(i, nodes.length);
    if (d > 2) {
      for (let q = 0; q < pixels.length; q++) {
        for (const j of around(img, w, pixels[q])) {
          if (deg.get(j) > 2 && !nodeOf.has(j)) {
            nodeOf.set(j, nodes.length);
            pixels.push(j);
          }
        }
      }
    }
    nodes.push({ pixels, at: centre(pixels, w), end: d <= 1 });
  }
  const edges = [];
  const seen = new Set();
  const xy = (i) => [i % w, Math.floor(i / w)];
  for (let n = 0; n < nodes.length; n++) {
    for (const p of nodes[n].pixels) {
      for (const first of around(img, w, p)) {
        if (nodeOf.get(first) === n || deg.get(first) === -1) continue;
        const key = `${p}>${first}`;
        if (seen.has(key)) continue;
        const chain = [p, first];
        let prev = p;
        let cur = first;
        while (!nodeOf.has(cur)) {
          const next = around(img, w, cur).find((j) => j !== prev && !(chain.length > 2 && j === chain[chain.length - 3]));
          if (next === undefined) break;
          prev = cur;
          cur = next;
          chain.push(cur);
        }
        if (!nodeOf.has(cur)) continue;
        seen.add(`${cur}>${chain[chain.length - 2]}`);
        seen.add(key);
        for (let c = 1; c < chain.length - 1; c++) deg.set(chain[c], -1);
        edges.push({ a: n, b: nodeOf.get(cur), pts: chain.map(xy) });
      }
    }
  }
  for (const [i, d] of deg) {
    if (d !== 2) continue;
    const chain = [i];
    deg.set(i, -1);
    let cur = i;
    for (;;) {
      const next = around(img, w, cur).find((j) => deg.get(j) === 2);
      if (next === undefined) break;
      deg.set(next, -1);
      chain.push(next);
      cur = next;
    }
    if (chain.length < 4) continue;
    chain.push(i);
    edges.push({ a: -1, b: -1, loop: true, pts: chain.map(xy) });
  }
  return { nodes, edges };
}

function edgeLength(e) {
  let s = 0;
  for (let i = 1; i < e.pts.length; i++) s += Math.hypot(e.pts[i][0] - e.pts[i - 1][0], e.pts[i][1] - e.pts[i - 1][1]);
  return s;
}

export function degree(g, n) {
  let d = 0;
  for (const e of g.edges) d += (e.a === n) + (e.b === n);
  return d;
}

export function prune(g, width) {
  for (let round = 0; round < 3; round++) {
    let cut = false;
    for (const e of [...g.edges]) {
      if (e.loop || e.a === e.b) continue;
      const [tip, base] = g.nodes[e.a].end && degree(g, e.a) === 1 ? [e.a, e.b] : [e.b, e.a];
      if (!(g.nodes[tip].end && degree(g, tip) === 1 && degree(g, base) >= 3)) continue;
      if (edgeLength(e) <= width(g.nodes[base].at) + width(g.nodes[tip].at) * 0.8 + 1) {
        g.edges.splice(g.edges.indexOf(e), 1);
        g.nodes[tip].dead = true;
        cut = true;
      }
    }
    if (!cut) break;
  }
  contract(g, width);
  for (let n = 0; n < g.nodes.length; n++) if (degree(g, n) === 2) join(g, n);
  return g;
}

function contract(g, width) {
  for (const e of [...g.edges]) {
    if (e.loop || e.a === e.b || !g.edges.includes(e)) continue;
    if (degree(g, e.a) < 3 || degree(g, e.b) < 3) continue;
    const r = Math.max(width(g.nodes[e.a].at), width(g.nodes[e.b].at));
    if (edgeLength(e) > r * 2 + 1) continue;
    const keep = e.a;
    const gone = e.b;
    g.edges.splice(g.edges.indexOf(e), 1);
    const A = g.nodes[keep];
    const B = g.nodes[gone];
    A.at = [(A.at[0] + B.at[0]) / 2, (A.at[1] + B.at[1]) / 2];
    for (const f of g.edges) {
      if (f.a === gone) f.a = keep;
      if (f.b === gone) f.b = keep;
    }
  }
}

function join(g, n) {
  const ends = g.edges.filter((e) => e.a === n || e.b === n);
  if (ends.length !== 2) {
    if (ends.length === 1 && ends[0].a === n && ends[0].b === n) {
      ends[0].loop = true;
      ends[0].a = ends[0].b = -1;
    }
    return;
  }
  const [e, f] = ends;
  const into = e.b === n ? e.pts : [...e.pts].reverse();
  const out = f.a === n ? f.pts : [...f.pts].reverse();
  const merged = { a: e.b === n ? e.a : e.b, b: f.a === n ? f.b : f.a, pts: [...into, ...out.slice(1)] };
  g.edges.splice(g.edges.indexOf(e), 1);
  g.edges.splice(g.edges.indexOf(f), 1);
  g.edges.push(merged);
}
