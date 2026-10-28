// The model at any moment, as pieces and texts.
import { model } from "../../src/model.ts";
import { LAMP, type Box, file, lamp, length, props, screens, tiles, trips } from "./parts.ts";
import { BEAT, LAND, begun, glide, toward } from "./time.ts";
import { slabs } from "./turn.ts";

/** A box, its landing progress, and whether it is dark. */
export interface Piece {
  box: Box;
  land: number;
  dark: number;
}

/** Everything one frame shows, before the camera. */
export interface Moment {
  pieces: Piece[];
  /** The page and texts: 0 light, 1 dark. */
  dark: number;
  /** The model's turn about the desk centre, radians. */
  angle: number;
  texts: Record<string, number>;
}

/** The desk grows outward from this tile. */
export const SEED = [8, 5];
const ring = (b: Box) => Math.abs(b.x - SEED[0]) + Math.abs(b.y - SEED[1]);
const angle = (b: Box) => Math.atan2(b.y - SEED[1], b.x - SEED[0]);

/** Tiles land faster and faster: count grows as time squared. */
export const DESK_END = 12;
const byRing = tiles.map((_, i) => i).sort((a, b) => ring(tiles[a]) - ring(tiles[b]) || angle(tiles[a]) - angle(tiles[b]));
const tileBeats: number[] = [];
byRing.forEach((tile, n) => {
  tileBeats[tile] = Math.round(DESK_END * Math.sqrt(n / (tiles.length - 1)) * 4) / 4;
});
const DESK_DONE = DESK_END + 1;

/** Beat each app starts on; its blocks follow on sixteenths. */
const partBeats = [16, 18, 20, 22, 24, 26, 28];

/** Fades in by the first beat; gone at the second. */
const TEXTS: Record<string, [number, number]> = {
  title: [24, 32],
  offline: [144.5, 147.5],
  open: [148.5, 151.5],
  local: [152.5, 155.5],
  name: [160, 168.1],
  version: [160.5, 168.1],
  url: [161, 168.1],
};

/** Bar 35 turns back from the montage's -90°. */
export const TURN: [number, number] = [136, 140];

/** The dark wave: from the lamp, one step per sixteenth. */
export const WAVE = 140;
export const NIGHT = 144;
const lampFar = Math.max(...tiles.map((t) => Math.hypot(t.x + 0.5 - LAMP.centre[0], t.y + 0.5 - LAMP.centre[1])));
function flip(b: Box): number {
  const d = Math.hypot(b.x + b.w / 2 - LAMP.centre[0], b.y + b.d / 2 - LAMP.centre[1]);
  return WAVE + Math.min(15, Math.floor((16 * d) / lampFar)) / 4;
}

function landed(t: number, box: Box, beat: number, out: Piece[]) {
  if (begun(t, beat)) out.push({ box, land: toward(t, beat), dark: t >= flip(box) * BEAT ? 1 : 0 });
}

/** A trip's file: dropped at its start, then hopping. */
function travel(t: number, trip: (typeof trips)[number], out: Piece[]) {
  if (!begun(t, trip.lands)) return;
  const step = length(trip.path) / trip.hops.length;
  const s = trip.hops.reduce((sum, beat) => sum + step * toward(t, beat), 0);
  out.push({ box: file(trip, s), land: toward(t, trip.lands), dark: 0 });
}

/** The model at `t`, plus a plate's screen. */
export function moment(t: number, screen?: { app: string; beat: number }): Moment {
  const pieces: Piece[] = [];
  const wave = t >= (WAVE - 0.01) * BEAT && t < NIGHT * BEAT;
  if (t < DESK_DONE * BEAT || wave) tiles.forEach((tile, i) => landed(t, tile, tileBeats[i], pieces));
  else landed(t, model[0].blocks[0], 0, pieces);
  model.slice(1).forEach((part, p) => part.blocks.forEach((b, i) => landed(t, b, partBeats[p] + i / 4, pieces)));
  lamp.forEach((b, i) => landed(t, b, LAMP.beat + i / 4, pieces));
  for (const trip of trips) travel(t, trip, pieces);
  for (const prop of props) {
    prop.blocks.forEach((box, i) => {
      if (begun(t, prop.bar * 4 + i / 4)) pieces.push({ box, land: toward(t, prop.bar * 4 + i / 4), dark: 1 });
    });
  }
  if (t >= TURN[0] * BEAT - 1e-6 && t < 138 * BEAT) {
    slabs.forEach((box, i) => pieces.push({ box, land: 1 - toward(t, 136.5 + i / 4), dark: 0 }));
  }
  if (screen) {
    const box = screens[screen.app];
    if (!box) throw new Error(`no screen for ${screen.app}`);
    landed(t, box, screen.beat, pieces);
  }
  const texts: Record<string, number> = {};
  for (const [name, [from, to]] of Object.entries(TEXTS)) {
    if (screen || t >= to * BEAT) continue;
    const out = to < 168 ? 1 - toward(t, to, LAND, glide) : 1;
    texts[name] = toward(t, from, LAND, glide) * out;
  }
  const turn = t < TURN[0] * BEAT || t >= TURN[1] * BEAT ? 0 : 1 - glide((t / BEAT - TURN[0]) / (TURN[1] - TURN[0]));
  return { pieces, dark: t >= NIGHT * BEAT ? 1 : 0, angle: (-Math.PI / 2) * turn, texts };
}
