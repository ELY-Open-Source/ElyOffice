# Audio

Music and foley are synthesised in `tools/audio` (numpy, scipy). Output is in `assets/audio`, which git ignores.

```sh
python3 tools/audio/build.py          # music.wav and sfx/ (build.py music | sfx for one)
python3 tools/audio/cues.py           # checks every cue file; names loud cues off the eighth-beat grid
python3 tools/audio/mix.py            # scenes/*.cues.json + music -> assets/audio/mix.wav
python3 tools/audio/analyze.py assets/audio/mix.wav --png /tmp/mix.png
```

If any cue file has a problem, `mix.py` lists every problem and writes nothing. Hits land on the 120 BPM grid: beat 0.5 s, bar 2 s.

## `scenes/<id>.cues.json`

```json
{
  "cues": [
    {"t": 2.0, "sfx": "match-strike", "gain": 1.0, "pan": -0.5, "var": 1},
    {"t": 4.0, "sfx": "thud", "gain": 0.9, "pan": -0.17}
  ],
  "flame": [[2.0, 0.0, -0.5], [2.05, 1.0, -0.5], [7.6, 0.5, 0.0], [8.0, 0.5, 0.0]]
}
```

- `t`: film seconds (not scene seconds), inside the scene's window. It is the moment the action happens; the library aligns each sound's own hit point there (the match flares, a whoosh peaks, a glyph pops), so a sound may begin a little earlier.
- `sfx`: a name from the list below. `gain`: above 0, up to 2, default 1; 1 is a clear foreground action. `pan`: -1 left to 1 right, default 0; screen x maps as `(x - 960) / 960`.
- `var`: variant number, optional. Without it each cue picks a fixed variant, never the same one twice running for that sound in the scene.
- `flame`: the crackle under the flame, as `[t, energy]` or `[t, energy, pan]`. Energy is 0 to 1: about the drawing's `energy` plus its `roar`, capped at 1. Points are joined by straight lines; equal times make a jump. All scenes join into one line, silent before the first point and after the last. At each scene boundary the flame is at rest, energy 0.5 and pan 0, so each scene starts and ends its line there; only s1 starts at the strike (2.0, energy 0) and s9 ends at 0.

## Sounds

| Name | Variants | For |
| --- | --- | --- |
| `match-strike` | 3 | scrape then flare; hit point is the flare |
| `scrape` | 3 | a match dragged on the box, no flame |
| `ignite` | 3 | a soft woof as something catches (1 small, 3 big) |
| `roar` | 2 | the flame roars as a thing comes alive |
| `catch` | 6 | a glyph catches: pff, pop, fizz |
| `sizzle` | 3 | burning away (0.9, 1.3, 1.8 s) |
| `spark` | 6 | a spit from the flame |
| `snuff` | 2 | snuffed out |
| `ember` | 3 | an ember winks |
| `whoosh` | 5 | a hop or leap (0.3 to 0.75 s); hit point is the peak |
| `stretch`, `squash`, `pop` | 3, 3, 6 | the flame stretches tall, squashes, or a round pop |
| `thud` | 5 | a landing (1 light to 5 heavy) |
| `pencil`, `charcoal` | 8, 8 | one stroke, 0.12 to 0.5 s |
| `scribble` | 4 | a burst of strokes, 0.8 s |
| `rustle` | 5 | paper handled (0.35 to 1.2 s) |
| `fold`, `tear`, `fray` | 3, 3, 3 | crease; rip (0.6, 0.9, 1.3 s); a thread giving |
| `slide` | 4 | dragged across the page (0.35 to 1 s) |
| `flick` | 4 | kicked or flung |
| `cardboard` | 4 | dropped into a box |
| `splash` | 4 | ink lands |
| `snap` | 3 | a gap snaps shut |

The crackle grains inside the fire sounds and the flame beds are band-limited to 300 Hz-8 kHz, so a pop never becomes a full-scale digital click.

## Music map

| Bars | Seconds | Music |
| --- | --- | --- |
| 0 | 0-2 | near silence; one reversed tine draws in |
| 1 | 2-4 | the strike: kalimba sparkle, reed organ swell |
| 2-3 | 4-8 | nylon picking wakes; hit 6.0; brushes swell |
| 4-9 | 8-20 | theme in D, full band; hit 16.0 |
| 10-15 | 20-32 | whistled bridge; stop on 24.0; a kalimba note per glyph 26-27.5 |
| 16-20 | 32-42 | theme with whistle; hits 36.0, 40.0 |
| 21-23 | 42-48 | breakdown, then a brush roll into 48.0 |
| 24-31 | 48-64 | theme; hits 52, 56; walking bass; silent 60.0-60.75 (see below) |
| 32-40 | 64-82 | lift to E; hits 66, 76, 80; silent 70.0-71.0 (see below) |
| 41-44 | 82-90 | montage climb; music silent 85.75-85.96, the fire fills it; tutti 86.0; held chord; ember 88.5; fades from 89.3 |

The music bus goes silent, reverb and all, at two moments:

- 60.0 (s6, the source sheet is torn away): the music stops at 60.0 with the snap. Only one soft kalimba D with a touch of glockenspiel stays, for the saved value that keeps glowing. The band comes back at 60.75, as the flame lands by the value and relights.
- 70.0 (s7, the flame snuffs the graph): the music stops at 70.0 with the snuff. Only a bass slide stays, about 0.5 s long, under the titles stretching into the graph's width. The band comes back at 71.0, when the drag starts.

Picture goes black at 89.3. Music and mix fade from 89.3: about -30 dB by 89.6, -50 dB by 89.8, and silent before 89.96, so the sound is clean well before 90.0 and nothing is cut off.

`mix.py` ducks the music under loud foley: from -18 dB of foley level, 0.3 dB per dB above that, at most 3 dB, with a 30 ms attack and a 0.2 s release. It never ducks in the first 0.35 s after a band hit, so the hits keep their punch. Under a high-energy flame it also cuts up to 4.5 dB of the music's 1.8-5.5 kHz band to leave the crackle its room. The mix is -14 LUFS with true peak at most -2 dBTP. The delivery limit is -1 dBTP after encoding, and the AAC 320k encode in `tools/render.js` raises peaks, so the master keeps 1 dB of headroom for it. The master also cuts everything above 19 kHz before the limiter. The encoder drops that band, and dropping it from a sharp transient made the peak jump 1.6 dB at 76.0; with the cut, encoding adds about 0.2 dB. To check, encode with `ffmpeg -i assets/audio/mix.wav -c:a aac -b:a 320k t.m4a` and measure with `-af ebur128=peak=true`; the encoded mix measures -2.1 dBTP.
