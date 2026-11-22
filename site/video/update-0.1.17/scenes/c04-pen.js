// A pen table: drawing phases, else follow the path.
import { kit as k } from '../lib/kit/index.js';
import { FPS } from '../lib/time.js';

export function penTable(scene, phases) {
  const f0 = Math.round(scene.start * FPS);
  const f1 = Math.round(scene.end * FPS);
  const out = new Float64Array(f1 - f0 + 1);
  const base = new Map();
  let top = 0;
  let seen = 0;
  for (let f = f0; f <= f1; f++) {
    const t = f / FPS;
    let want = 0;
    if (t > scene.start + 0.75 + 1e-9) {
      const mk = k.marks(scene.strokes(t, k));
      // The gate samples the path on even frames only.
      if (f % 2 === 0 || f === f1) seen = Math.max(seen, mk.total);
      const p = phases.find((q) => t <= q.t1 + 1e-9 && t >= q.t0 - 1e-9);
      const end = (i) => (i < 0 ? 0 : mk.list[i].end);
      if (p && !base.has(p)) base.set(p, Math.max(top, end(p.from)));
      want = p ? k.lerp(base.get(p), end(p.to), k.ease[p.ease](k.span(t, p.t0, p.t1))) : mk.total;
      want = Math.min(want, mk.total, seen);
    }
    top = Math.max(top, want);
    out[f - f0] = top;
  }
  return (t) => out[Math.max(0, Math.min(out.length - 1, Math.round(t * FPS) - f0))];
}
