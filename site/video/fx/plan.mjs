// Beats, punch-ins and callouts for the six app sections.
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
export const EDIT_PLAN = resolve(here, "../edit/plan.json");

export const FPS = 30;
export const BEAT = 60 / 112;
export const LAND = 0.2;

/** The letterbox window, read from the integrator's plan. */
export const WINDOW = JSON.parse(readFileSync(EDIT_PLAN, "utf8")).window;
export const REC = { w: 1920, h: 1080 };

/** CSS cubic-bezier timing, solved by bisection. */
function bezier(x1, y1, x2, y2) {
  const at = (a, b, u) => 3 * a * u * (1 - u) ** 2 + 3 * b * u * u * (1 - u) + u ** 3;
  return (x) => {
    if (x <= 0) return 0;
    if (x >= 1) return 1;
    let [lo, hi] = [0, 1];
    for (let i = 0; i < 50; i++) {
      const mid = (lo + hi) / 2;
      if (at(x1, x2, mid) < x) lo = mid;
      else hi = mid;
    }
    return at(y1, y2, (lo + hi) / 2);
  };
}

export const settle = bezier(0.2, 0.8, 0.3, 1);
export const glide = bezier(0.65, 0, 0.35, 1);

/** Per app: section's first beat, punch-ins, callouts (recording pixels). */
export const APPS = [
  {
    app: "doc",
    start: 32,
    zooms: [
      { rect: [280, 190, 960, 540], in: [4.5, 5], out: [8, 8.5] },
      { rect: [250, 187, 900, 506.25], in: [10, 10.5], out: [13.5, 14] },
    ],
    callouts: [
      { en: "Type", zh: "离线输入", at: [5, 8], box: [385, 405, 750, 100] },
      { en: "Bold  ⌘B", zh: "加粗", at: [11, 14], box: [553, 407, 52, 28] },
      { en: "125%", zh: "缩放", at: [15, 16], box: [1828, 1054, 54, 24] },
    ],
  },
  {
    app: "table",
    start: 48,
    zooms: [
      { rect: [0, 70, 1140, 641.25], in: [4.5, 5], out: [7.5, 8] },
      { rect: [145, 110, 900, 506.25], in: [12, 12.5], out: [14.5, 15] },
    ],
    callouts: [
      { en: "=SUM", zh: "求和", at: [5, 7.5], box: [178, 92, 110, 24] },
      { en: "Fill right", zh: "向右填充", at: [6, 8], box: [229, 414, 892, 36] },
      { en: "Chart", zh: "插入图表", at: [13, 15], box: [232, 158, 726, 408] },
    ],
  },
  {
    app: "show",
    start: 64,
    zooms: [
      { rect: [920, 252, 1000, 562.5], in: [5, 5.5], out: [7.5, 8] },
      { rect: [360, 202, 1200, 675], in: [12.5, 13], out: [15.5, 16] },
    ],
    callouts: [
      { en: "Fly In", zh: "飞入", at: [5.5, 7], box: [1580, 340, 200, 30] },
      { en: "Preview", zh: "预览动画", at: [8, 10], box: [1722, 768, 158, 30] },
      { en: "Fly In", zh: "飞入", at: [13, 15], box: [425, 423, 1068, 92], side: "left" },
      { en: "Fade In", zh: "淡入", at: [15, 16], box: [585, 612, 750, 54], side: "right" },
    ],
  },
  {
    app: "pdf",
    start: 80,
    zooms: [
      { rect: [605, 360, 960, 540], in: [6.5, 7], out: [8.5, 9] },
      { rect: [480, 270, 960, 540], in: [10, 10.5], out: [12.5, 13] },
      { rect: [760, 540, 960, 540], in: [14.5, 15], out: [15.5, 16] },
    ],
    callouts: [
      { en: "Highlight", zh: "高亮", at: [7, 9], box: [822, 617, 525, 38] },
      { en: "Draw", zh: "手写签名", at: [10, 13], box: [727, 478, 466, 190] },
      { en: "Sign", zh: "签名", at: [15, 16], box: [1148, 848, 184, 78] },
    ],
  },
  {
    app: "tree",
    start: 96,
    zooms: [
      { rect: [960, 120, 960, 540], in: [4.5, 5], out: [6, 6.5] },
      { rect: [0, 517.5, 1000, 562.5], in: [6.5, 7], out: [8.5, 9] },
      { rect: [0, 140, 900, 506.25], in: [9.5, 10], out: [12.5, 13] },
    ],
    callouts: [
      { en: "Stage hunk", zh: "暂存此块", at: [5, 6.5], box: [1805, 168, 98, 26] },
      { en: "Commit", zh: "提交", at: [7, 9], box: [508, 1013, 129, 29] },
      { en: "History", zh: "提交历史", at: [10, 13], box: [2, 170, 640, 605] },
    ],
  },
  {
    app: "mark",
    start: 112,
    zooms: [
      { rect: [603, 150, 960, 540], in: [8.5, 9], out: [10.5, 11] },
      { rect: [603, 530, 960, 540], in: [10.5, 11], out: [11.5, 12] },
    ],
    callouts: [
      { en: "Math", zh: "公式", at: [9, 10.5], box: [652, 291, 862, 245] },
      { en: "Mermaid", zh: "流程图", at: [11, 12], box: [672, 880, 820, 100] },
      { en: "Source  ⌘2", zh: "源码", at: [13, 15], box: [1845, 1056, 26, 22] },
    ],
  },
];

/** Global seconds of a section beat. */
export const seconds = (a, beat) => (a.start + beat) * BEAT;

/** Global frames of bars 2–4 of a section. */
export function frames(a) {
  const first = Math.ceil(seconds(a, 4) * FPS - 1e-6);
  const end = Math.ceil(seconds(a, 16) * FPS - 1e-6);
  return { first, count: end - first };
}

const FULL = [0, 0, REC.w, REC.h];
const mix = (a, b, p) => a.map((v, i) => v + (b[i] - v) * p);

/** Eased view-rect moves: [t0, t1, from, to, ease]. */
function moves(a) {
  const out = [];
  a.zooms.forEach((z, i) => {
    const prev = a.zooms[i - 1];
    const panned = prev && prev.out[0] === z.in[0] && prev.out[1] === z.in[1];
    if (!panned) out.push([seconds(a, z.in[0]), seconds(a, z.in[1]), FULL, z.rect, settle]);
    const next = a.zooms[i + 1];
    const pans = next && next.in[0] === z.out[0] && next.in[1] === z.out[1];
    out.push([seconds(a, z.out[0]), seconds(a, z.out[1]), z.rect, pans ? next.rect : FULL, glide]);
  });
  return out;
}

/** The recording rect shown in the window at time t. */
export function view(a, t) {
  let rect = FULL;
  for (const [t0, t1, from, to, ease] of moves(a)) {
    if (t >= t1) rect = to;
    else if (t > t0) return mix(from, to, ease((t - t0) / (t1 - t0)));
    else break;
  }
  return rect;
}

/** Recording pixel to frame pixel through a view rect. */
export function place([rx, ry], [vx, vy, vw, vh]) {
  return [WINDOW.x + ((rx - vx) / vw) * WINDOW.w, WINDOW.y + ((ry - vy) / vh) * WINDOW.h];
}

/** The punch-ins in the integrator's terms, global seconds. */
export function zoomList(a) {
  const r = (v) => Math.round(v * 10000) / 10000;
  return a.zooms.map((z) => ({
    t0: r(seconds(a, z.in[0])),
    t1: r(seconds(a, z.out[1])),
    rect: z.rect,
    in: z.in.map((b) => r(seconds(a, b))),
    out: z.out.map((b) => r(seconds(a, b))),
  }));
}
