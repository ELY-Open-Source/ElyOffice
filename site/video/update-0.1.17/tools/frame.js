// PNG frames at given seconds: frame.js out-dir seconds...
import fs from 'node:fs';
import path from 'node:path';
import { Film } from '../lib/film.js';
import { filmEnd } from '../lib/world.js';
import { FPS } from '../lib/time.js';

const [dir, ...times] = process.argv.slice(2);
if (!dir || !times.length) throw new Error('usage: frame.js out-dir seconds...');
const frames = times.map((v) => {
  const s = Number(v);
  if (!Number.isFinite(s) || s < 0 || s >= filmEnd()) throw new Error(`not a time inside the film: ${v}`);
  return [v, Math.round(s * FPS)];
});
fs.mkdirSync(dir, { recursive: true });
const film = new Film();
for (const [label, n] of frames) {
  const t = performance.now();
  film.frame(n);
  const file = path.join(dir, `t${Number(label).toFixed(2).padStart(6, '0')}.png`);
  fs.writeFileSync(file, film.canvas.toBuffer('image/png'));
  process.stdout.write(`${file} ${(performance.now() - t).toFixed(0)} ms ${film.points} points\n`);
}
