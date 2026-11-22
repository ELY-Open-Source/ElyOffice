// Quiet routes between drawings, with a curl to spare length.
import { kit } from '../lib/kit/index.js';

const STYLE = { w: 0.45, alpha: 0.18 };

export function route(a, b, { bow = 0.12, extra = 0 } = {}) {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const d = Math.hypot(dx, dy) || 1;
  const side = dx >= 0 ? -1 : 1;
  const c = [(a[0] + b[0]) / 2 + (side * -dy / d) * bow * d, (a[1] + b[1]) / 2 + (side * dx / d) * bow * d];
  const n = Math.max(4, Math.ceil(d / 12));
  const pts = [];
  for (let i = 0; i <= n; i++) {
    const u = i / n;
    pts.push([(1 - u) ** 2 * a[0] + 2 * u * (1 - u) * c[0] + u * u * b[0], (1 - u) ** 2 * a[1] + 2 * u * (1 - u) * c[1] + u * u * b[1]]);
  }
  if (extra > 0) pts.push(...curl(pts, extra));
  return { ...kit.stroke(pts, STYLE), link: 0, route: true };
}

function curl(pts, extra) {
  const b = pts.at(-1);
  const p = pts.at(-2);
  const len = Math.hypot(b[0] - p[0], b[1] - p[1]) || 1;
  const dir = [(b[0] - p[0]) / len, (b[1] - p[1]) / len];
  const nrm = [-dir[1], dir[0]];
  const rc = Math.min(34, Math.max(10, extra / (2 * Math.PI)));
  const turns = Math.ceil(extra / (2 * Math.PI * rc));
  const c = [b[0] + nrm[0] * rc, b[1] + nrm[1] * rc];
  const a0 = Math.atan2(b[1] - c[1], b[0] - c[0]);
  const out = [];
  const steps = Math.ceil(turns * 2 * Math.PI * rc / 6);
  for (let i = 1; i <= steps; i++) {
    const a = a0 + (i / steps) * turns * 2 * Math.PI;
    out.push([c[0] + rc * Math.cos(a), c[1] + rc * Math.sin(a)]);
  }
  return out;
}
