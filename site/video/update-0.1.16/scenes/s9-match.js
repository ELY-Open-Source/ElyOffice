// s9's spent match: a cracked head, a charred third.
import { rnd, signed } from '../lib/rand.js';

const WOOD = [222, 186, 132];
const CHAR = [34, 26, 22];
const SCORCH = [116, 68, 40];
const CRACK = [156, 142, 128];

export function spent(ink, [hx, hy, a]) {
  const c = Math.cos(a);
  const s = Math.sin(a);
  const len = 330;
  const w = 7;
  const burnt = len / 3;
  const bend = (d) => (d < burnt ? 8 * Math.sin((Math.PI * d) / burnt) * (1 - (0.4 * d) / burnt) : 0);
  const along = (d, off) => [hx + c * d - s * (off + bend(d)), hy + s * d + c * (off + bend(d))];
  const thick = (d) => (d < burnt ? w * (0.48 + (0.47 * d) / burnt) : w);

  const wood = [[along(burnt - 6, -w), along(len, -w * 0.9), along(len, w * 0.9), along(burnt - 6, w)]];
  ink.wash(wood, { color: WOOD, alpha: 0.75, bleed: 1.5 });
  ink.line([along(burnt, -w), along(len, -w * 0.9)], { width: 2.2, overshoot: 3 });
  ink.line([along(burnt, w), along(len, w * 0.9)], { width: 2.6, overshoot: 3 });
  ink.line([along(len, -w * 0.9), along(len + 2, w * 0.9)], { width: 2 });
  ink.line([along(burnt + 40, 1), along(250, 2)], { width: 1, alpha: 0.4, style: 'charcoal' });
  ink.wash([[along(burnt - 12, -w), along(burnt + 18, -w), along(burnt + 26, 0), along(burnt + 18, w), along(burnt - 12, w)]],
    { color: SCORCH, alpha: 0.7, bleed: 3 });

  const left = [];
  const right = [];
  for (let d = 12; d <= burnt + 2; d += 6) {
    left.push(along(d, -thick(d)));
    right.push(along(d, thick(d)));
  }
  const coal = [[...left, ...[...right].reverse()]];
  ink.wash(coal, { color: CHAR, alpha: 0.95, bleed: 1.5 });
  ink.wash(coal, { color: CHAR, alpha: 0.8, bleed: 1 });
  ink.hatch(coal, { gap: 3, width: 1.1, angle: a + 1.3, alpha: 0.7 });
  ink.curve(left, { width: 2, overshoot: 2, segs: 4 });
  ink.curve(right, { width: 2.4, overshoot: 2, segs: 4 });

  const lump = [];
  for (let i = 0; i < 11; i++) {
    const q = (i / 11) * Math.PI * 2;
    const r = 12 * (0.62 + 0.55 * rnd(311, i)) * (i === 3 ? 0.45 : 1);
    lump.push(along(6 + Math.cos(q) * r * 1.25, Math.sin(q) * r));
  }
  ink.wash([lump], { color: CHAR, alpha: 0.96, bleed: 1.2 });
  ink.wash([lump], { color: CHAR, alpha: 0.85, bleed: 0.8 });
  ink.hatch([lump], { gap: 2.6, width: 1, angle: a - 0.8, alpha: 0.7 });
  ink.line([...lump, lump[0]], { width: 2.2, overshoot: 2, taper: [0.05, 0.1] });
  for (let k = 0; k < 3; k++) {
    const d0 = 1 + 4 * k;
    const off = signed(313, k) * 4;
    ink.line([along(d0, off), along(d0 + 5, off + signed(317, k) * 3), along(d0 + 9, off - signed(319, k) * 2)],
      { width: 1, color: CRACK, alpha: 0.85, overshoot: 0, taper: [0.2, 0.4] });
  }
}
