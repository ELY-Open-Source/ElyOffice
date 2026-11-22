"""Lays pen and foley over the score; masters it."""
import argparse
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))

import cues as cue_file  # noqa: E402
import pen as pen_voice  # noqa: E402
from dsp import (balance, block_peaks, butter, compress, db, follow, lufs, N, per_sample, read_wav,  # noqa: E402
                 secs, SR, true_peak, write_wav)
from grid import HITS  # noqa: E402
from master import FADE, finish  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
TARGET_LUFS = -14.0
CEILING_DB = -1.9
MAX_TP_DB = -1.5      # AAC adds a few tenths; delivery is -1
PEN_LUFS = -31.0
DUCK_FLOOR_DB = -20.0
DUCK_SLOPE = 0.3
DUCK_MAX_DB = 3.0
HIT_HOLD = 0.35       # band hits keep their punch

# pen level ride in dB: featured alone and while writing
PEN_RIDE = ((0, 4.0), (7.5, 4.0), (8.5, 0.0), (77.5, 0.0), (78.5, 3.0), (87.5, 3.0), (88.5, 0.0), (111.5, 0.0),
            (112.0, -3.0), (113.5, -3.0), (114.0, 4.0), (120, 4.0))


def place(cues, index, sfx_dir):
    bus, cache = np.zeros((N, 2)), {}
    for _, _, t, name, var, gain, p in cues:
        key = f"{name}-{var}"
        if key not in cache:
            cache[key] = read_wav(sfx_dir / f"{key}.wav")
        x = balance(cache[key], p) * gain
        start = int(round((t - index[name]["anchors"][var - 1]) * SR))
        if start < 0:
            x, start = x[-start:], 0
        k = min(len(x), N - start)
        bus[start:start + k] += x[:k]
    return bus


def pen_bed(path):
    y = pen_voice.voice(pen_voice.load(path))
    loud = lufs(y)
    if not np.isfinite(loud):
        return y
    ts, vs = zip(*PEN_RIDE)
    ride = 10 ** (np.interp(secs(N), ts, vs) / 20)
    return y * 10 ** ((PEN_LUFS - loud) / 20) * ride[:, None]


def duck(music, foley):
    """Music ducks under loud foley, never on band hits."""
    lvl = per_sample(follow(block_peaks(foley, 48), 0.03, 0.2, 48), len(foley), 48)
    dip = np.clip((20 * np.log10(lvl + 1e-9) - DUCK_FLOOR_DB) * DUCK_SLOPE, 0, DUCK_MAX_DB)
    hold = np.zeros(len(foley))
    for t in HITS:
        hold[int((t - 0.02) * SR):int((t + HIT_HOLD) * SR)] = 1.0
    dip *= 1 - np.convolve(hold, np.ones(960) / 960, "same")
    return music * 10 ** (-dip / 20)[:, None]


def master(x):
    x = compress(butter(x, "high", 24), -16, 1.4, 0.015, 0.2, knee=8)
    return finish(x, TARGET_LUFS, CEILING_DB, MAX_TP_DB, drive=0.85)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--cues", default=str(ROOT / "scenes"), help="folder of <scene>.cues.json")
    ap.add_argument("--pen", default=str(ROOT / ".work" / "pen.json"))
    ap.add_argument("--music", default=str(ROOT / "assets" / "audio" / "music.wav"))
    ap.add_argument("--sfx", default=str(ROOT / "assets" / "audio" / "sfx"))
    ap.add_argument("--out", default=str(ROOT / "assets" / "audio" / "mix.wav"))
    a = ap.parse_args()
    sfx_dir = Path(a.sfx)
    index = json.loads((sfx_dir / "index.json").read_text())
    try:
        cues = cue_file.load(sorted(Path(a.cues).glob("*.cues.json")), index)
    except cue_file.CueError as e:
        sys.exit(f"cue errors, nothing written:\n{e}")
    try:
        bed = pen_bed(Path(a.pen))
    except (pen_voice.PenError, OSError, json.JSONDecodeError) as e:
        sys.exit(f"pen errors, nothing written:\n{e}")
    music = read_wav(a.music)
    if music.shape != (N, 2):
        sys.exit(f"{a.music}: expected {N} stereo samples, got {music.shape}")
    shots = place(cues, index, sfx_dir)
    x = duck(music, shots) + shots + bed
    t = secs(N)
    x *= (np.cos(np.clip((t - FADE[0]) / (FADE[1] - FADE[0]), 0, 1) * np.pi / 2) ** 2)[:, None]
    x = master(x)
    x[t >= FADE[1]] = 0.0
    write_wav(a.out, x)
    scenes = sorted({c[0] for c in cues})
    print(f"{len(cues)} cues from {', '.join(scenes) or 'no scenes'}; pen {a.pen}; {a.out}: "
          f"{lufs(x):.2f} LUFS, true peak {db(true_peak(x)):.2f} dBTP")


if __name__ == "__main__":
    main()
