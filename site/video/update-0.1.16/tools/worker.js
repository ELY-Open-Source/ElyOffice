// Renders frames [a, b) into one H.264 segment.
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { Film } from '../lib/film.js';
import { W, H, FPS } from '../lib/time.js';

const [a, b, out, wrapA, wrapB] = process.argv.slice(2).map((v, i) => (i === 2 ? v : Number(v)));
if (!Number.isInteger(a) || !(b > a) || !out) throw new Error('usage: worker.js first end out.mp4 [wrapFirst wrapEnd]');
const wrapped = Number.isInteger(wrapA);
if (wrapped && !(wrapB > wrapA && wrapA >= 0)) throw new Error(`bad wrap ${wrapA}-${wrapB}`);
const loop = wrapB - wrapA;

const ff = spawn('ffmpeg', [
  '-hide_banner', '-loglevel', 'error', '-y',
  '-f', 'rawvideo', '-pix_fmt', 'rgba', '-s', `${W}x${H}`, '-r', String(FPS), '-i', '-',
  '-vf', 'scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int',
  '-c:v', 'libx264', '-preset', 'medium', '-crf', '14', '-pix_fmt', 'yuv420p', '-threads', '2',
  '-colorspace', 'bt709', '-color_primaries', 'bt709', '-color_trc', 'bt709', '-g', '48',
  out,
], { stdio: ['pipe', 'inherit', 'inherit'] });
const exited = once(ff, 'exit');

const film = new Film();
const started = performance.now();
for (let n = a; n < b; n++) {
  const m = wrapped ? wrapA + ((((n - wrapA) % loop) + loop) % loop) : n;
  const buf = film.frame(m);
  if (!ff.stdin.write(buf)) await once(ff.stdin, 'drain');
}
ff.stdin.end();
const [code] = await exited;
if (code !== 0) throw new Error(`ffmpeg exited ${code} for frames ${a}-${b}`);
const ms = performance.now() - started;
process.stdout.write(`${a}-${b} ${(ms / (b - a)).toFixed(1)} ms/frame\n`);
