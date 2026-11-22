// Hidden slack retraced under the ink: later arc lengths hold.
import { visible, STEP } from '../lib/line.js';
import { slice, cumulative, densify, bridge, startTangent, endTangent, length } from '../lib/geom.js';

const TAIL = 24;
export const SLACK = { w: 0.2, alpha: 0.002, link: 0 };

function kept(st) {
  return slice(st.pts, st.show[0], st.show[1], cumulative(st.pts));
}

function lastVisible(list, at) {
  for (let i = at - 1; i >= 0; i--) if (visible(list[i])) return list[i];
  throw new Error('c03: slack has no visible stroke before it');
}

// Back and forth along the tail, ending where it began.
function retrace(tail, total) {
  const L = length(tail);
  const n = Math.max(1, Math.ceil(total / (2 * L)));
  const c = cumulative(tail);
  const leg = slice(tail, 1 - total / (2 * n) / L, 1, c);
  const back = [...leg].reverse();
  const pts = [leg[leg.length - 1]];
  for (let k = 0; k < n; k++) pts.push(...back.slice(1), ...leg.slice(1));
  return pts;
}

// The slack's tail, its own gap, and the gap onward.
export function measure(list, at, next) {
  const sub = kept(lastVisible(list, at));
  const d = densify(sub, STEP);
  const P = d[d.length - 1];
  const tan = endTangent(d);
  const c = cumulative(sub);
  const L = c[c.length - 1];
  const tail = L > TAIL ? slice(sub, 1 - TAIL / L, 1, c) : L > 4 ? sub : [[P[0] - tan[0] * 4, P[1] - tan[1] * 4], P];
  const probe = retrace(tail, 2 * length(tail));
  const c0 = length(bridge(P, tan, P, startTangent(densify(probe, STEP)), 6));
  const j = list[next].pts;
  const conn = length(bridge(P, tan, j[0], startTangent(densify(j, STEP)), 6));
  return { tail, c0, conn };
}

export function slack(m, total) {
  if (!(total > 1)) throw new Error(`c03: slack of ${total} cannot absorb the change`);
  return { pts: retrace(m.tail, total), show: [0, 1], app: null, ...SLACK };
}
