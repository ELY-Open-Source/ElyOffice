// Fonts registered once; glyphs rasterised to binary masks.
import fs from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { GlobalFonts, createCanvas } from '@napi-rs/canvas';

export const EM = 240;
const SIZE = Math.round(EM * 1.7);
const LEFT = Math.round(EM * 0.35);
const BASE = Math.round(EM * 1.2);

const local = (p) => fileURLToPath(new URL(p, import.meta.url));
export const FONTS = {
  sans: { file: '/usr/share/fonts/noto-cjk/NotoSansCJK-Light.ttc', face: 2, family: 'ElySans', weight: 300 },
  serif: { file: '/usr/share/fonts/noto-cjk/NotoSerifCJK-Light.ttc', face: 2, family: 'ElySerif', weight: 300 },
  jp: { file: '/usr/share/fonts/noto-cjk/NotoSansCJK-Light.ttc', face: 0, family: 'ElyJapanese', weight: 300 },
  mono: { file: '/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf', family: 'ElyMono', weight: 400 },
  brush: { file: local('../../../update-0.1.16/assets/fonts/LXGWWenKai-Regular.ttf'), family: 'ElyBrush', weight: 400 },
};

const registered = new Set();

function face(f) {
  const out = local(`../../.work/fonts/${f.family}.otf`);
  if (!fs.existsSync(out)) {
    const run = spawnSync('python3', [local('../../tools/fonts.py'), f.file, String(f.face), out], { encoding: 'utf8' });
    if (run.status !== 0) throw new Error(`could not take face ${f.face} of ${f.file}: ${run.stderr}`);
  }
  return out;
}
const canvas = createCanvas(SIZE, SIZE);
const ctx = canvas.getContext('2d');

function use(font, px) {
  const f = FONTS[font];
  if (!f) throw new Error(`unknown font ${font}; known: ${Object.keys(FONTS).join(' ')}`);
  if (!registered.has(font)) {
    if (!fs.existsSync(f.file)) throw new Error(`missing font file ${f.file} for ${font}`);
    GlobalFonts.registerFromPath(f.face === undefined ? f.file : face(f), f.family);
    registered.add(font);
  }
  ctx.font = `${f.weight} ${px}px ${f.family}`;
}

export function advance(font, str) {
  use(font, EM);
  return ctx.measureText(str).width / EM;
}

export function raster(font, ch) {
  use(font, EM);
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, SIZE, SIZE);
  ctx.fillStyle = '#000';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(ch, LEFT, BASE);
  const data = ctx.getImageData(0, 0, SIZE, SIZE).data;
  const bin = new Uint8Array(SIZE * SIZE);
  for (let i = 0; i < bin.length; i++) bin[i] = data[i * 4 + 3] >= 128 ? 1 : 0;
  return { bin, w: SIZE, h: SIZE, ox: LEFT, oy: BASE };
}
