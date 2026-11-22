// Gate: pure frames, pen contract, windows, cues, touch, lift, rules.
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Film } from '../lib/film.js';
import { entries } from '../lib/world.js';
import { join } from '../lib/line.js';
import { lineAt } from '../lib/path.js';
import { FPS, LENGTH, TOUCH, LIFT } from '../lib/time.js';
import { rules } from './rules.js';

const here = fileURLToPath(import.meta.url);
const root = path.dirname(path.dirname(here));
const digest = (buf) => crypto.createHash('sha1').update(buf).digest('hex');
const hashes = (list) => {
  const film = new Film();
  return list.map((n) => `${n} ${digest(film.frame(n))}`);
};

if (process.argv[2] === '--hash') {
  process.stdout.write(hashes(process.argv[3].split(',').map(Number)).join('\n'));
  process.exit(0);
}

const fail = [];
const say = (m) => fail.push(m);

const first = entries[0].s;
const last = entries[entries.length - 1].s;
if (first.start !== 0) say(`${first.id} starts at ${first.start}, not 0`);
if (last.end !== LENGTH) say(`${last.id} ends at ${last.end}, not ${LENGTH}`);
for (let i = 1; i < entries.length; i++) {
  if (entries[i].s.start !== entries[i - 1].s.end) say(`${entries[i].s.id} starts at ${entries[i].s.start}, ${entries[i - 1].s.id} ends at ${entries[i - 1].s.end}`);
}

for (const e of entries) {
  const { s, k } = e;
  let prev = -Infinity;
  let top = 0;
  let count = null;
  for (let f = e.f0; f <= e.f1; f++) {
    const t = f / FPS;
    const p = s.pen(t, k);
    if (p < prev - 1e-9) say(`${s.id}: pen falls at ${t.toFixed(3)} s (${prev.toFixed(1)} -> ${p.toFixed(1)})`);
    prev = p;
    const early = e.i === 0 ? TOUCH : s.start + 0.75;
    if (t <= early + 1e-9 && p > 1e-6) say(`${s.id}: pen is ${p.toFixed(1)} at ${t.toFixed(3)} s, before ${early} s`);
    if (f % 2 && f !== e.f1) continue;
    const list = s.strokes(t, k);
    if (count === null) count = list.length;
    else if (list.length !== count) say(`${s.id}: ${list.length} strokes at ${t.toFixed(3)} s, ${count} before`);
    for (const st of list) {
      if (!st.pts.every((q) => Number.isFinite(q[0]) && Number.isFinite(q[1]))) say(`${s.id}: a stroke has a bad point at ${t.toFixed(3)} s`);
      if (!(st.show[0] >= 0 && st.show[1] <= 1 && st.show[0] <= st.show[1])) say(`${s.id}: show ${st.show} at ${t.toFixed(3)} s`);
    }
    top = Math.max(top, join(list, s.id).len);
    if (p > top + 1) say(`${s.id}: pen ${p.toFixed(1)} past the longest path ${top.toFixed(1)} at ${t.toFixed(3)} s`);
  }
  if (fail.length > 40) break;
}

const touch = Math.round(TOUCH * FPS);
const count = (n) => lineAt(n).blocks.reduce((a, b) => a + b.x.length, 0);
if (count(touch) > 1) say(`the line has ${count(touch)} points at ${TOUCH} s; the pen lands then`);
if (count(touch + 1) < 1) say(`nothing is drawn just after ${TOUCH} s`);
const lift = entries[entries.length - 1];
const held = lift.s.pen(LIFT, lift.k);
for (let f = Math.round(LIFT * FPS); f <= lift.f1; f++) {
  if (lift.s.pen(f / FPS, lift.k) !== held) say(`${lift.s.id}: pen moves after the lift, at ${(f / FPS).toFixed(3)} s`);
  if (lift.s.cursor?.(f / FPS, lift.k)) say(`${lift.s.id}: a cursor after the lift, at ${(f / FPS).toFixed(3)} s`);
}

for (const e of entries) {
  const file = path.join(root, 'scenes', `${e.s.id}.cues.json`);
  if (!fs.existsSync(file)) continue;
  const { cues } = JSON.parse(fs.readFileSync(file, 'utf8'));
  if (!Array.isArray(cues)) say(`${e.s.id}.cues.json has no cues list`);
  for (const c of cues ?? []) {
    if (!(c.t >= e.s.start && c.t < e.s.end)) say(`${e.s.id}: cue ${c.sfx} at ${c.t} is outside ${e.s.start}-${e.s.end}`);
    if (Math.abs(c.t * 16 - Math.round(c.t * 16)) > 1e-6) say(`${e.s.id}: cue ${c.sfx} at ${c.t} is off the eighth-beat grid`);
    if (typeof c.sfx !== 'string') say(`${e.s.id}: cue at ${c.t} has no sfx`);
  }
}

const picks = [0, 59, 60, 61, 300, 360, 509, 510, 600, 3600, 6400, 7140, Math.round(last.end * FPS) - 1];
const up = hashes(picks);
const down = execFileSync('node', [here, '--hash', [...picks].reverse().join(',')]).toString().trim().split('\n').reverse();
up.forEach((h, i) => {
  if (h !== down[i]) say(`frame ${picks[i]} differs by render order`);
});

fail.push(...rules(root));
if (fail.length) {
  process.stderr.write(fail.join('\n') + '\n');
  process.exit(1);
}
process.stdout.write(`check: ${entries.length} scenes, ${picks.length} frames pure, pen and cues hold, rules hold\n`);
