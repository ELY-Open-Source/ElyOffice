// Frame clock, easing, keyframes, the beat grid.

export const W = 1920;
export const H = 1080;
export const FPS = 24;
export const BEAT = 0.5;
export const BAR = 2;
export const LENGTH = 90;

export const onOnes = (n) => n / FPS;
export const onTwos = (n) => (n - (n & 1)) / FPS;
export const drawingOf = (n) => n >> 1;

export const clamp = (v, a = 0, b = 1) => (v < a ? a : v > b ? b : v);
export const lerp = (a, b, u) => a + (b - a) * u;
export const span = (t, a, b) => clamp((t - a) / (b - a));
export const smooth = (u) => u * u * (3 - 2 * u);

export const ease = {
  linear: (u) => u,
  in: (u) => u * u * u,
  out: (u) => 1 - (1 - u) ** 3,
  inOut: (u) => (u < 0.5 ? 4 * u * u * u : 1 - (-2 * u + 2) ** 3 / 2),
  back: (u) => 1 + 2.4 * (u - 1) ** 3 + 1.4 * (u - 1) ** 2,
  snap: (u) => 1 - (1 - u) ** 6,
};

export function keys(t, frames) {
  if (t <= frames[0][0]) return frames[0][1];
  for (let i = 1; i < frames.length; i++) {
    const [t1, v1, how = 'inOut'] = frames[i];
    if (t <= t1) {
      const [t0, v0] = frames[i - 1];
      const u = ease[how]((t - t0) / (t1 - t0));
      return Array.isArray(v0) ? v0.map((a, k) => lerp(a, v1[k], u)) : lerp(v0, v1, u);
    }
  }
  return frames[frames.length - 1][1];
}

export function spring(tau, amount, freq = 13, damp = 6) {
  if (tau < 0) return 0;
  return amount * Math.exp(-damp * tau) * Math.cos(freq * tau);
}
