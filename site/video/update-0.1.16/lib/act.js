// Acting for the flame: hops, landings, blends.
import { REST } from './flame.js';
import { lerp, ease, clamp, spring } from './time.js';

export function hop(t, o) {
  const pre = o.pre ?? 0.17;
  const post = o.post ?? 0.4;
  const [fx, fy] = o.from;
  const [tx, ty] = o.to;
  const dir = Math.sign(tx - fx);
  if (t < o.t0 - pre || t > o.t1 + post) return null;
  if (t < o.t0) {
    const u = ease.in((t - (o.t0 - pre)) / pre);
    return { x: fx, y: fy, squash: 1 - 0.32 * u, lean: dir * 0.22 * u };
  }
  if (t <= o.t1) {
    const d = o.t1 - o.t0;
    const u = (t - o.t0) / d;
    const vy = (ty - fy - 4 * o.height * (1 - 2 * u)) / d;
    const vx = (tx - fx) / d;
    const speed = Math.hypot(vx, vy);
    return {
      x: lerp(fx, tx, u),
      y: lerp(fy, ty, u) - 4 * o.height * u * (1 - u),
      squash: 1 + 0.42 * clamp(speed / 1400) * (1 - 0.6 * Math.sin(Math.PI * u)),
      lean: clamp(Math.atan2(vx, Math.max(Math.abs(vy), 200)) * 0.9, -0.7, 0.7),
    };
  }
  const tau = t - o.t1;
  return { x: tx, y: ty, squash: 1 - spring(tau, 0.38, 15, 7), lean: dir * spring(tau, -0.18, 9, 6) };
}

export function blend(a, b, u) {
  const out = { ...REST, ...a };
  const bb = { ...REST, ...b };
  for (const k of Object.keys(REST)) out[k] = lerp(out[k], bb[k], u);
  return out;
}

export function first(...states) {
  for (const s of states) if (s) return s;
  return null;
}
