# Engine contract

Node and Skia (`@napi-rs/canvas`), no browser. 1920×1080, 60 fps, 120 BPM (beat 0.5 s, bar 2 s). Frame n is a pure function of n: no `Math.random`, no clock, no state carried between calls; randomness from `k.hash`. Module-level memo of pure results (glyphs, pen tables, frozen scenes) is allowed and used.

## Commands

```text
npm install                                   once
node tools/check.js                           the gate (below); run before handing back
tools/preview.sh c07 60 61 62                 your window ±1 s at --jobs 3 -> renders/preview/c07/c07.mp4,
                                              c07-contact.png (≤48 tiles), and PNGs at the seconds given
node tools/frame.js DIR 60 61.5               PNG frames at seconds (t060.00.png ...)
node tools/render.js --from 0 --to 120 --jobs 3 --out renders/film.mp4 [--audio a.wav] [--contact sheet.png]
node tools/pen.js                             .work/pen.json for the score
```

The machine is shared: `--jobs 3` at most. Drawing costs 3–13 ms a frame (the whole sheet at 119 s is 13 ms); a 12 s window renders in about 6 s.

## Files

```text
lib/time.js       W H FPS BEAT BAR LENGTH TOUCH(1.0) LIFT(118.5), ease, keys, spring, span, lerp, clamp, beat, onGrid
lib/rand.js       hash, rnd, signed, noise1, noise2, fbm (seeded, stateless)
lib/geom.js       polyline length, slice, resample, tangents, cubic, bridge (connectors), densify
lib/line.js       join(strokes) -> one polyline with connectors; marks(strokes) -> lengths
lib/pen.js        pen tables, draw times, speed, pressure
lib/tether.js     the cursor spring
lib/path.js       the film's line at frame n: frozen past scenes + the live one
lib/camera.js     rest cameras, flights, world->screen matrix
lib/paper.js      paper, tooth, vignette
lib/draw.js       bucketed variable-width stroking, wet colour, bead
lib/film.js       Film.frame(n) -> RGBA buffer
lib/world.js      scene table, placement transforms, k.sheet / k.prev / k.next
lib/kit/          the kit (shapes, svg, icon, window, text)
lib/glyph/        font skeletons: raster, thin, graph, refine, assemble, shape
scenes/index.js   chapter order and placement (integrator owns it)
scenes/cNN.js     one module per chapter; helpers as scenes/cNN-*.js; cues in scenes/cNN.cues.json
tools/            render, worker, frame, preview.sh, pen, check, rules, fonts.py (one face out of a .ttc)
```

A chapter agent edits only `scenes/cNN.js`, `scenes/cNN-*.js` and `scenes/cNN.cues.json`.

## World and camera

One infinite sheet. Each chapter draws on its own panel in panel-local units: x 0–1920, y 0–1080, the frame a viewer sees when the camera rests on it. `scenes/index.js` places c01–c11 on a serpentine grid, 4 columns, pitch 2440 × 1600 (520 gutters), the third row centred under the first two, `place: { x, y, s: 1, r: 0 }`; c12 is a title band under the map, placed at `s: 4` (`BAND` in `scenes/c12.js`):

```text
c01 c02 c03 c04
c08 c07 c06 c05
  c09 c10 c11
 [ c12 title  ]
```

A scene never assumes where it sits. The kit tells it, in its own panel units: `k.sheet` `{ x, y, w, h }` (bounding box of every panel), `k.prev` and `k.next` (`[x, y]` centre of the neighbouring panels, `null` at the ends). Point your exit at `k.next`.

Camera: by default it rests on the whole panel, `{ x: 960, y: 540, w: 1920, r: 0 }`. `camera(t, k)` may return `{ x, y, w, r }` (centre, visible width, rotation; panel units) or `null`. Between chapters the engine flies over `[start − 0.25, start + 0.75]`, eased `inOut`, along an arc bowed 0.14 × distance to one side, with a hop that widens the view up to 1.55× mid-flight so both panels show. Both ends keep moving: it leaves the previous chapter's camera as it is at that moment (held from its end) and lands on yours as it is at that moment (held before your start), so a camera already moving at the landing carries straight on. Make `camera()` continuous over the flight.

## The one line

```js
export default {
  id: 'c07', start: 58, end: 68, app: 'show',
  strokes(t, k) { return [/* stroke, ... */]; },
  pen(t, k) { return 0; },        // drawn length of this scene's own path, panel units
  cursor(t, k) { return null; },  // optional { x, y }
  camera(t, k) { return null; },  // optional
};
```

`app` is one of `suite doc table show pdf mail tree mark`.

A stroke: `{ pts: [[x, y], ...], w = 1, alpha = 1, show: [a, b], app = null, link = 1 }`.

- `show` keeps the part of the stroke between fractions a and b of its length. A stroke is on the path only while `b − a > 1e-6`, `alpha > 0.001`, and its kept length is over 0.5. An object leaves by shrinking `show` (`k.leave`), so its line draws back into the line around it. A stroke that has not appeared yet uses `show: [0, 0]`.
- `w` scales pressure (guides 0.45, emphasis 1.4). `alpha` fades toward the paper colour (opaque blend, never transparency). `app` overrides the scene's wet-ink colour. `link` scales the opacity of the connector that leads into this stroke.
- The number and order of strokes may not change within a scene; geometry, `show`, `alpha`, `w` may.
- Other fields (`g`, `ch` from `k.text`) are carried and ignored.

Joining (`lib/line.js`): visible strokes are densified to 10-unit steps and joined in order; between consecutive visible strokes a connector, a cubic from the end tangent of one to the start tangent of the next, handles `clamp(0.38 d, 3, 900)`, drawn at `w 0.35`, `alpha 0.45 × link`. Its length counts toward your pen. `k.marks(strokes)` gives the exact same lengths: `{ list: [{ start, end }], total }` per stroke, path units.

Between scenes the engine draws an entry connector from the previous scene's last pen point (with its tangent) to your first visible stroke's start (or to `cursor` if nothing is visible, else the panel centre), over `[start, start + 0.75]` eased `inOut`, `w 0.55`, `alpha 0.7`, landing handle at most 260 units, shortened to 35 % as your first stroke turns across the approach, so it arrives with a short hook rather than a loop. Your content draws only after it lands.

Pen:

- `pen(t, k)` is the drawn length. It never decreases. It is 0 until `start + 0.75` (c01: until `TOUCH` 1.0). It is sampled once per frame into a table (running max); the path drawn is its first `min(pen, length)` units. Keep `pen()` cheap: compute lengths once at module load with `kit.marks(...)` (import `{ kit }` from `../lib/kit/index.js`), and key with `k.keys`.
- Every point remembers the time the pen first reached its arc length; that gives wet colour and pressure. A point appearing by `show` alone, after the pen has passed, counts as drawn at that earlier time.
- The path is complete when the drawn length is within 0.5 of its length. When a stroke appears later, raise `pen` so the pen draws it.
- `pen` may exceed the current length when objects leave; it may not exceed the longest path the scene has had so far (checked).

Cursor: when the path is complete and `cursor(t, k)` returns a point, a tether is drawn from the drawn end to the cursor: a cubic whose end trails a spring copy of the cursor (stiffness 170, damping 13, simulated over the last 36 frames), `w 0.75`, fresh app colour. The bead sits on the cursor, so the pen is the pointer: it drags, clicks and holds. Clicks are drawn by the scene (`k.ripple`). A scene's last pen point at `end` (the tether's end if there is one) is where the next entry connector leaves from, with the tangent there.

After a scene ends, its state at `end` is frozen on the sheet for the rest of the film (the finale sees every panel as it was left). Leave a composed final state.

## Line, ink and pen

- Width on screen: `3.1 × w × pressure × place.s × zoom` px, at least 0.7 px, quantised to 0.12 px. `pressure = clamp(1.35 − speed / 2600, 0.55, 1.35)`, speed in panel units per second at the moment the point was drawn, from the pen table over ±3 frames. The first 40 units of c01 taper in; the last 40 units taper out over `[LIFT − 0.3, LIFT]`.
- Wet ink: a point younger than 1.2 s is drawn in its app colour blended to ink by smoothstep of age / 1.2. Ink `#1C1F26`; ElyDoc `#3B6FE0`, ElyTable `#1E9E61`, ElyShow `#E8692E`, ElyPDF `#D5413A`, ElyMail `#7A5AF0`, ElyTree `#C98A1A`, ElyMark `#2A9BB0`, suite is ink.
- The pen tip is a 7 px bead in the live scene's app colour, from `TOUCH` until `LIFT`, shrinking over the last 0.3 s.
- Paper `#F3F0E8`; a 512-unit tooth tile fixed to the sheet (it moves with the camera, fading out as the view widens below 0.8× zoom); a static 7 % vignette.

## The kit `k`

Passed to `strokes`, `pen`, `cursor`, `camera`; also importable as `{ kit }` from `lib/kit/index.js` (without `sheet`, `prev`, `next`). Primitives return one stroke, starting where a hand would start; composites return arrays. Every option object accepts `w`, `alpha`, `show`, `app`.

```text
k.keys(t, [[t0, v0], [t1, v1, ease = 'inOut'], ...])   numbers or arrays; holds outside
k.ease.{linear, in, out, inOut, back, snap, sine}(u)
k.spring(tau, amount, freq = 13, damp = 6)             damped cosine, 0 before tau 0
k.span(t, a, b)  k.lerp(a, b, u)  k.clamp(v, a = 0, b = 1)  k.beat(n) = n × 0.5
k.hash(...parts) -> [0, 1)
k.stroke(pts, o)                                        a stroke from points
k.line(x0, y0, x1, y1, o)
k.poly(points, { closed })
k.rect(x, y, w, h, { r })                               starts after the top-left corner, clockwise, closed
k.circle(x, y, r, { from = −π/2, to = from − 2π })     top, counter-clockwise on screen
k.arc(x, y, r, from, to, o)                             radians, y down
k.check(x, y, size, on) -> [box, tick]                  on: boolean or 0..1 (tick drawn that far)
k.ripple(x, y, u, { r = 14 }) -> stroke                 click ring; grows, then draws back; hidden at u ≤ 0, u ≥ 1
k.morph(a, b, u) -> stroke                              arc-length correspondence, a's other fields
k.move(strokes, { x, y, s, r, ox, oy }) -> strokes      scale and rotate about (ox, oy), then translate
k.leave(strokes, u) -> strokes                          retracts from the last stroke backwards; u 0..1
k.text(str, { x, y, size = 32, align = 'left', font = 'sans', leading = 1.35, w, app, link = 0.5 }) -> strokes
k.measure(str, { size, font, leading }) -> { w, h }
k.icon(app, x, y, size, o) -> strokes                   centre x, y; size is the icon box (192 source units)
k.window(x, y, w, h, { chrome = 'windows', title, titleSize = 28, bar = 56, w, app }) -> strokes
k.length(stroke)
k.marks(strokes) -> { list: [{ start, end }], total }
k.sheet  k.prev  k.next
```

Text:

- `y` is the baseline; `\n` starts a new line. Fonts: `sans` Noto Sans CJK SC Light, `serif` Noto Serif CJK SC Light, `jp` Noto Sans CJK JP Light (Japanese glyph forms), `mono` JetBrains Mono (Latin only: CJK in `mono` falls back to another face), `brush` LXGW WenKai (`../update-0.1.16/assets/fonts`). The SC and JP faces are taken out of the system `.ttc` collections into `.work/fonts/` on first use (`tools/fonts.py`, needs Python fontTools).
- Glyphs are rendered at 240 px, thinned (Zhang-Suen and a simple-point pass), traced to a graph; spurs pruned, junctions re-centred from incoming tangents, ends extended to the outline, edges joined through junctions by good continuation, smoothed with corners kept, simplified to 0.3 px. Strokes run left to right or top to bottom; loops start at the top, counter-clockwise; components go column by column, top to bottom, horizontal before a crossing vertical in CJK, dots after stems in Latin. Cached in `.work/glyphs/v4/<font>/<codepoint>.json`; delete that folder to rebuild.
- Each stroke carries `g` (glyph index) and `ch`. A closed loop's start is moved to the point nearest the strokes before and after it. Connectors inside a text get `link 0.5`, so text stays crisp while the line stays whole.
- Default weight `min(1.1, 0.62 + size / 200)`. Words on screen must be at least 28 px tall at the zoom they are read at. `…` is set as three baseline dots.

Icons (`doc table show pdf mail tree mark`) are parsed from `assets/icons/ely-*.svg` (shadow groups dropped) and redrawn by a per-app recipe: body outlines, the folded corner open, thin bars as centre lines, Git nodes as rings with branches clipped at them, the ElyShow stand clipped at the board.

Windows: outline (radius windows 8, mac 14, linux 12), title-bar rule at `bar`, then controls (windows: minimise, maximise, close on the right; mac: three circles on the left; linux: three ringed buttons on the right), then the title (left on Windows, centred elsewhere).

## Sound out

`node tools/pen.js` writes `.work/pen.json`: `{ fps, frames: [{ t, x, y, speed, pressure, drawing, scene }] }`, screen pixels, `x`/`y` null before the pen lands. Cues per scene in `scenes/<id>.cues.json`: `{ "cues": [{ "t": 60.0, "sfx": "snap", "gain": 1, "pan": 0 }] }`, global seconds, inside the scene's window, on the eighth-beat grid (multiples of 0.0625 s).

## Checks

`node tools/check.js` fails on any of: scene windows not contiguous from 0 to 120; `pen` decreasing, non-zero before `start + 0.75` (c01: before 1.0), or past the longest path so far; stroke count changing within a scene; a non-finite point or a `show` outside 0..1; points on the line at 1.0 s or none just after; `pen` moving or a `cursor` present after 118.5 s; a cue outside its scene, off the grid or without `sfx`; frames that differ between ascending order in one process and descending order in another; any `.js`, `.mjs`, `.py` or `.sh` file here over 500 lines, a comment over ten words, or a block comment over several lines. `scripts/check.sh` at the repo root checks tracked files only, so run it after the files are added.
