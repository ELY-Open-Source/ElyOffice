// The pen front: stroke groups drawn over set windows.
import { kit } from '../lib/kit/index.js';
import { FPS } from '../lib/time.js';

// Groups: from, to, a, b, ease; indices into strokes.
function front(groups, strokesAt, t) {
  let g = null;
  for (const x of groups) if (x.a <= t) g = x;
  if (!g) return 0;
  const list = strokesAt(t);
  if (g === groups.at(-1) && t >= g.b) return kit.marks(list).total;
  const m = kit.marks(list.slice(0, g.to + 1)).list;
  const start = g.from > 0 ? m[g.from - 1].end : 0;
  const end = m[g.to].end;
  if (t >= g.b) return end;
  return start + kit.ease[g.ease ?? 'inOut'](kit.span(t, g.a, g.b)) * (end - start);
}

// Running maximum per frame, so the pen never falls.
export function schedule(groups, strokesAt, start, end) {
  let table = null;
  const f0 = Math.round(start * FPS);
  const build = () => {
    const n = Math.round(end * FPS) - f0 + 1;
    table = new Float64Array(n);
    let top = 0;
    for (let i = 0; i < n; i++) {
      top = Math.max(top, front(groups, strokesAt, (f0 + i) / FPS));
      table[i] = top;
    }
  };
  return (t) => {
    if (!table) build();
    const i = Math.floor(t * FPS + 1e-6) - f0;
    return table[Math.max(0, Math.min(table.length - 1, i))];
  };
}
