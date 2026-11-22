// A window outline with its platform's title bar controls.
import { rect, line, circle } from './shapes.js';
import { text } from './text.js';

const RADIUS = { windows: 8, mac: 14, linux: 12 };

export function window(x, y, w, h, o = {}) {
  const chrome = o.chrome ?? 'windows';
  if (!(chrome in RADIUS)) throw new Error(`window chrome ${chrome}; known: ${Object.keys(RADIUS).join(' ')}`);
  const bar = o.bar ?? 56;
  const p = { w: o.w, app: o.app };
  const mid = y + bar / 2;
  const out = [rect(x, y, w, h, { ...p, r: RADIUS[chrome] }), line(x, y + bar, x + w, y + bar, p)];
  if (chrome === 'mac') {
    for (let i = 0; i < 3; i++) out.push(circle(x + 30 + i * 26, mid, 7.5, p));
  } else if (chrome === 'windows') {
    const g = 7;
    out.push(line(x + w - 156 - g, mid, x + w - 156 + g, mid, p));
    out.push(rect(x + w - 102 - g, mid - g, g * 2, g * 2, p));
    out.push(line(x + w - 46 - g, mid - g, x + w - 46 + g, mid + g, p));
    out.push(line(x + w - 46 + g, mid - g, x + w - 46 - g, mid + g, p));
  } else {
    const g = 5;
    for (const [i, cx] of [[0, x + w - 112], [1, x + w - 74], [2, x + w - 36]]) {
      out.push(circle(cx, mid, 13, p));
      if (i === 0) out.push(line(cx - g, mid + 1, cx + g, mid + 1, p));
      if (i === 1) out.push(rect(cx - g, mid - g, g * 2, g * 2, p));
      if (i === 2) out.push(line(cx - g, mid - g, cx + g, mid + g, p), line(cx + g, mid - g, cx - g, mid + g, p));
    }
  }
  if (o.title) {
    const size = o.titleSize ?? 28;
    const left = chrome === 'windows';
    out.push(...text(o.title, { x: left ? x + 22 : x + w / 2, y: mid + size * 0.36, size, align: left ? 'left' : 'center', ...p, w: o.w }));
  }
  return out;
}
