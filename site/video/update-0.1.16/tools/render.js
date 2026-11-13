// Parallel render by frame ranges; joins segments, adds audio.
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { FPS } from '../lib/time.js';
import { filmEnd } from '../lib/film.js';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.dirname(here);
const FLAGS = ['from', 'to', 'jobs', 'out', 'audio', 'contact', 'wrap'];
const argv = process.argv.slice(2);
if (argv.length % 2) throw new Error(`flags take one value each: ${argv.join(' ')}`);
const args = {};
for (let i = 0; i < argv.length; i += 2) {
  const key = argv[i].replace(/^--/, '');
  if (!argv[i].startsWith('--') || !FLAGS.includes(key)) throw new Error(`unknown flag ${argv[i]}; known: ${FLAGS.join(' ')}`);
  args[key] = argv[i + 1];
}
const number = (v, name) => {
  const n = Number(v);
  if (!Number.isFinite(n) || n < 0) throw new Error(`--${name} needs a number, got ${v}`);
  return n;
};

const from = Math.round(number(args.from ?? 0, 'from') * FPS);
const to = Math.round(number(args.to ?? filmEnd(), 'to') * FPS);
const jobs = number(args.jobs ?? os.cpus().length, 'jobs');
const out = path.resolve(args.out ?? path.join(root, 'renders', 'film.mp4'));
const wrap = args.wrap?.split('-').map((v) => Math.round(number(v, 'wrap') * FPS));
if (!(to > from) || !Number.isInteger(jobs) || jobs < 1) throw new Error('need --from < --to and a whole --jobs of 1 or more');
if (wrap && !(wrap.length === 2 && wrap[1] > wrap[0])) throw new Error(`--wrap needs a-b seconds, got ${args.wrap}`);
if (!wrap && to > Math.round(filmEnd() * FPS)) throw new Error(`scenes end at ${filmEnd()} s; --to ${to / FPS} is past them`);
if (args.audio && !fs.existsSync(args.audio)) throw new Error(`no audio at ${args.audio}`);

const work = path.join(root, '.work', path.basename(out, '.mp4'));
fs.rmSync(work, { recursive: true, force: true });
fs.mkdirSync(work, { recursive: true });
fs.mkdirSync(path.dirname(out), { recursive: true });

const children = new Set();
async function run(cmd, list, log) {
  const p = spawn(cmd, list, { stdio: ['ignore', 'pipe', 'inherit'] });
  children.add(p);
  let text = '';
  p.stdout.on('data', (d) => (text += d));
  const [code] = await once(p, 'close');
  children.delete(p);
  if (code !== 0) throw new Error(`${cmd} ${list.join(' ')} exited ${code}`);
  if (log) fs.appendFileSync(log, text);
  return text;
}

const started = performance.now();
const total = to - from;
const size = Math.ceil(total / jobs);
const parts = [];
for (let a = from, k = 0; a < to; a += size, k++) parts.push([a, Math.min(to, a + size), path.join(work, `seg${String(k).padStart(3, '0')}.mp4`)]);
const log = path.join(work, 'frames.log');
try {
  await Promise.all(parts.map(([a, b, seg]) => run('node', [path.join(here, 'worker.js'), String(a), String(b), seg,
    ...(wrap ? wrap.map(String) : [])], log)));
} catch (err) {
  for (const p of children) p.kill('SIGTERM');
  throw err;
}
const drawn = performance.now();

const list = path.join(work, 'segments.txt');
fs.writeFileSync(list, parts.map(([, , seg]) => `file '${seg}'`).join('\n') + '\n');
const audio = args.audio
  ? ['-ss', String(from / FPS), '-t', String(total / FPS), '-i', path.resolve(args.audio), '-map', '0:v', '-map', '1:a', '-c:a', 'aac', '-b:a', '320k']
  : [];
await run('ffmpeg', ['-hide_banner', '-loglevel', 'error', '-y', '-f', 'concat', '-safe', '0', '-i', list,
  ...audio, '-c:v', 'copy', '-movflags', '+faststart', out]);

if (args.contact) {
  const cols = 12;
  const rows = Math.ceil(total / 2 / cols);
  await run('ffmpeg', ['-hide_banner', '-loglevel', 'error', '-y', '-i', out, '-vf',
    `select='not(mod(n\\,2))',scale=320:180,tile=${cols}x${rows}:padding=4:margin=4`, '-frames:v', '1', '-update', '1',
    path.resolve(args.contact)]);
}

const secs = (performance.now() - started) / 1000;
const summary = `${total} frames, ${parts.length} jobs: ${secs.toFixed(1)} s total, ${((drawn - started) / 1000).toFixed(1)} s drawing, ${(total / secs).toFixed(1)} frames/s -> ${out}`;
fs.appendFileSync(path.join(work, 'summary.log'), summary + '\n');
process.stdout.write(summary + '\n');
