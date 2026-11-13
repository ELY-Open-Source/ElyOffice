// One frame from the scene list: page, light, fire.
import { createCanvas } from '@napi-rs/canvas';
import scenes from '../scenes/index.js';
import { paper, illuminate } from './paper.js';
import { REST, rested, drawFlame, lightOf } from './flame.js';
import { embers } from './embers.js';
import { kit } from './ink.js';
import { W, H, onOnes, onTwos, drawingOf } from './time.js';

export function sceneAt(t) {
  for (const s of scenes) if (t >= s.start && t < s.end) return s;
  const last = scenes[scenes.length - 1];
  return last && t >= last.start && t <= last.end + 1e-9 ? last : null;
}

export function flameAt(t) {
  if (!scenes.length || t < scenes[0].start) return { ...REST, alpha: 0, pool: 0 };
  const s = sceneAt(t);
  return s ? rested(s.flame(t)) : { ...REST };
}

export const filmEnd = () => scenes[scenes.length - 1].end;

export class Film {
  constructor() {
    this.canvas = createCanvas(W, H);
    this.ctx = this.canvas.getContext('2d');
    this.page = createCanvas(W, H);
    this.pageCtx = this.page.getContext('2d');
    this.drawing = -1;
    this.lit = this.ctx.createImageData(W, H);
    this.src = null;
  }

  drawPage(n) {
    const d = drawingOf(n);
    if (d === this.drawing) return;
    const t = onTwos(n);
    const ctx = this.pageCtx;
    ctx.save();
    ctx.globalCompositeOperation = 'copy';
    ctx.drawImage(paper().canvas, 0, 0);
    ctx.restore();
    const scene = sceneAt(t);
    if (scene) {
      ctx.save();
      scene.draw(ctx, t, kit(ctx, d, lightOf(flameAt(t), t)));
      ctx.restore();
    }
    this.src = ctx.getImageData(0, 0, W, H).data;
    this.drawing = d;
  }

  frame(n) {
    const t = onOnes(n);
    this.drawPage(n);
    const scene = sceneAt(t);
    const flame = flameAt(t);
    const lights = [lightOf(flame, t), ...(scene?.lights?.(t) ?? [])];
    illuminate(this.src, this.lit.data, lights, scene?.ambient?.(t) ?? 0, scene?.grain);
    const ctx = this.ctx;
    ctx.putImageData(this.lit, 0, 0);
    if (scene?.fx) {
      ctx.save();
      scene.fx(ctx, t, kit(ctx, n, lights[0]));
      ctx.restore();
    }
    embers(ctx, t, flameAt);
    drawFlame(ctx, flame, t, n);
    return this.canvas.data();
  }
}
