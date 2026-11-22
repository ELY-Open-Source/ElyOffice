// ElyShow smart guides: edges, centres, slide, gaps; Alt frees.
import { build, penAt } from './c07-line.js';
import { cursorAt, cameraAt } from './c07-motion.js';

export default {
  id: 'c07', start: 58, end: 68, app: 'show',
  strokes: (t) => build(t),
  pen: (t) => penAt(t),
  cursor: (t, k) => cursorAt(t, k),
  camera: (t) => cameraAt(t),
};
