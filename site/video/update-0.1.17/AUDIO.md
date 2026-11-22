# Audio

Score, pen and foley are synthesised in `tools/audio` (numpy, scipy; matplotlib only for `analyze.py --png`). Output goes to `assets/audio`, which git ignores. 120 s, stereo, 48 kHz, 24-bit.

```sh
python3 tools/audio/build.py          # music.wav and sfx/ (build.py music | sfx for one); about 45 s
python3 tools/audio/cues.py           # checks every scenes/*.cues.json against the library
node tools/pen.js                     # .work/pen.json, the pen per frame
python3 tools/audio/mix.py            # music + pen + cues -> assets/audio/mix.wav; about 10 s
python3 tools/audio/analyze.py assets/audio/mix.wav --png .work/audio/mix.png [--from 56 --to 70]
node tools/render.js --from 0 --to 120 --jobs 3 --out renders/film.mp4 --audio assets/audio/mix.wav
```

If any cue file or the pen file has a problem, `mix.py` lists every problem and writes nothing. Every output is a pure function of its inputs: the same cues, pen and code give the same bytes.

## `scenes/<id>.cues.json`

```json
{ "cues": [{ "t": 60.0, "sfx": "snap", "gain": 1, "pan": -0.2 }, { "t": 61.0, "sfx": "snap", "var": 3 }] }
```

- `t`: film seconds, `start <= t < end` of the scene, a multiple of 0.0625 s (an eighth of a beat). It is the moment the action happens; each sound lines its own hit point up there (table below), so it may begin a little earlier.
- `sfx`: a name from the table. `gain`: above 0, up to 2, default 1; 1 is a clear foreground action. `pan`: -1 left to 1 right, default 0; screen x maps as `(x - 960) / 960`.
- `var`: variant number, optional. Without it each cue gets a fixed variant, never the same one twice running for that sound in the scene.
- Nothing else is allowed in the file. Windows come from importing `scenes/<id>.js` alone, so a scene that does not load fails only its own cues.

The music already marks the storyboard hits (`HITS` in `grid.py`): strong ones get the kick, a low piano octave and the sub; the rest a soft marimba glint. Cues are for what the user does on screen.

## Sounds

| Name | Variants | Hit point | For |
| --- | --- | --- | --- |
| `click` | 5 | the press; a faint release 75 ms later | a pointer click |
| `double-click` | 3 | the first press; the second is 0.125 s later, also on the grid | opening a file |
| `tick` | 6 | the tk | a checkbox; a tiny tuned tink rising F6 G6 A6 C7 D7 F7 by variant, so 1-6 climbs |
| `snap` | 5 | the tock | guide snap: a woody tock, a 42 ms overshoot tock, settled |
| `slide` | 4 | where the drag starts | 0.35, 0.5, 0.75, 1.0 s; speeds up, glides, settles, drifts left to right |
| `key` | 6 | the press | key caps; 6 is the space bar |
| `peel` | 3 | where the tape comes free | 0.45, 0.6, 0.85 s of stick-slip crackle before it |
| `pop` | 5 | the pop | something appears; 520, 620, 740, 440, 880 Hz |
| `whoosh` | 4 | the peak, mid-sound | camera flight: 0.6, 0.8, 1.0, 1.25 s; odd variants travel left to right, even right to left |
| `lock` | 3 | the latch | the latch slides, then a metal click |
| `error` | 3 | the first tone | two soft tones falling |
| `paper` | 4 | the start | 1 flick, 2 slide, 3 crinkle, 4 page turn |

`assets/audio/sfx/index.json` lists variants, hit points (`anchors`, seconds into each file) and lengths.

## The pen

`mix.py` reads `.work/pen.json` (`--pen` for another), written by `node tools/pen.js`:

```json
{ "fps": 60, "frames": [{ "t": 0, "x": 724.3, "y": 616.4, "speed": 0, "pressure": 1.35, "drawing": false, "scene": "c01" }] }
```

- One frame per video frame, `t = i / fps`, through 120 s. Every frame has exactly these keys.
- `x`, `y`: the tip in screen pixels, `null` before the pen lands (then `drawing` must be false). `speed`: screen pixels per second. `pressure`: 0.55-1.35 from `lib/pen.js`. `drawing`: true while the line grows, including entry connectors.

The scratch is the paper's tooth read by the nib: a fixed noise texture laid along the stroke (one cell per 1/16 px, with scattered fibres) is read at the pen's arc length, so grain rate and brightness follow speed on their own. On top: level `sqrt(speed / 1000)` scaled by pressure, a moving low-pass from 1.8 to 8.3 kHz that darkens under pressure, a nib resonance at 3.3 kHz, a pressure-driven body at 260-700 Hz, a tick wherever the stroke turns sharply at speed, a small room, and pan `0.7 × (x - 960) / 960`. Silent whenever `drawing` is false. The bed sits at -31 LUFS, ridden up 4 dB for the first chapter, 3 dB in the breakdown and 4 dB while "Ely Office" is written, down 3 dB under the tutti.

## Music

F major, then G from bar 29. Felt piano (stiff-string partials, felt hammer, beating unisons, damper noise), bowed strings (violin, viola and cello sections from detuned bowed voices with vibrato and body resonances), harp and pizzicato (Karplus-Strong), marimba (tuned modes over a resonator), a warm sub, felt kick and brushes. Rooms are convolution: a small room, a plate for the piano and a 3.6 s hall.

| Bars | Seconds | Music |
| --- | --- | --- |
| 0 | 0-1 | near silence; a reversed felt note draws in toward the touch |
| 0-3 | 1-8 | the touch: a felt D and one held viola line under the writing, D at 1.0, E at 6.0 as the 6 becomes a 7, F at 8.0: degrees six, seven, eight |
| 4-9 | 8-20 | the theme on piano, full band; hit 12.0; marimba ticks along 14-17.5; 18.0 lands the theme |
| 10-14 | 20-30 | idle: no kick, harp offbeats, piano fragments; 26.0 opens bright; 28.0 one note stays |
| 15-18 | 30-38 | launch: marimba spins in sixteenths into 32.0; 34.0 everything stops but one chord (Reduce Motion); 36.0 back |
| 19-22 | 38-46 | the theme's answer; hits 40, 42, 43, 44.5 |
| 23-28 | 46-58 | the theme again with pizzicato; turns to D7 for G |
| 29-33 | 58-68 | smart guides in G: sixteenth marimba, staccato piano offbeats, tight kit; 66.0 Alt: the grid lets go, a free piano run |
| 34-38 | 68-78 | the theme on violins |
| 39-42 | 78-86 | breakdown: solo piano song, strings, no drums or sub |
| 43 | 86-88 | build: repeated chords, brush swell |
| 44-52 | 88-106 | the theme in octaves with violins, then its answer |
| 53-55 | 106-112 | the pull-back: drops 5 dB, crescendos on quarter chords and rising strings |
| 56-57 | 112-116 | tutti: the crown of the theme in octaves; 114.0 as "Ely Office" is written |
| 58-59 | 116-120 | G 6/9 held to the lift at 118.5, where it releases and one high D rings; the tail is silent by 119.95 |

`mix.py` ducks the music under loud foley (from -20 dBFS of foley level, 0.3 dB per dB, at most 3 dB, 30 ms attack, 0.2 s release), never in the 0.35 s after a band hit. The music is mastered to -15 LUFS; the mix to -14 LUFS integrated with true peak at most -1.5 dBTP, cut above 19 kHz before the limiter, so the AAC 320k encode in `tools/render.js` stays below -1 dBTP (it measured -1.9 dBTP). To check: `ffmpeg -i assets/audio/mix.wav -c:a aac -b:a 320k t.m4a` and `ffmpeg -i t.m4a -af ebur128=peak=true -f null -`.
