# Engine

Node and Skia (`@napi-rs/canvas`), not a browser. A frame is `Film.frame(n)`: the page is drawn, the flame's light falls on it, then the fire is drawn on top. Workers render contiguous frame ranges into H.264 segments; `tools/render.js` joins them and adds the mix.

```sh
npm install
node tools/render.js --from 0 --to 8 --audio assets/audio/mix.wav --out renders/s1.mp4 --contact renders/s1-contact.png
node tools/render.js --out renders/film.mp4 --audio assets/audio/mix.wav   # every scene so far
node tools/render.js --to 90 --wrap 0-8 --out renders/bench.mp4            # load test: s1 looped to 90 s
node tools/frame.js renders/frames 48 144                                    # single frames as PNG
node tools/check.js                                                          # determinism and handoffs
```

The font is LXGW WenKai Regular v1.530 (`assets/fonts/LXGWWenKai-Regular.ttf`, ignored; licence in `OFL-LXGWWenKai.txt`), from the project's GitHub release.

## Time

24 fps. Drawings change on twos: `draw` gets `t = 2⌊n/2⌋/24` and a kit seeded by the drawing `⌊n/2⌋`. Fire moves on ones: `flame`, `fx` and `lights` get `t = n/24`. Seconds are global, as in the storyboard. Beat 0.5 s, bar 2 s.

Frame n is a pure function of n. No `Math.random`, no clock, no state carried between calls. Randomness comes from `lib/rand.js` (`rnd`, `signed`, `noise1`, `hash`) or `ink.seed()`. Anything that must stay put across drawings (a front, a path) uses a fixed seed; anything drawn boils by itself.

## A scene

One module in `scenes/`, listed in order in `scenes/index.js`.

```js
export default {
  id: 's2', start: 8, end: 20,
  flame(t) { return { x, y, h, lean, squash, energy, curl, ember, roar, alpha, pool, glow }; },
  draw(ctx, t, ink) {},   // the page, on twos, lit by the light pass
  fx(ctx, t, ink) {},     // optional: emissive, on ones, drawn after the light
  lights(t) { return [{ x, y, r, i, c: [1, 1, 1] }]; },   // optional extra fire
  ambient(t) { return 0; },  // optional: light everywhere, 0 to about 1
};
```

Sound cues are the score's, in `scenes/<id>.cues.json`; put actions on the beat grid and tell the score which moved.

`flame(t)` returns only what differs from `REST` (`lib/flame.js`). Fields: `x, y` the base where the flame touches paper; `h` height; `lean` radians, positive to the right; `squash` below 1 squashes, above 1 stretches; `energy` licks and embers; `curl` 0 to 1 bends it into a question mark; `ember` 0 to 1 shrinks it to a coal (thinking); `roar` 0 to 1 flares it; `alpha` presence; `pool` light radius; `glow` light strength; `sparks` scales how many embers it sheds (1 by default). The pool follows the flame.

Handoff: at `start` and on the last frame the flame is exactly `REST` (960, 610, h 140, upright, pool 520). `tools/check.js` fails otherwise (the film's first start and, once scenes reach 90 s, its last end excepted). Before the first scene there is no fire; `render.js` refuses frames past the last scene unless `--wrap` loops them.

## Acting

`lib/act.js`: `hop(t, { t0, t1, from, to, height, pre, post })` gives anticipation squash, stretched arc, landing squash and settle, or `null` outside its window; `first(...)` takes the first non-null; `blend(a, b, u)`. `lib/time.js`: `keys(t, [[t, v, ease], ...])` for numbers or arrays, `span`, `ease`, `spring`. `lib/embers.js`: `burst(ctx, t, { t0, x, y, count, speed, angle, spread, gravity, life, seed })` for sparks in `fx`.

## The kit

`ink` is bound to the context and seeded per drawing:

- `ink.line(points, o)`, `ink.curve(points, o)` (Catmull-Rom through the points): a boiling stroke. `o.width`, `o.style` `'ink'` or `'charcoal'`, `o.color` RGB, `o.alpha`, `o.boil` jitter in px, `o.overshoot`, `o.taper [in, out]`, `o.pressure`, `o.from`/`o.upto` 0 to 1 to draw part of it, `o.head` a round pen tip at `upto`, `o.closed`, `o.id` for a seed that ignores call order.
- `ink.shape(contours, o)` outlines closed contours; `ink.hatch(shape, { gap, angle, width, style, reach })`; `ink.wash(shape, { color, alpha, bleed })` watercolour that multiplies into the paper; `ink.smudge(x, y, r, o)`.
- `ink.text(str, { x, y, size, align, style, fill: 'hatch' | 'solid' | 'none', upto })` writes WenKai outlines retraced with boiling strokes; below 44 px it fills solid without outline. `ink.glyphs(str, o)` lays out without drawing (contours and boxes per glyph) and `ink.write(list, o)` draws such a list, so a scene can burn, move or hide single glyphs.
- `ink.lift(height, [x, y], paint)` draws `paint(kit)` as an object above the paper: it casts a soft shadow away from the flame.
- `ink.light` is the flame's light for this drawing.

Colours are RGB arrays; washes in an app's colour, lines in `ink.INK` or `ink.CHARCOAL`. Only what an app shows is written; no labels, no boxes in boxes.
