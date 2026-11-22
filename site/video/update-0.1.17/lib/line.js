// Joins a scene's visible strokes into one polyline.
import { slice, cumulative, densify, bridge, startTangent, endTangent, length } from './geom.js';

export const STEP = 10;
export const LINK = { w: 0.35, alpha: 0.45 };

export function visible(s) {
  return s.show[1] - s.show[0] > 1e-6 && (s.alpha ?? 1) > 0.001 && s.pts.length > 1 && length(s.pts) > 0.5;
}

export function check(stroke, where) {
  if (!stroke || !Array.isArray(stroke.pts) || !Array.isArray(stroke.show) || stroke.show.length !== 2) {
    throw new Error(`${where}: a stroke needs pts and show [a, b]`);
  }
}

export function join(strokes, where = 'scene') {
  const x = [];
  const y = [];
  const s = [];
  const w = [];
  const a = [];
  const app = [];
  const link = [];
  let prev = null;
  let prevTan = null;
  let run = 0;
  const push = (p, ww, aa, ap, lk) => {
    if (x.length) run += Math.hypot(p[0] - x[x.length - 1], p[1] - y[y.length - 1]);
    x.push(p[0]);
    y.push(p[1]);
    s.push(run);
    w.push(ww);
    a.push(aa);
    app.push(ap);
    link.push(lk);
  };
  let first = null;
  for (const st of strokes) {
    check(st, where);
    if (!visible(st)) continue;
    const sub = densify(slice(st.pts, st.show[0], st.show[1], cumulative(st.pts)), STEP);
    const t0 = startTangent(sub);
    const ww = st.w ?? 1;
    const aa = st.alpha ?? 1;
    const ap = st.app ?? null;
    if (prev) {
      const c = bridge(prev, prevTan, sub[0], t0, 6);
      for (let i = 1; i < c.length - 1; i++) push(c[i], LINK.w, LINK.alpha * (st.link ?? 1), ap, 1);
    } else first = { at: sub[0], tan: t0 };
    for (let i = 0; i < sub.length; i++) push(sub[i], ww, aa, ap, 0);
    prev = sub[sub.length - 1];
    prevTan = endTangent(sub);
  }
  return { x, y, s, w, a, app, link, len: run, first, last: prev ? { at: prev, tan: prevTan } : null };
}

export function marks(strokes) {
  const out = [];
  let prev = null;
  let prevTan = null;
  let run = 0;
  for (const st of strokes) {
    if (!visible(st)) {
      out.push({ start: run, end: run });
      continue;
    }
    const sub = slice(st.pts, st.show[0], st.show[1], cumulative(st.pts));
    if (prev) run += length(bridge(prev, prevTan, sub[0], startTangent(densify(sub, STEP)), 6));
    const start = run;
    run += length(sub);
    out.push({ start, end: run });
    prev = sub[sub.length - 1];
    prevTan = endTangent(densify(sub, STEP));
  }
  return { list: out, total: run };
}
