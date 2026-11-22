// Pointer path, sticky snaps and camera for the guides scene.
import { kit } from '../lib/kit/index.js';
import { A, B, CENTRE, D } from './c07-stage.js';

const { ease, span, spring, keys } = kit;
const FPS = 60;
const HALF = D.h / 2;
const TH = 14;

export const HIT = { grab: 59.5, edge: 60, centres: 61, slide: 62, gaps: 64, alt: 65, drop: 66.75, keyUp: 67, leave: 67.25 };
export const GAPS = { x: (A.x + A.w + B.x - B.r) / 2, y: 412 };

const P0 = [1060, 640];
const P1 = [889, 611];
const P2 = [1646, 657];
const P3 = [1233, 551];
const P4 = [GAPS.x + 3, GAPS.y];
const P5 = [1385, 758];
const VC = [120, 420];

// Bezier legs: start, end, four points, ease.
const LEGS = [
  [59.5, 59.625, [P0, P0, [1069, 646], [1069, 646]], 'out'],
  [59.625, 60.125, [[1069, 646], [1010, 652], [930, 618], P1], 'inOut'],
  [60.125, 60.4375, [P1, P1, [886, 606], [886, 606]], 'inOut'],
  [60.4375, 61.1333, [[886, 606], [1010, 720], [1540, 700], P2], 'inOut'],
  [61.1333, 61.375, [P2, P2, [1648, 652], [1648, 652]], 'inOut'],
  [61.375, 62.1333, [[1648, 652], [1600, 545], [1340, 540], P3], 'inOut'],
  [62.1333, 62.875, [P3, P3, [1231, 554], [1231, 554]], 'sine'],
  [62.875, 64.3, [[1231, 554], [1262, 470], [1340, 416], P4], 'inOut'],
  [64.3, 65.25, [P4, P4, [P4[0] - 1, P4[1] + 1], [P4[0] - 1, P4[1] + 1]], 'sine'],
  [65.25, 66, [[P4[0] - 1, P4[1] + 1], [P4[0] - 1, P4[1] + 1], [CENTRE[0] - VC[0] / 4, CENTRE[1] - VC[1] / 4], CENTRE], 'linear'],
  [66, 66.75, [CENTRE, [CENTRE[0] + VC[0] / 4, CENTRE[1] + VC[1] / 4], P5, P5], 'linear'],
];

function bez([p0, p1, p2, p3], u) {
  const v = 1 - u;
  const a = v * v * v;
  const b = 3 * v * v * u;
  const c = 3 * v * u * u;
  const d = u * u * u;
  return [a * p0[0] + b * p1[0] + c * p2[0] + d * p3[0], a * p0[1] + b * p1[1] + c * p2[1] + d * p3[1]];
}

// Where the hand wants the shape's centre.
export function free(t) {
  let last = P0;
  for (const [a, b, pts, how] of LEGS) {
    if (t < a) return last;
    if (t <= b) return bez(pts, ease[how](span(t, a, b)));
    last = pts[3];
  }
  return last;
}

const frame = (t) => Math.round(t * FPS) / FPS;

function breakAt(ta, to) {
  for (let t = ta + 0.1; t < 68; t += 1 / FPS) {
    const f = free(frame(t));
    if (to.some((v, i) => v !== null && Math.abs(f[i] - v) > TH)) return frame(t);
  }
  throw new Error('c07: a snap never breaks');
}

function snap(hit, to, until) {
  const ta = hit - 2 / FPS;
  const tb = until ?? breakAt(ta, to);
  return { hit, ta, tb, to, fa: free(ta), fb: free(tb) };
}

export const SNAPS = [
  snap(HIT.edge, [A.x + D.w / 2, null]),
  snap(HIT.centres, [B.x, null]),
  snap(HIT.slide, [CENTRE[0], CENTRE[1]]),
  snap(HIT.gaps, [GAPS.x, null], HIT.alt),
];

// The shape's centre: the hand, pulled onto guides.
export function centre(t) {
  const f = free(t);
  return [0, 1].map((i) => {
    let v = f[i];
    for (const s of SNAPS) {
      if (s.to[i] === null || t < s.ta) continue;
      if (t < s.tb) return s.to[i] + spring(t - s.ta, s.fa[i] - s.to[i], 47, 15);
      v += spring(t - s.tb, s.to[i] - s.fb[i], 30, 14);
    }
    return v;
  });
}

export const grip = (c) => [c[0], c[1] + HALF];

export const MINI = { from: P0, to: P5 };

// Leaves under the window, toward the next panel's side.
function exitPoint(next) {
  const [x] = next ?? [1920, 540];
  return [x < 960 ? 60 : 1860, 1010];
}

export const EXIT_DIR = [0, 1];

export function cursorAt(t, k) {
  if (t < HIT.grab) return null;
  const f = free(t);
  const hand = grip(f);
  if (t <= HIT.leave) return { x: hand[0], y: hand[1] };
  const x = exitPoint(k.next);
  const g = grip(P5);
  const p = bez([g, [g[0], g[1] + 230], [x[0] + 640, x[1]], x], ease.inOut(span(t, HIT.leave, 68)));
  return { x: p[0], y: p[1] };
}

const FAR = [1090, 520, 1780];
const NEAR = [1100, 522, 1660];
const REST = [960, 540, 1920];

export function cameraAt(t) {
  const v = keys(t, [[59.5, FAR], [63.5, NEAR, 'sine'], [64.125, NEAR], [64.75, REST, 'inOut']]);
  return { x: v[0], y: v[1], w: v[2], r: 0 };
}

