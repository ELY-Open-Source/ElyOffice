// s9's flashback: one drawing from each scene's moments.
import s1 from './s1.js';
import s2 from './s2.js';
import s3 from './s3.js';
import s4 from './s4.js';
import s5 from './s5.js';
import s6 from './s6.js';
import s7 from './s7.js';
import s8 from './s8.js';

export { s8 };

export const FLASH = [82.5, 84.5];

const MOMENTS = [
  [s1, 2.08], [s1, 5.25], [s1, 6.0],
  [s2, 10], [s2, 13], [s2, 16.0],
  [s3, 22], [s3, 24.0], [s3, 27.5],
  [s4, 34], [s4, 36.0], [s4, 40.0],
  [s5, 45], [s5, 48.0], [s5, 51],
  [s6, 56.0], [s6, 59], [s6, 61],
  [s7, 66.0], [s7, 70.0], [s7, 72.5],
  [s8, 76.0], [s8, 78.5], [s8, 80.0],
].map(([scene, t]) => [scene, Math.round(t * 12) / 12]);

export function moment(t) {
  if (t < FLASH[0] || t >= FLASH[1]) return null;
  const k = Math.min(MOMENTS.length - 1, Math.floor((t - FLASH[0]) * 12 + 1e-6));
  const [scene, at] = MOMENTS[k];
  return { scene, at };
}
