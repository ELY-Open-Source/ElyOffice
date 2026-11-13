// s7: ElyDoc indents by two characters; ElyTree drops its graph.
import { keys, span, clamp } from '../lib/time.js';
import { noise1, rnd } from '../lib/rand.js';
import { REST } from '../lib/flame.js';
import { hop, first } from '../lib/act.js';
import { burst } from '../lib/embers.js';
import { rgba } from '../lib/stroke.js';
import * as doc from './s7-doc.js';
import * as tree from './s7-tree.js';

const P0 = [doc.X0 - 34, doc.Y0 + 6];
const PERCH = [doc.X0 + 24, doc.Y0 - 64];
const AWAY = [1190, 772];
const WATCH = [1296, 792];
const GULPS = [68.5, 69.0, 69.5];
const twos = (t) => Math.floor(t * 12 + 1e-6) / 12;

const docX = (t) => keys(t, [[64, 1300], [64.5, 0, 'back'], [66.95, 0], [67.35, -1500, 'in']]);
const treeX = (t) => keys(t, [[67.25, 1500], [67.7, 0, 'back'], [73.3, 0], [73.95, -1600, 'in']]);

function motion(f, t) {
  const a = f(t - 1 / 48);
  const b = f(t + 1 / 48);
  return [(b[0] - a[0]) * 24, (b[1] - a[1]) * 24];
}

function onDoc(t) {
  if (t < 64.45) return { lean: keys(t, [[64, 0], [64.2, 0.16], [64.45, 0]]) };
  const h1 = hop(t, { t0: 64.62, t1: 64.92, from: [REST.x, REST.y], to: P0, height: 90 });
  if (h1 && t < 65.32) return h1;
  if (t < doc.PUSH[0]) {
    const u = span(t, 65.32, doc.PUSH[0]);
    return { x: P0[0] - 8 * u, y: P0[1], lean: 0.32 * u, squash: 1 - 0.16 * u, energy: 0.5 + 0.3 * u };
  }
  if (t < doc.PUSH[1]) {
    const x = doc.lineStart(twos(t)) - 34;
    return { x, y: P0[1], lean: 0.42, squash: 0.86, energy: 0.9 };
  }
  if (t < 66.48) {
    const from = [doc.X0 + 58, P0[1]];
    return {
      ...(hop(t, { t0: 66.06, t1: 66.28, from, to: PERCH, height: 34, pre: 0.06, post: 0.3 }) ?? { x: from[0], y: from[1] }),
      energy: 0.8, roar: keys(t, [[66.0, 0.45], [66.3, 0.05]]),
    };
  }
  return first(
    hop(t, { t0: 66.65, t1: 66.95, from: PERCH, to: AWAY, height: 110 }),
    { x: AWAY[0], y: AWAY[1], lean: -0.14 * span(t, 67.0, 67.2) + 0.14 * span(t, 67.3, 67.5) },
  );
}

function onTree(t) {
  if (t < 70.6) {
    const land = hop(t, { t0: 67.75, t1: 68.0, from: AWAY, to: tree.MOUTH, height: 150 }) ?? { x: tree.MOUTH[0], y: tree.MOUTH[1] };
    const gulp = GULPS.reduce((a, g) => a + Math.exp(-(((t - g) / 0.07) ** 2)), 0);
    const eating = span(t, 68.05, 68.2) * (1 - span(t, 69.7, 69.8));
    return {
      ...land,
      squash: (land.squash ?? 1) * (1 + 0.16 * gulp),
      lean: (land.lean ?? 0) + 0.22 * eating * noise1(t * 5, 31),
      roar: keys(t, [[68.1, 0], [69.6, 0.75, 'in'], [69.8, 0.2], [69.95, 0]]),
      ember: keys(t, [[69.72, 0], [70.0, 1, 'in'], [70.5, 1], [70.62, 0, 'out']]),
      energy: 0.5 + 0.5 * eating,
    };
  }
  if (t < tree.DRAG[0]) {
    return { ...hop(t, { t0: 70.72, t1: 70.95, from: tree.MOUTH, to: tree.dragPath(tree.DRAG[0]), height: 70, pre: 0.12, post: 0.05 }),
      roar: keys(t, [[70.6, 0.5], [70.8, 0]]) };
  }
  if (t < tree.DRAG[1]) {
    const [x, y] = tree.dragPath(twos(t));
    const [vx] = motion(tree.dragPath, t);
    return { x, y, lean: clamp(vx / 2600, -0.5, 0.5), squash: 0.9, energy: 0.85 };
  }
  const end = tree.dragPath(tree.DRAG[1]);
  return first(
    hop(t, { t0: 72.12, t1: 72.4, from: end, to: WATCH, height: 80, pre: 0.12 }),
    t < 72.93 && { x: WATCH[0], y: WATCH[1], lean: -0.16 * span(t, 72.8, 72.93) },
    hop(t, { t0: 73.1, t1: 73.4, from: WATCH, to: [REST.x, REST.y], height: 100, post: 0.45 }),
    {},
  );
}

function flame(t) {
  const pool = keys(t, [[64, REST.pool], [64.6, 700], [67.6, 820], [69.72, 820], [70.0, 1350], [70.5, 1350], [70.62, 820],
    [73.2, 820], [73.9, REST.pool]]);
  const h = keys(t, [[64.45, REST.h], [64.92, 96], [72.4, 96], [73.4, REST.h, 'out']]);
  return { h, pool, ...(t < 67.58 ? onDoc(t) : onTree(t)) };
}

function slid(ctx, dx, paint) {
  if (Math.abs(dx) > 1450) return;
  ctx.save();
  ctx.translate(dx + 960, 540);
  ctx.rotate(-dx * 0.00004);
  ctx.translate(-960, -540);
  paint();
  ctx.restore();
}

function draw(ctx, t, ink) {
  if (t < 67.4) {
    slid(ctx, docX(t), () => {
      doc.page(ink, '春雨.docx');
      doc.paragraph(ink, t);
    });
  }
  if (t < 67.25) return;
  const tx = treeX(t);
  slid(ctx, tx, () => {
    tree.page(ink);
    tree.selection(ink, t);
    tree.graph(ink, t);
    tree.rows(ink, t);
    tree.smoke(ink, t);
  });
  const s = tree.strip(t);
  if (s.on) {
    ctx.save();
    ctx.translate(tx, 0);
    ink.lift(s.height, [960 + tx + s.dx, 504 + s.dy], (k) => tree.stripPaint(k, t));
    ctx.restore();
  }
}

function streaks(ctx, t) {
  const [mx, my] = [tree.MOUTH[0], tree.MOUTH[1] - 40];
  tree.cutEnds(t).forEach(({ p, color }, n) => {
    for (let j = 0; j < 3; j++) {
      const u = (t * 2.6 + j / 3 + rnd(71, n)) % 1;
      const e = u * u;
      const x = p[0] + (mx - p[0]) * e;
      const y = p[1] + (my - p[1]) * e;
      const bx = p[0] + (mx - p[0]) * Math.max(0, e - 0.12);
      const by = p[1] + (my - p[1]) * Math.max(0, e - 0.12);
      ctx.strokeStyle = rgba(color, 0.85 * (1 - u));
      ctx.lineWidth = 4 * (1 - u) + 1;
      ctx.lineCap = 'round';
      ctx.beginPath();
      ctx.moveTo(bx, by);
      ctx.lineTo(x, y);
      ctx.stroke();
    }
  });
}

function fx(ctx, t) {
  const sx = doc.X0 + 2 * doc.SIZE + docX(t);
  burst(ctx, t, { t0: doc.PUSH[1], x: sx - 6, y: doc.Y0 - 20, count: 16, speed: 240, life: 0.5, angle: -1.9, spread: 0.9, gravity: 380, seed: 71, size: 1.8 });
  if (t > tree.EAT[0] && t < tree.EAT[1]) streaks(ctx, t);
  const [mx, my] = tree.MOUTH;
  burst(ctx, t, { t0: 70.0, x: mx, y: my - 10, count: 22, speed: 150, life: 0.9, spread: 0.7, gravity: -70, seed: 72, size: 1.6 });
  burst(ctx, t, { t0: 70.5, x: mx, y: my - 14, count: 26, speed: 320, life: 0.6, spread: 1.1, gravity: 300, seed: 73, size: 2 });
  const [ex, ey] = tree.dragPath(tree.DRAG[1]);
  burst(ctx, t, { t0: tree.DRAG[1], x: ex, y: ey - 20, count: 12, speed: 200, life: 0.45, spread: 1.2, gravity: 320, seed: 74, size: 1.6 });
}

export default { id: 's7', start: 64, end: 74, flame, draw, fx };
