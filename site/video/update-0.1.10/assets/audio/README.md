# Audio

Everything here is synthesized in Python (numpy, scipy): no samples, soundfonts or existing melodies.

## Music: `music.wav`

48 kHz, 24-bit stereo, exactly 90.000 s (4,320,000 samples), 120 BPM 4/4 (bar = 2 s), about −15 LUFS integrated, true peak ≤ −2 dBTP. It's future-funk bubblegum pop in E major that lifts a whole step to F# major for the peak chorus.

Harmony: the "royal road" IVmaj9–V13–iii11–vi9 (Amaj9 B13 G#m11 C#m9), with a ii–iii–IV–V answer (F#m9 G#m11 Amaj9 B13) in section B. Tonic Emaj9 arrives only on the accent hits at 5.0 and 47.0. C#m9 to C#7sus pivots into F# at 60, where it resolves to F#maj9 at the 78 unison hit.

Hook: a chopped vocal-formant synth (saw and pulse through morphing vowel formants), doubled an octave up by a glass sine, with FM gloss plucks on the accents. Bar 1 is a tresillo cascade, E5–C#5–B4 then C#–B–G#–E. Bar 2 answers upward, F#–G#–B–A–F#. Bars 3 and 4 repeat the pair a step lower.

| Seconds | Section |
| --- | --- |
| 0–1 | Near silence, then a glassy shimmer swell |
| 1–4 | Bubble plinks, a soft pad, a muffled kalimba hint of the hook |
| 4–5 | Riser into 5.0: an Emaj9 accent hit (stab, glock, clap, boom) |
| 5–11.5 | Finger snaps on 2 and 4, kalimba arpeggio, bouncy sub bass, EP chords, shaker; snare roll and riser |
| 11.5–12 | Gap: the bed is gated out and only a dry two-note vocal pickup remains |
| 12–26 | Drop, section A: four-on-the-floor, clap, open hats, rubbery funk bass, glossy stabs, the hook; kick-keyed pumping |
| 26–41.4 | Section B: lighter at first (snaps, EP comping, kalimba answers to long vocal notes), refilling by 34 |
| 41.4–42.6 | "Offline": the whole mix low-passes to about 520 Hz and drops 3.5 dB, then pops open with a rising bloop and swoosh |
| 42.6–44 | Back in the groove |
| 44–47 | Build: snare roll, rising glock arpeggio, synth filter opening, riser |
| 47 | Impact (boom, crash, Emaj9 stab) |
| 47–50 | Half time |
| 50–60 | Section C: the hook with a marimba counter-arpeggio and a whistle counter-melody; tom fill into 60 |
| 60–78 | Peak chorus in F#: the hook in octaves, tambourine, layered snare, glock sparkle, airy pads; fills into 68 and 76 |
| 78 | Unison F#maj9 hit, then a rising bubble wave |
| 80–88 | Outro: kalimba reprise of the hook over EP and pads |
| 89.0 | Button note |
| 89.88 onward | Digital silence |

Mix: plate, hall and ping-pong delay sends; kick-keyed sidechain on the pads, stabs, bass, keys and reverbs; mono below 120 Hz; a −1.5 dB dip at 2.6 kHz and a lighter 1.5–4 kHz bed so the sound effects sit on top; bus compression, soft saturation and a 4×-oversampled look-ahead limiter.

## Sound effects: `sfx/<name>.wav`

Eighteen one-shots: bloop, drip, splash, stretch, snap, pop, merge, whoosh, swoosh-up, click, type, ding, chime, tick, thud, glass, boing, shimmer. They are 48 kHz 24-bit, mono or stereo, and peak-normalized with a per-sound trim so equal cue gains sound roughly equally loud. The tonal ones (ding, chime, shimmer) use F#, G#, B and C#, which fit both keys.

## Rebuild

```sh
python tools/audio/build.py                      # music.wav and sfx/*.wav (deterministic, fixed seeds)
python tools/audio/mix.py                        # mix.wav from music + every scenes/*.cues.json
python tools/audio/mix.py path/to/cues --out x.wav
python tools/audio/analyze.py assets/audio/music.wav --png .work/audio/music.png
```

`mix.py` places each cue `{"t", "sfx", "gain"}` sample-accurately, with a small seeded pitch (±35 cents), gain (±1.2 dB) and pan (±0.35) variation per cue. It dips the music by up to 3 dB under loud effects, then masters `mix.wav` to −14 LUFS with true peak ≤ −1.0 dBTP, exactly 90.000 s. Missing or empty cue files are fine. Unknown sound names and malformed files produce a warning and are skipped.

Sources: `tools/audio/` holds `dsp.py` (filters, oscillators, reverb, dynamics, BS.1770 loudness, WAV I/O), `instruments.py` (voices), `score.py` (the arrangement and master), `sfx.py`, `build.py`, `mix.py` and `analyze.py`.
