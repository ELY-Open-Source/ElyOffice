// Seeded hashing and noise; no state, no clock.

export function hash(...parts) {
  let h = 0x9e3779b9;
  for (const p of parts) {
    let k = typeof p === 'string' ? text(p) : Math.floor(p) | 0;
    k = Math.imul(k ^ (k >>> 16), 0x85ebca6b);
    k = Math.imul(k ^ (k >>> 13), 0xc2b2ae35);
    h = Math.imul(h ^ k ^ (k >>> 16), 0x27d4eb2d) + 0x165667b1;
    h ^= h >>> 15;
  }
  h = Math.imul(h ^ (h >>> 16), 0x85ebca6b);
  h = Math.imul(h ^ (h >>> 13), 0xc2b2ae35);
  return (h ^ (h >>> 16)) >>> 0;
}

function text(s) {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619);
  return h | 0;
}

export function rnd(...parts) {
  return hash(...parts) / 4294967296;
}

export function signed(...parts) {
  return rnd(...parts) * 2 - 1;
}

export function stream(seed) {
  let s = hash(seed) || 1;
  return () => {
    s = (s + 0x6d2b79f5) | 0;
    let r = Math.imul(s ^ (s >>> 15), 1 | s);
    r = (r + Math.imul(r ^ (r >>> 7), 61 | r)) ^ r;
    return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
  };
}

const fade = (u) => u * u * u * (u * (u * 6 - 15) + 10);

export function noise1(x, seed = 0) {
  const i = Math.floor(x);
  const f = x - i;
  const a = signed(seed, i);
  const b = signed(seed, i + 1);
  return a + (b - a) * fade(f);
}

export function noise2(x, y, seed = 0) {
  const i = Math.floor(x);
  const j = Math.floor(y);
  const fx = fade(x - i);
  const fy = fade(y - j);
  const a = signed(seed, i, j);
  const b = signed(seed, i + 1, j);
  const c = signed(seed, i, j + 1);
  const d = signed(seed, i + 1, j + 1);
  return a + (b - a) * fx + (c - a) * fy + (a - b - c + d) * fx * fy;
}

export function fbm(x, seed = 0, octaves = 3) {
  let sum = 0;
  let amp = 1;
  let norm = 0;
  for (let o = 0; o < octaves; o++) {
    sum += noise1(x, seed + o * 101) * amp;
    norm += amp;
    amp *= 0.5;
    x *= 2.03;
  }
  return sum / norm;
}
