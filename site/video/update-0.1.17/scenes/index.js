// Film order: a serpentine map, the title band below.
import c01 from './c01.js';
import c02 from './c02.js';
import c03 from './c03.js';
import c04 from './c04.js';
import c05 from './c05.js';
import c06 from './c06.js';
import c07 from './c07.js';
import c08 from './c08.js';
import c09 from './c09.js';
import c10 from './c10.js';
import c11 from './c11.js';
import c12, { BAND } from './c12.js';

const COLS = 4;
const PITCH_X = 1920 + 520;
const PITCH_Y = 1080 + 520;
const MAP_W = COLS * PITCH_X - 520;
const MAP_H = 3 * PITCH_Y - 520;

const map = [c01, c02, c03, c04, c05, c06, c07, c08, c09, c10, c11].map((s, i) => {
  const row = Math.floor(i / COLS);
  const col = row % 2 ? COLS - 1 - (i % COLS) : i % COLS;
  const centre = row === 2 ? PITCH_X / 2 : 0;
  return { ...s, place: { x: col * PITCH_X + centre, y: row * PITCH_Y, s: 1, r: 0 } };
});

const band = { x: MAP_W / 2 - 960 * BAND, y: MAP_H, s: BAND, r: 0 };

export default [...map, { ...c12, place: band }];
