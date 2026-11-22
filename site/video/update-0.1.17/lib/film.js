// One frame: paper, the line, the pen bead.
import { createCanvas } from '@napi-rs/canvas';
import { cameraAt, matrix } from './camera.js';
import { lineAt } from './path.js';
import { drawPaper } from './paper.js';
import { drawLine, drawBead } from './draw.js';
import { W, H, FPS, TOUCH, LIFT, span, smooth, lerp } from './time.js';

const TAIL = 40;
const LIFTING = 0.3;

function taper(b, u, s) {
  let d = 0;
  for (let i = b.x.length - 1; i >= 0 && d < TAIL; i--) {
    b.wd[i] *= lerp(1, 0.25 + 0.75 * smooth(d / TAIL), u);
    if (i > 0) d += Math.hypot(b.x[i] - b.x[i - 1], b.y[i] - b.y[i - 1]) / s;
  }
}

export class Film {
  constructor() {
    this.canvas = createCanvas(W, H);
    this.ctx = this.canvas.getContext('2d');
  }

  frame(n) {
    const t = n / FPS;
    const m = matrix(cameraAt(n));
    drawPaper(this.ctx, m);
    const line = lineAt(n);
    const lift = span(t, LIFT - LIFTING, LIFT);
    if (lift > 0) taper(line.blocks[line.blocks.length - 1], lift, line.entry.s.place.s);
    this.points = drawLine(this.ctx, line.blocks, m, t);
    if (line.tip && t >= TOUCH && t < LIFT) drawBead(this.ctx, line.tip.at, m, line.entry.s.app, 3.5 * (1 - lift));
    return this.canvas.data();
  }
}
