// Zhang-Suen thinning, staircase cleanup, distance transform.

const DX = [0, 1, 1, 1, 0, -1, -1, -1];
const DY = [-1, -1, 0, 1, 1, 1, 0, -1];

function ring(img, w, i) {
  const x = i % w;
  const y = (i - x) / w;
  const out = new Array(8);
  for (let k = 0; k < 8; k++) out[k] = img[(y + DY[k]) * w + x + DX[k]];
  return out;
}

export function zhangSuen(src, w, h) {
  const img = src.slice();
  for (let x = 0; x < w; x++) img[x] = img[(h - 1) * w + x] = 0;
  for (let y = 0; y < h; y++) img[y * w] = img[y * w + w - 1] = 0;
  let changed = true;
  while (changed) {
    changed = false;
    for (let pass = 0; pass < 2; pass++) {
      const del = [];
      for (let i = w; i < w * (h - 1); i++) {
        if (!img[i]) continue;
        const p = ring(img, w, i);
        let b = 0;
        let a = 0;
        for (let k = 0; k < 8; k++) {
          b += p[k];
          if (!p[k] && p[(k + 1) & 7]) a++;
        }
        if (b < 2 || b > 6 || a !== 1) continue;
        if (pass === 0 ? p[0] * p[2] * p[4] || p[2] * p[4] * p[6] : p[0] * p[2] * p[6] || p[0] * p[4] * p[6]) continue;
        del.push(i);
      }
      for (const i of del) img[i] = 0;
      if (del.length) changed = true;
    }
  }
  return img;
}

function pieces(p) {
  const on = [];
  for (let k = 0; k < 8; k++) if (p[k]) on.push(k);
  const root = on.map((_, i) => i);
  const top = (i) => (root[i] === i ? i : (root[i] = top(root[i])));
  for (let i = 0; i < on.length; i++) {
    for (let j = i + 1; j < on.length; j++) {
      if (Math.abs(DX[on[i]] - DX[on[j]]) <= 1 && Math.abs(DY[on[i]] - DY[on[j]]) <= 1) root[top(i)] = top(j);
    }
  }
  return { n: on.length, parts: new Set(on.map((_, i) => top(i))).size };
}

export function tidy(img, w, h) {
  let changed = true;
  while (changed) {
    changed = false;
    for (let i = w; i < w * (h - 1); i++) {
      if (!img[i]) continue;
      const { n, parts } = pieces(ring(img, w, i));
      if (n >= 2 && parts === 1) {
        img[i] = 0;
        changed = true;
      }
    }
  }
  return img;
}

export function distance(bin, w, h) {
  const d = new Float32Array(w * h);
  const INF = 1e9;
  for (let i = 0; i < d.length; i++) d[i] = bin[i] ? INF : 0;
  const A = 1;
  const B = Math.SQRT2;
  for (let y = 1; y < h - 1; y++) {
    for (let x = 1; x < w - 1; x++) {
      const i = y * w + x;
      if (!d[i]) continue;
      d[i] = Math.min(d[i], d[i - 1] + A, d[i - w] + A, d[i - w - 1] + B, d[i - w + 1] + B);
    }
  }
  for (let y = h - 2; y > 0; y--) {
    for (let x = w - 2; x > 0; x--) {
      const i = y * w + x;
      if (!d[i]) continue;
      d[i] = Math.min(d[i], d[i + 1] + A, d[i + w] + A, d[i + w + 1] + B, d[i + w - 1] + B);
    }
  }
  return d;
}

export function blobs(bin, img, dt, w) {
  const label = new Int32Array(bin.length).fill(-1);
  for (let i = 0; i < bin.length; i++) {
    if (!bin[i] || label[i] >= 0) continue;
    const list = [i];
    label[i] = i;
    let found = false;
    let best = i;
    for (let q = 0; q < list.length; q++) {
      const p = list[q];
      if (img[p]) found = true;
      if (dt[p] > dt[best]) best = p;
      for (const j of [p - 1, p + 1, p - w, p + w]) {
        if (bin[j] && label[j] < 0) {
          label[j] = i;
          list.push(j);
        }
      }
    }
    if (!found) img[best] = 1;
  }
  return img;
}

export const NEIGHBOURS = { DX, DY };
