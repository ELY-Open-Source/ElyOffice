// The cursor's free end: a spring trailing the pointer.
import { FPS } from './time.js';

const WINDOW = 36;
const K = 170;
const DAMP = 13;

export function tether(s, k, f) {
  let p = null;
  let v = [0, 0];
  for (let g = f - WINDOW; g <= f; g++) {
    const c = s.cursor(Math.max(g, 0) / FPS, k);
    if (!p) {
      if (c) p = [c.x, c.y];
      continue;
    }
    const target = c ? [c.x, c.y] : p;
    const dt = 1 / FPS;
    v = [v[0] + (K * (target[0] - p[0]) - DAMP * v[0]) * dt, v[1] + (K * (target[1] - p[1]) - DAMP * v[1]) * dt];
    p = [p[0] + v[0] * dt, p[1] + v[1] * dt];
  }
  return p;
}
