// ElyMark Settings: the document text size row, moved and reset.
import { kit as k } from '../lib/kit/index.js';
import { reveal, settle, scale, resetArrow } from './c06-util.js';

const X = 80;
const Y = 580;
const WD = 840;
const HT = 430;
const ROW = 704;
const S0 = 400;
const S1 = 740;
const HOME = 0.28;
const FAR = 0.76;
const ARROW = [800, ROW - 7];
const TAIL = [ARROW[0] + 16 * Math.cos((155 / 180) * Math.PI), ARROW[1] + 16 * Math.sin((155 / 180) * Math.PI)];
const T = { grab: 52.5, let: 53.25, reset: 54.0 };

const chrome = k.window(X, Y, WD, HT, { chrome: 'linux', title: 'Settings', titleSize: 20, bar: 48, w: 0.8 });
const label = k.text('Document text size', { x: 124, y: ROW, size: 24, w: 0.8 });
const rule = { ...k.line(X + 40, ROW + 44, X + WD - 40, ROW + 44), w: 0.35, alpha: 0.6, link: 0.4 };
const sample = k.text('Aa', { x: 500, y: 920, size: 64, align: 'center', w: 0.85 });

// Knob position: anticipation, drag, release, spring home.
function knob(t) {
  const out = k.keys(t, [[T.grab, HOME], [T.grab + 0.0625, HOME - 0.012, 'out'], [T.let, FAR, 'inOut']]);
  if (t < T.reset) return out;
  return FAR + (HOME - FAR) * settle(t - T.reset, 17, 6.5);
}

const size = (f) => 1 + (f - HOME) * 1.15;
const press = (t) => k.keys(t, [[T.grab - 0.0625, 0], [T.grab, 1, 'out'], [T.let, 1], [T.let + 0.125, 0]]);
const knobTop = (t) => [S0 + (S1 - S0) * knob(Math.min(t, T.let)), ROW - 18 - 3 * press(t)];

export const settings = {
  t0: 51.5,
  t1: 52.375,
  strokes(t) {
    const f = knob(t);
    const x = S0 + (S1 - S0) * f;
    const spin = -Math.PI * 2 * k.ease.out(k.span(t, T.reset, T.reset + 0.375));
    const arrow = resetArrow(ARROW[0], ARROW[1], 16, spin, { w: 0.9, alpha: t >= T.reset + 0.25 ? 0.32 : 1 });
    return [
      ...chrome, ...label, rule,
      { ...k.line(S0, ROW - 7, x, ROW - 7), w: 1.25 }, { ...k.line(x, ROW - 7, S1, ROW - 7), w: 0.4 },
      ...scale(sample, size(f), 500, 920),
      k.circle(x, ROW - 7, 11 + 3 * press(t), { w: 0.9 + 0.4 * press(t) }),
      ...reveal([arrow], k.ease.out(k.span(t, T.let, T.let + 0.1875))),
    ];
  },
  // Pointer: knob, drag, over to reset, click.
  cursor(t, tip, to) {
    if (t < this.t1) return null;
    if (t < T.grab - 0.0625) return k.keys(t, [[this.t1, tip], [T.grab - 0.0625, knobTop(T.grab - 0.0625), 'inOut']]);
    if (t < T.let + 0.125) return knobTop(t);
    return k.keys(t, [[T.let + 0.125, knobTop(T.let + 0.125)], [T.reset - 0.0625, TAIL, 'inOut'], [T.reset + 0.25, TAIL], [54.5, to, 'inOut']]);
  },
  clicks: [[T.grab, S0 + (S1 - S0) * HOME, ROW - 7], [T.reset, ...TAIL]],
  cam: { x: X + WD / 2, y: Y + HT / 2, w: 1000 },
};
