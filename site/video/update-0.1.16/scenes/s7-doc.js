// ElyDoc page: the first line indents by two characters.
import { span, ease, lerp, spring, smooth } from '../lib/time.js';
import { glyphs } from '../lib/glyph.js';

export const SIZE = 46;
const COLS = 15;
const LEAD = 72;
export const X0 = 960 - (COLS * SIZE) / 2;
export const Y0 = 418;
export const PUSH = [65.5, 66.0];
const TEXT = [...'春雨落在旧城的屋檐上，巷子里的石板一块块亮了起来。她撑着伞走过桥头，听见远处有人在唱一支很老的歌，调子和外婆当年哼的一模一样。'];
const BLUE = [61, 127, 229];
const GHOST = [70, 100, 170];
const PAGE = { x0: X0 - 70, x1: X0 + COLS * SIZE + 70, y0: 236, y1: 830 };

const cell = (j) => [X0 + (j % COLS) * SIZE, Y0 + Math.floor(j / COLS) * LEAD];
const row = (j) => Math.floor(j / COLS);

export const lineStart = (t) => cell(0)[0] + 2 * SIZE * pushU(t, 0);

function pushU(t, line) {
  const a = PUSH[0] + 0.07 * line;
  return ease.inOut(span(t, a, a + 0.5));
}

export const landing = (line) => PUSH[1] + 0.07 * line;

function placed(i, t) {
  const line = row(i);
  const u = pushU(t, line);
  const [ax, ay] = cell(i);
  const [bx, by] = cell(i + 2);
  const settle = spring(t - landing(line), 5, 22, 9);
  if (row(i + 2) === line) return [lerp(ax, bx, u) + settle, ay];
  const arc = 46 * Math.sin(Math.PI * u);
  return [lerp(ax, bx, u) + settle, lerp(ay, by, u) - arc];
}

export function page(ink, title) {
  const { x0, x1, y0, y1 } = PAGE;
  ink.wash([[[x0, y0], [x1, y0], [x1, y0 + 54], [x0, y0 + 54]]], { color: BLUE, alpha: 0.32, bleed: 4 });
  ink.text(title, { x: (x0 + x1) / 2, y: y0 + 37, size: 26, align: 'center', color: [24, 40, 80] });
  ink.line([[x0, y0], [x1, y0]], { width: 2.6 });
  ink.line([[x1, y0], [x1, y1]], { width: 2.4 });
  ink.line([[x1, y1], [x0, y1]], { width: 2.8 });
  ink.line([[x0, y1], [x0, y0]], { width: 2.2 });
  ink.smudge(x1 + 8, (y0 + y1) / 2, 60, { alpha: 0.1, blots: 4 });
  ink.smudge((x0 + x1) / 2, y1 + 8, 90, { alpha: 0.08, blots: 4 });
}

export function paragraph(ink, t) {
  const list = TEXT.map((ch, i) => {
    const [x, y] = placed(i, t);
    return glyphs(ch, { x, y, size: SIZE })[0];
  });
  ink.write(list, { size: SIZE, fill: 'solid', alpha: 0.88, width: 1.6 });
  const ghost = smooth(span(t, PUSH[1] - 0.04, PUSH[1] + 0.2));
  if (ghost <= 0) return;
  const g = [0, 1].map((k) => glyphs('字', { x: X0 + k * SIZE, y: Y0, size: SIZE })[0]);
  ink.write(g, { size: SIZE, fill: 'hatch', gap: 3.6, angle: -0.8, color: GHOST, alpha: 0.5 * ghost, width: 1.8 });
}
