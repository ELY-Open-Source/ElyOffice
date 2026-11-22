// The kit handed to scenes: time, shapes, text, icons, windows.
import { keys, span, ease, spring, lerp, clamp, beat } from '../time.js';
import { rnd } from '../rand.js';
import { length } from '../geom.js';
import { marks } from '../line.js';
import { stroke, line, poly, arc, circle, rect, check, morph, ripple, move, leave } from './shapes.js';
import { text, measure } from './text.js';
import { icon } from './icon.js';
import { window } from './window.js';

export const kit = {
  keys, span, ease, spring, lerp, clamp, beat,
  hash: (...parts) => rnd(...parts),
  stroke, line, poly, arc, circle, rect, check, morph, ripple, move, leave,
  text, measure, icon, window,
  length: (s) => length(s.pts),
  marks,
};

export function kitFor(where) {
  return { ...kit, ...where };
}
