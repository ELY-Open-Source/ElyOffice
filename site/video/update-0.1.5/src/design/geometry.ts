export type Point = [number, number];

const TAU = Math.PI * 2;

// An arc sampled in y-down space; angles run clockwise.
export const arc = (cx: number, cy: number, r: number, from: number, to: number, steps: number): Point[] => {
  const out: Point[] = [];
  for (let i = 0; i <= steps; i++) {
    const a = from + ((to - from) * i) / steps;
    out.push([cx + r * Math.cos(a), cy + r * Math.sin(a)]);
  }
  return out;
};

export const roundedRect = (x: number, y: number, w: number, h: number, r: number, steps = 10): Point[] => [
  ...arc(x + w - r, y + r, r, -TAU / 4, 0, steps),
  ...arc(x + w - r, y + h - r, r, 0, TAU / 4, steps),
  ...arc(x + r, y + h - r, r, TAU / 4, TAU / 2, steps),
  ...arc(x + r, y + r, r, TAU / 2, (TAU * 3) / 4, steps),
];

export const circle = (r: number, steps = 96): Point[] =>
  Array.from({ length: steps }, (_, i): Point => [r * Math.cos((TAU * i) / steps - TAU / 4), r * Math.sin((TAU * i) / steps - TAU / 4)]);

// Exact rounded rectangle as an SVG path, for the stage.
export const roundedRectPath = (x: number, y: number, w: number, h: number, r: number) =>
  `M${x + r} ${y}H${x + w - r}A${r} ${r} 0 0 1 ${x + w} ${y + r}V${y + h - r}A${r} ${r} 0 0 1 ${x + w - r} ${y + h}` +
  `H${x + r}A${r} ${r} 0 0 1 ${x} ${y + h - r}V${y + r}A${r} ${r} 0 0 1 ${x + r} ${y}Z`;

export const toPath = (points: Point[], scale: number, cx: number, cy: number) => {
  let d = '';
  for (let i = 0; i < points.length; i++) {
    const [x, y] = points[i];
    d += `${i === 0 ? 'M' : 'L'}${(cx + x * scale).toFixed(2)} ${(cy + y * scale).toFixed(2)}`;
  }
  return `${d}Z`;
};

// ---- Signed-distance silhouettes ------------------------------------------

export type Sdf = (x: number, y: number) => number;

export const sdfCircle = (cx: number, cy: number, r: number): Sdf => (x, y) => Math.hypot(x - cx, y - cy) - r;

export const sdfCapsule = (ax: number, ay: number, bx: number, by: number, r: number): Sdf => {
  const dx = bx - ax;
  const dy = by - ay;
  const len2 = dx * dx + dy * dy;
  return (x, y) => {
    const t = Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / len2));
    return Math.hypot(x - ax - t * dx, y - ay - t * dy) - r;
  };
};

export const sdfRoundedBox = (x: number, y: number, w: number, h: number, r: number): Sdf => {
  const cx = x + w / 2;
  const cy = y + h / 2;
  return (px, py) => {
    const qx = Math.abs(px - cx) - w / 2 + r;
    const qy = Math.abs(py - cy) - h / 2 + r;
    return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - r;
  };
};

// A cubic Bézier stroke as a chain of capsules.
export const sdfCubicStroke = (p: [Point, Point, Point, Point], r: number, segments = 32): Sdf => {
  const at = (t: number): Point => {
    const u = 1 - t;
    return [
      u * u * u * p[0][0] + 3 * u * u * t * p[1][0] + 3 * u * t * t * p[2][0] + t * t * t * p[3][0],
      u * u * u * p[0][1] + 3 * u * u * t * p[1][1] + 3 * u * t * t * p[2][1] + t * t * t * p[3][1],
    ];
  };
  const parts: Sdf[] = [];
  for (let i = 0; i < segments; i++) {
    const a = at(i / segments);
    const b = at((i + 1) / segments);
    parts.push(sdfCapsule(a[0], a[1], b[0], b[1], r));
  }
  return union(parts);
};

export const union = (parts: Sdf[]): Sdf => (x, y) => {
  let d = Infinity;
  for (const part of parts) d = Math.min(d, part(x, y));
  return d;
};

// Marching squares on the zero level; the longest closed loop.
export const contour = (sdf: Sdf, x0: number, y0: number, x1: number, y1: number, step: number): Point[] => {
  const nx = Math.ceil((x1 - x0) / step) + 1;
  const ny = Math.ceil((y1 - y0) / step) + 1;
  const v = new Float64Array(nx * ny);
  for (let j = 0; j < ny; j++) for (let i = 0; i < nx; i++) v[j * nx + i] = sdf(x0 + i * step, y0 + j * step);
  const at = (i: number, j: number) => v[j * nx + i];

  const points = new Map<string, Point>();
  const links = new Map<string, string[]>();
  const edgePoint = (key: string, ax: number, ay: number, bx: number, by: number, va: number, vb: number) => {
    if (!points.has(key)) {
      const t = va / (va - vb);
      points.set(key, [x0 + (ax + (bx - ax) * t) * step, y0 + (ay + (by - ay) * t) * step]);
    }
    return key;
  };
  const link = (a: string, b: string) => {
    links.set(a, [...(links.get(a) ?? []), b]);
    links.set(b, [...(links.get(b) ?? []), a]);
  };

  for (let j = 0; j < ny - 1; j++) {
    for (let i = 0; i < nx - 1; i++) {
      const a = at(i, j);
      const b = at(i + 1, j);
      const c = at(i + 1, j + 1);
      const d = at(i, j + 1);
      const top = a < 0 !== b < 0 ? edgePoint(`h${i},${j}`, i, j, i + 1, j, a, b) : null;
      const right = b < 0 !== c < 0 ? edgePoint(`v${i + 1},${j}`, i + 1, j, i + 1, j + 1, b, c) : null;
      const bottom = d < 0 !== c < 0 ? edgePoint(`h${i},${j + 1}`, i, j + 1, i + 1, j + 1, d, c) : null;
      const left = a < 0 !== d < 0 ? edgePoint(`v${i},${j}`, i, j, i, j + 1, a, d) : null;
      const crossed = [top, right, bottom, left].filter((e): e is string => e !== null);
      if (crossed.length === 2) link(crossed[0], crossed[1]);
      else if (crossed.length === 4) {
        const centreInside = (a + b + c + d) / 4 < 0;
        const aInside = a < 0;
        if (aInside === centreInside) {
          link(top!, right!);
          link(bottom!, left!);
        } else {
          link(top!, left!);
          link(right!, bottom!);
        }
      }
    }
  }

  const seen = new Set<string>();
  let best: Point[] = [];
  for (const start of links.keys()) {
    if (seen.has(start)) continue;
    const loop: Point[] = [];
    let prev = '';
    let cur = start;
    while (!seen.has(cur)) {
      seen.add(cur);
      loop.push(points.get(cur)!);
      const next = links.get(cur)!.find((n) => n !== prev && !seen.has(n));
      if (!next) break;
      prev = cur;
      cur = next;
    }
    if (loop.length > best.length) best = loop;
  }
  return simplify(best, step * 0.12);
};

// Ramer–Douglas–Peucker on a closed ring.
const simplify = (ring: Point[], tolerance: number): Point[] => {
  const keep = new Uint8Array(ring.length);
  const walk = (a: number, b: number) => {
    let worst = -1;
    let dist = tolerance;
    const [ax, ay] = ring[a];
    const [bx, by] = ring[b];
    const len = Math.hypot(bx - ax, by - ay) || 1;
    for (let i = a + 1; i < b; i++) {
      const d = Math.abs((bx - ax) * (ay - ring[i][1]) - (ax - ring[i][0]) * (by - ay)) / len;
      if (d > dist) {
        dist = d;
        worst = i;
      }
    }
    if (worst >= 0) {
      keep[worst] = 1;
      walk(a, worst);
      walk(worst, b);
    }
  };
  const half = Math.floor(ring.length / 2);
  keep[0] = 1;
  keep[half] = 1;
  walk(0, half);
  walk(half, ring.length - 1);
  keep[ring.length - 1] = 1;
  return ring.filter((_, i) => keep[i] === 1);
};
