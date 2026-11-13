// Paper tooth, and the light that falls on it.
import { createCanvas } from '@napi-rs/canvas';
import { noise2, rnd } from './rand.js';
import { W, H } from './time.js';

const BASE = [238, 226, 203];
let made = null;

function relief() {
  const h = new Float32Array(W * H);
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const i = y * W + x;
      h[i] = 0.5 * noise2(x / 7.5, y / 6.2, 17) + 0.32 * noise2(x / 2.8, y / 3.1, 23)
        + 0.18 * (rnd(29, x, y) - 0.5);
    }
  }
  for (let f = 0; f < 2600; f++) {
    let x = rnd(31, f) * W;
    let y = rnd(37, f) * H;
    let a = rnd(41, f) * Math.PI * 2;
    const len = 10 + 40 * rnd(43, f);
    const lift = (rnd(47, f) - 0.35) * 0.6;
    for (let s = 0; s < len; s++) {
      a += (rnd(53, f, s) - 0.5) * 0.25;
      x += Math.cos(a);
      y += Math.sin(a);
      const xi = x | 0;
      const yi = y | 0;
      if (xi >= 0 && yi >= 0 && xi < W && yi < H) h[yi * W + xi] += lift;
    }
  }
  return h;
}

function make() {
  const h = relief();
  const nx = new Float32Array(W * H);
  const ny = new Float32Array(W * H);
  const canvas = createCanvas(W, H);
  const ctx = canvas.getContext('2d');
  const img = ctx.createImageData(W, H);
  const a = img.data;
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const i = y * W + x;
      const l = x > 0 ? h[i - 1] : h[i];
      const r = x < W - 1 ? h[i + 1] : h[i];
      const u = y > 0 ? h[i - W] : h[i];
      const d = y < H - 1 ? h[i + W] : h[i];
      nx[i] = l - r;
      ny[i] = u - d;
      const blot = 0.035 * noise2(x / 260, y / 230, 61) + 0.02 * noise2(x / 70, y / 90, 67);
      const tone = 1 + blot + 0.035 * h[i];
      a[i * 4] = BASE[0] * tone;
      a[i * 4 + 1] = BASE[1] * tone;
      a[i * 4 + 2] = BASE[2] * (tone - 0.01 * rnd(71, x >> 3, y >> 3));
      a[i * 4 + 3] = 255;
    }
  }
  ctx.putImageData(img, 0, 0);
  return { canvas, nx, ny, h };
}

export function paper() {
  made ??= make();
  return made;
}

const Q = 7;
const N = 2048;
const NEAR = [1.18, 1.0, 0.74];
const MID = [1.0, 0.56, 0.24];
const FAR = [0.5, 0.17, 0.05];
const WHITE = [1, 1, 1];

function ramp(q) {
  const d = Math.sqrt(q);
  const pool = Math.exp(-1.6 * q ** 1.5);
  const core = 0.28 / (1 + q * 40);
  const u = Math.min(1, d / 1.6);
  const c = u < 0.5
    ? NEAR.map((v, k) => v + (MID[k] - v) * u * 2)
    : MID.map((v, k) => v + (FAR[k] - v) * (u - 0.5) * 2);
  return c.map((v) => v * (pool + core));
}

const LUT = new Float32Array(N * 3);
for (let i = 0; i < N; i++) LUT.set(ramp((i / N) * Q), i * 3);

const TONE = new Uint8ClampedArray(2048);
for (let i = 0; i < 2048; i++) {
  const v = i / 512;
  TONE[i] = 255 * (v < 0.8 ? v : 0.8 + 0.2 * (1 - Math.exp(-(v - 0.8) * 2.2)));
}

export function illuminate(src, out, lights, ambient = 0, grain = 0.45) {
  const { nx, ny } = paper();
  const amb = Array.isArray(ambient) ? ambient : [ambient, ambient * 0.93, ambient * 0.82];
  const L = lights.filter((l) => l.i > 0.001 && l.r > 1);
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const i = y * W + x;
      let r = amb[0];
      let g = amb[1];
      let b = amb[2];
      for (let k = 0; k < L.length; k++) {
        const l = L[k];
        const dx = x - l.x;
        const dy = y - l.y;
        const d2 = dx * dx + dy * dy;
        const q = d2 / (l.r * l.r);
        if (q >= Q) continue;
        const j = ((q * (N / Q)) | 0) * 3;
        const d = Math.sqrt(d2) + 1;
        const tilt = 1 - (grain * (nx[i] * dx + ny[i] * dy)) / (d + 160);
        const s = l.i * (tilt > 0 ? tilt : 0);
        const c = l.c ?? WHITE;
        r += LUT[j] * s * c[0];
        g += LUT[j + 1] * s * c[1];
        b += LUT[j + 2] * s * c[2];
      }
      const p = i * 4;
      out[p] = TONE[Math.min(2047, (src[p] * r * 2.0078) | 0)];
      out[p + 1] = TONE[Math.min(2047, (src[p + 1] * g * 2.0078) | 0)];
      out[p + 2] = TONE[Math.min(2047, (src[p + 2] * b * 2.0078) | 0)];
      out[p + 3] = 255;
    }
  }
  return out;
}
