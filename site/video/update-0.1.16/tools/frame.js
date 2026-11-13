// Renders single frames to PNG: frame.js out-dir n...
import fs from 'node:fs';
import path from 'node:path';
import { Film } from '../lib/film.js';

const [dir, ...frames] = process.argv.slice(2);
if (!dir || !frames.length) throw new Error('usage: frame.js out-dir n...');
const numbers = frames.map((v) => {
  const n = Number(v);
  if (!Number.isInteger(n) || n < 0) throw new Error(`not a frame number: ${v}`);
  return n;
});
fs.mkdirSync(dir, { recursive: true });
const film = new Film();
for (const n of numbers) {
  const t = performance.now();
  film.frame(n);
  const file = path.join(dir, `f${String(n).padStart(4, '0')}.png`);
  fs.writeFileSync(file, film.canvas.toBuffer('image/png'));
  process.stdout.write(`${file} ${(performance.now() - t).toFixed(0)} ms\n`);
}
