// Writes .work/pen.json: the pen per frame, for the score.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { lineAt } from '../lib/path.js';
import { cameraAt, matrix } from '../lib/camera.js';
import { penAt, speedAt, pressure } from '../lib/pen.js';
import { filmEnd } from '../lib/world.js';
import { FPS, TOUCH, LIFT } from '../lib/time.js';

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const out = path.join(root, '.work', 'pen.json');
const frames = [];
let prev = null;
const last = Math.round(filmEnd() * FPS);
for (let n = 0; n < last; n++) {
  const t = n / FPS;
  const { tip, entry } = lineAt(n);
  const m = matrix(cameraAt(n));
  const at = tip ? [m.a * tip.at[0] + m.c * tip.at[1] + m.e, m.b * tip.at[0] + m.d * tip.at[1] + m.f] : null;
  const speed = at && prev ? Math.hypot(at[0] - prev[0], at[1] - prev[1]) * FPS : 0;
  const grow = penAt(entry, n) - penAt(entry, n - 1);
  const linking = entry.i > 0 && t < entry.s.start + 0.75;
  const down = t >= TOUCH && t < LIFT && !!at;
  frames.push({
    t: Number(t.toFixed(4)),
    x: at ? Number(at[0].toFixed(1)) : null,
    y: at ? Number(at[1].toFixed(1)) : null,
    speed: Number(speed.toFixed(1)),
    pressure: Number(pressure(speedAt(entry, t)).toFixed(3)),
    drawing: down && (grow > 1e-6 || linking),
    scene: entry.s.id,
  });
  prev = at;
}
fs.mkdirSync(path.dirname(out), { recursive: true });
fs.writeFileSync(out, JSON.stringify({ fps: FPS, frames }));
process.stdout.write(`${frames.length} frames -> ${out}\n`);
