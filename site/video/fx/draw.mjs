// SVG for callout slabs, leaders and the margin grid.
import { BEAT, LAND, WINDOW, glide, place, seconds, settle, view } from "./plan.mjs";

const COS = Math.cos(Math.PI / 6);
const SIN = Math.sin(Math.PI / 6);
const ACCENT = "#3d7fe5";
const STONE = "#efefec";
const INK = "#1d1d1f";
const MUTED = "#6e6e73";
const W = 1920;
const H = 1080;

const clamp = (v, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, v));
const f = (v) => v.toFixed(2);
const pts = (list) => list.map(([x, y]) => `${f(x)},${f(y)}`).join(" ");

/** A grid point to screen pixels around an origin. */
const iso = (u, [ox, oy]) => (x, y, z) => [ox + (x - y) * COS * u, oy + ((x + y) * SIN - z) * u];

/** A three-tone block, like site/src/iso.ts, with a CSS tone. */
function block(p, { w, d, h }, tone) {
  const face = (list, shade) => {
    const fill = shade ? `color-mix(in oklab, ${tone}, black ${shade}%)` : tone;
    return `<polygon points="${pts(list)}" style="fill:${fill}"/>`;
  };
  return [
    face([p(0, d, 0), p(w, d, 0), p(w, 0, 0), p(w, 0, h), p(0, 0, h), p(0, d, h)], 9),
    face([p(0, d, 0), p(w, d, 0), p(w, d, h), p(0, d, h)], 9),
    face([p(w, d, 0), p(w, 0, 0), p(w, 0, h), p(w, d, h)], 20),
    face([p(0, 0, h), p(w, 0, h), p(w, d, h), p(0, d, h)], 0),
  ].join("");
}

const SLAB_U = 18;
const LEFT = { w: 4, d: 2, h: 1, x: WINDOW.x - 14 };
const RIGHT = { w: 2, d: 4, h: 1, x: WINDOW.x + WINDOW.w + 14 };

/** Points along an elbow path, cut at progress l. */
function elbow(a, b, l) {
  const corner = [b[0], a[1]];
  const first = Math.abs(corner[0] - a[0]);
  const total = first + Math.abs(b[1] - corner[1]);
  let left = total * l;
  if (left <= first) return [a, [a[0] + Math.sign(corner[0] - a[0]) * left, a[1]]];
  left -= first;
  return [a, corner, [corner[0], corner[1] + Math.sign(b[1] - corner[1]) * left]];
}

/** Which margin a callout stands in and how high. */
function stance(a, c) {
  const mid = view(a, (seconds(a, c.at[0]) + seconds(a, c.at[1])) / 2);
  const [bx, by, bw, bh] = c.box;
  const side = c.side ?? (place([bx + bw / 2, 0], mid)[0] < W / 2 ? "left" : "right");
  const edge = [side === "left" ? bx : bx + bw, by + bh / 2];
  return { side, edge, y: clamp(place(edge, mid)[1], 132, 1036) };
}

/** Corner ticks around a frame-space box. */
function ticks([x0, y0], [x1, y1]) {
  const pad = 6;
  const [l, t, r, b] = [x0 - pad, y0 - pad, x1 + pad, y1 + pad];
  const n = Math.min(16, (r - l) / 3, (b - t) / 3);
  return [
    [[l, t + n], [l, t], [l + n, t]],
    [[r - n, t], [r, t], [r, t + n]],
    [[r, b - n], [r, b], [r - n, b]],
    [[l + n, b], [l, b], [l, b - n]],
  ]
    .map((c) => `<polyline points="${pts(c)}"/>`)
    .join("");
}

/** One callout at time t, or nothing outside its life. */
function callout(a, c, t) {
  const born = seconds(a, c.at[0]);
  const gone = seconds(a, c.at[1]);
  if (t < born - LAND || t > gone) return "";
  const rise = settle((t - (born - LAND)) / LAND);
  const fall = settle((t - (gone - LAND)) / LAND);
  const alpha = rise * (1 - fall);
  const drop = SLAB_U * (1 - rise) + SLAB_U * fall;
  const { side, edge, y } = stance(a, c);
  const shape = side === "left" ? LEFT : RIGHT;
  const base = iso(SLAB_U, [0, 0]);
  const tip = side === "left" ? base(shape.w, 0, shape.h) : base(0, shape.d, shape.h);
  const origin = [shape.x - tip[0], y + drop - tip[1]];
  const p = iso(SLAB_U, origin);
  const anchor = [shape.x, y + drop];
  const top = p(0, 0, shape.h)[1];
  const cx = (p(0, shape.d, shape.h)[0] + p(shape.w, 0, shape.h)[0]) / 2;

  const now = view(a, t);
  const target = place(edge, now);
  const [bx, by, bw, bh] = c.box;
  const frame = ticks(place([bx, by], now), place([bx + bw, by + bh], now));
  const reach = settle(clamp((t - born) / (BEAT / 2)));
  const path = elbow(anchor, target, reach);
  const mark = clamp((reach - 0.85) / 0.15);
  const font = `font:600 22px 'SF Pro Text','SF Pro Display',system-ui`;
  const [mx, my] = target;
  return `<g opacity="${f(alpha)}">
    <polyline points="${pts(path)}" fill="none" stroke="${ACCENT}" stroke-width="1.5"/>
    <g opacity="${f(mark)}" fill="none" stroke="${ACCENT}" stroke-width="2">${frame}</g>
    <polygon opacity="${f(mark)}" fill="${ACCENT}"
      points="${pts([[mx, my - 5], [mx + 8.66, my], [mx, my + 5], [mx - 8.66, my]])}"/>
    ${block(p, shape, ACCENT)}
    <text x="${f(cx)}" y="${f(top - 35)}" text-anchor="middle" fill="${INK}" style="${font}">${c.en}</text>
    <text x="${f(cx)}" y="${f(top - 11)}" text-anchor="middle" fill="${MUTED}"
      style="font:300 15px 'PingFang SC',system-ui;letter-spacing:0.04em">${c.zh}</text>
  </g>`;
}

const svg = (body) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}">${body}</svg>`;

/** The callout layer of one app at time t. */
export function calloutFrame(a, t) {
  return svg(a.callouts.map((c) => callout(a, c, t)).join(""));
}

const GRID_U = 24;
const GAP = 18;
const HOLE = { x: WINDOW.x - GAP, y: WINDOW.y - GAP, x1: WINDOW.x + WINDOW.w + GAP };
const GROUND = iso(GRID_U, [W / 2, -GRID_U * 20]);

/** Whether a frame point is in the grid's margin. */
const free = ([x, y]) => x > 4 && x < W - 4 && y > 4 && y < H - 4 && !(x > HOLE.x - 4 && x < HOLE.x1 + 4 && y > HOLE.y - 4);

/** Tiles whose lifted cube stays inside the margin. */
export const TILES = (() => {
  const out = [];
  for (let i = -60; i < 160; i++) {
    for (let j = -60; j < 160; j++) {
      const corners = [GROUND(i, j, 0), GROUND(i + 1, j, 0), GROUND(i + 1, j + 1, 0), GROUND(i, j + 1, 0)];
      const up = corners.map(([x, y]) => [x, y - GRID_U]);
      if ([...corners, ...up].every(free)) out.push([i, j]);
    }
  }
  return out;
})();

/** A seeded generator, mulberry32. */
function random(seed) {
  let s = seed >>> 0;
  return () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let r = Math.imul(s ^ (s >>> 15), 1 | s);
    r = (r + Math.imul(r ^ (r >>> 7), 61 | r)) ^ r;
    return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
  };
}

/** Lifts per section: tile, landing beat and settling beat. */
export function lifts(a, seed) {
  const next = random(seed);
  const busy = [];
  const out = [];
  for (let beat = 5; beat <= 15; beat++) {
    const n = 2 + Math.floor(next() * 2);
    for (let k = 0; k < n; k++) {
      const tile = TILES[Math.floor(next() * TILES.length)];
      const until = Math.min(16, beat + 1 + Math.floor(next() * 3));
      const key = tile.join(",");
      if (busy.some((b) => b.key === key && b.until > beat)) continue;
      busy.push({ key, until });
      out.push({ tile, up: seconds(a, beat), down: seconds(a, until) });
    }
  }
  return out;
}

const LINES = (() => {
  const segs = [];
  for (let k = -60; k < 160; k++) {
    segs.push(`M${pts([GROUND(k, -60, 0)])}L${pts([GROUND(k, 160, 0)])}`);
    segs.push(`M${pts([GROUND(-60, k, 0)])}L${pts([GROUND(160, k, 0)])}`);
  }
  return segs.join("");
})();

const CLIP = `<clipPath id="m"><path clip-rule="evenodd" d="M0,0H${W}V${H}H0Z
  M${HOLE.x},${HOLE.y}H${HOLE.x1}V${H}H${HOLE.x}Z"/></clipPath>`;

/** The grid layer at time t for the given lifts. */
export function gridFrame(list, t) {
  const cubes = list
    .map(({ tile, up, down }) => {
      const rise = settle((t - (up - LAND)) / LAND);
      const sink = glide((t - (down - LAND)) / LAND);
      return { tile, h: rise * (1 - sink) };
    })
    .filter((c) => c.h > 0.001)
    .sort((p, q) => p.tile[0] + p.tile[1] - (q.tile[0] + q.tile[1]));
  const body = cubes
    .map(({ tile: [i, j], h }) => block((x, y, z) => GROUND(i + x, j + y, z), { w: 1, d: 1, h }, STONE))
    .join("");
  return svg(`<defs>${CLIP}</defs><g clip-path="url(#m)">
    <path d="${LINES}" stroke="#e7e7e4" stroke-width="1" fill="none"/>
    <g opacity="0.75">${body}</g></g>`);
}
