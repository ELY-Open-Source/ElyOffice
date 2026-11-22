// SVG elements and path data flattened to polylines.

function arcTo(out, x0, y0, rx, ry, rot, large, sweep, x, y) {
  const phi = (rot * Math.PI) / 180;
  const c = Math.cos(phi);
  const s = Math.sin(phi);
  const dx = (x0 - x) / 2;
  const dy = (y0 - y) / 2;
  const x1 = c * dx + s * dy;
  const y1 = -s * dx + c * dy;
  let k = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
  if (k > 1) {
    rx *= Math.sqrt(k);
    ry *= Math.sqrt(k);
  }
  const num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
  k = Math.sqrt(Math.max(0, num / (rx * rx * y1 * y1 + ry * ry * x1 * x1))) * (large === sweep ? -1 : 1);
  const cx1 = (k * rx * y1) / ry;
  const cy1 = (-k * ry * x1) / rx;
  const cx = c * cx1 - s * cy1 + (x0 + x) / 2;
  const cy = s * cx1 + c * cy1 + (y0 + y) / 2;
  const ang = (ux, uy, vx, vy) => Math.atan2(ux * vy - uy * vx, ux * vx + uy * vy);
  const t0 = ang(1, 0, (x1 - cx1) / rx, (y1 - cy1) / ry);
  let dt = ang((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry);
  if (!sweep && dt > 0) dt -= 2 * Math.PI;
  if (sweep && dt < 0) dt += 2 * Math.PI;
  const n = Math.max(4, Math.ceil(Math.abs(dt) * Math.max(rx, ry) / 2));
  for (let i = 1; i <= n; i++) {
    const t = t0 + (dt * i) / n;
    out.push([cx + rx * Math.cos(t) * c - ry * Math.sin(t) * s, cy + rx * Math.cos(t) * s + ry * Math.sin(t) * c]);
  }
}

export function pathData(d) {
  const tok = d.match(/[a-zA-Z]|-?\d*\.?\d+(?:e-?\d+)?/g);
  const subs = [];
  let cur = null;
  let x = 0;
  let y = 0;
  let cmd = '';
  let i = 0;
  const num = () => Number(tok[i++]);
  while (i < tok.length) {
    if (/[a-zA-Z]/.test(tok[i])) cmd = tok[i++];
    const rel = cmd === cmd.toLowerCase();
    const C = cmd.toUpperCase();
    const ox = rel ? x : 0;
    const oy = rel ? y : 0;
    if (C === 'M') {
      x = ox + num();
      y = oy + num();
      cur = { pts: [[x, y]], closed: false };
      subs.push(cur);
      cmd = rel ? 'l' : 'L';
    } else if (C === 'L') {
      x = ox + num();
      y = oy + num();
      cur.pts.push([x, y]);
    } else if (C === 'H') {
      x = ox + num();
      cur.pts.push([x, y]);
    } else if (C === 'V') {
      y = oy + num();
      cur.pts.push([x, y]);
    } else if (C === 'C') {
      const [a, b, c2, d2, e, f] = [ox + num(), oy + num(), ox + num(), oy + num(), ox + num(), oy + num()];
      for (let k = 1; k <= 16; k++) {
        const u = k / 16;
        const v = 1 - u;
        cur.pts.push([v * v * v * x + 3 * v * v * u * a + 3 * v * u * u * c2 + u * u * u * e,
          v * v * v * y + 3 * v * v * u * b + 3 * v * u * u * d2 + u * u * u * f]);
      }
      x = e;
      y = f;
    } else if (C === 'A') {
      const [rx, ry, rot, large, sweep] = [num(), num(), num(), num(), num()];
      const ex = ox + num();
      const ey = oy + num();
      arcTo(cur.pts, x, y, rx, ry, rot, large, sweep, ex, ey);
      x = ex;
      y = ey;
    } else if (C === 'Z') {
      cur.closed = true;
      [x, y] = cur.pts[0];
    } else throw new Error(`svg path command ${cmd} is not handled`);
  }
  return subs;
}

const attr = (tag, name) => {
  const m = tag.match(new RegExp(`\\s${name}="([^"]*)"`));
  return m ? m[1] : null;
};

export function elements(svg) {
  const body = svg.replace(/<defs>.*?<\/defs>/s, '').replace(/<g [^>]*filter="[^"]*"[^>]*>.*?<\/g>/gs, '');
  const out = [];
  for (const m of body.matchAll(/<(path|rect|circle|polyline)\b[^>]*>/g)) {
    const tag = m[0];
    const n = (k) => Number(attr(tag, k) ?? 0);
    if (m[1] === 'path') out.push({ kind: 'path', subs: pathData(attr(tag, 'd')) });
    else if (m[1] === 'rect') out.push({ kind: 'rect', x: n('x'), y: n('y'), w: n('width'), h: n('height'), r: n('rx') });
    else if (m[1] === 'circle') out.push({ kind: 'circle', x: n('cx'), y: n('cy'), r: n('r') });
    else out.push({ kind: 'polyline', pts: attr(tag, 'points').trim().split(/\s+/).map((p) => p.split(',').map(Number)) });
  }
  return out;
}
