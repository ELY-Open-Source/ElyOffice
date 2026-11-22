// Renders frames [a, b) into one H.264 segment.
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { Film } from '../lib/film.js';
import { W, H, FPS } from '../lib/time.js';

const [a, b, out] = process.argv.slice(2).map((v, i) => (i === 2 ? v : Number(v)));
if (!Number.isInteger(a) || !(b > a) || !out) throw new Error('usage: worker.js first end out.mp4');

const ff = spawn('ffmpeg', [
  '-hide_banner', '-loglevel', 'error', '-y',
  '-f', 'rawvideo', '-pix_fmt', 'rgba', '-s', `${W}x${H}`, '-r', String(FPS), '-i', '-',
  '-vf', 'scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int,setparams=color_primaries=bt709:color_trc=bt709:colorspace=bt709',
  '-c:v', 'libx264', '-preset', 'medium', '-crf', '14', '-pix_fmt', 'yuv420p', '-threads', '2',
  '-colorspace', 'bt709', '-color_primaries', 'bt709', '-color_trc', 'bt709', '-g', '120',
  out,
], { stdio: ['pipe', 'inherit', 'inherit'] });
const exited = once(ff, 'exit');

const film = new Film();
const started = performance.now();
let worst = 0;
for (let n = a; n < b; n++) {
  const t = performance.now();
  const buf = film.frame(n);
  worst = Math.max(worst, performance.now() - t);
  if (!ff.stdin.write(buf)) await once(ff.stdin, 'drain');
}
ff.stdin.end();
const [code] = await exited;
if (code !== 0) throw new Error(`ffmpeg exited ${code} for frames ${a}-${b}`);
const ms = performance.now() - started;
process.stdout.write(`${a}-${b} ${(ms / (b - a)).toFixed(1)} ms/frame, worst ${worst.toFixed(0)} ms\n`);
