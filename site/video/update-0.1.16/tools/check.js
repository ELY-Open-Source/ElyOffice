// Gate: pure frames; scenes hand off at rest.
import crypto from 'node:crypto';
import scenes from '../scenes/index.js';
import { Film } from '../lib/film.js';
import { REST, rested } from '../lib/flame.js';
import { FPS, LENGTH } from '../lib/time.js';

const fail = [];
const digest = (buf) => crypto.createHash('sha1').update(buf).digest('hex');

for (let i = 0; i < scenes.length; i++) {
  const s = scenes[i];
  if (!(s.end > s.start)) fail.push(`${s.id}: end before start`);
  if (i && scenes[i - 1].end > s.start) fail.push(`${s.id}: overlaps ${scenes[i - 1].id}`);
  for (const [label, t] of [['start', s.start], ['end', s.end - 1 / FPS]]) {
    if ((i === 0 && label === 'start') || (i === scenes.length - 1 && s.end >= LENGTH && label === 'end')) continue;
    const f = rested(s.flame(t));
    for (const k of Object.keys(REST)) {
      if (Math.abs(f[k] - REST[k]) > 1e-6) fail.push(`${s.id} ${label}: ${k} is ${f[k]}, rest is ${REST[k]}`);
    }
  }
}

const last = Math.round(scenes[scenes.length - 1].end * FPS) - 1;
const picks = [...new Set([0, 1, 47, 48, 49, 97, 121, 143, 144, 145, last].filter((n) => n <= last))];
const sequence = new Film();
const alone = new Film();
const hashes = [];
for (const n of picks) {
  if (n > 0) sequence.frame(n - 1);
  const a = digest(sequence.frame(n));
  const b = digest(new Film().frame(n));
  const c = digest(alone.frame(n));
  if (a !== b || b !== c) fail.push(`frame ${n}: differs by render order`);
  hashes.push(`${n} ${b}`);
}
process.stdout.write(hashes.join('\n') + '\n');
if (fail.length) {
  process.stderr.write(fail.join('\n') + '\n');
  process.exit(1);
}
process.stdout.write(`check: ${scenes.length} scene(s), ${picks.length} frames deterministic, handoffs at rest\n`);
