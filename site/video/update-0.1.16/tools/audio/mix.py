"""Lays foley and flame over the music; masters the film."""
import argparse
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))

import cues as cue_file  # noqa: E402
from dsp import (balance, block_peaks, butter, compress, db, follow, lufs, N, per_sample, read_wav, secs,  # noqa: E402
                 SR, true_peak, write_wav)
from band import HITS  # noqa: E402
from master import finish  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
TARGET_LUFS = -14.0
CEILING_DB = -2.3
MAX_TP_DB = -2.0       # AAC encoding overshoots about 1 dB
FADE = (89.3, 89.96)      # picture black at 89.3
DUCK_FLOOR_DB = -18.0
DUCK_SLOPE = 0.3
DUCK_MAX_DB = 3.0
HIT_HOLD = 0.35       # a band hit is not ducked this long
SLOT_DB = 4.5         # music dip under a roaring flame


def place_cues(cues, index, sfx_dir):
    bus, cache = np.zeros((N, 2)), {}
    for scene, i, t, name, var, gain, p in cues:
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


def flame_bed(points, sfx_dir):
    """Crackle beds crossfaded by energy, panned with the flame."""
    if not points:
        return np.zeros((N, 2)), np.zeros(N)
    t = secs(N)
    ts, es, ps = (np.array(v, float) for v in zip(*points))
    inside = (t >= ts[0]) & (t <= ts[-1])
    box = np.ones(480) / 480
    e = np.convolve(np.where(inside, np.interp(t, ts, es), 0.0), box, "same")
    p = np.convolve(np.interp(t, ts, ps), box, "same")
    soft, mid, roar = (np.resize(read_wav(sfx_dir / f"crackle-{k}.wav"), (N, 2)) for k in ("soft", "mid", "roar"))
    a = np.clip((e - 0.25) / 0.35, 0, 1)
    b = np.clip((e - 0.6) / 0.4, 0, 1)
    w_soft, w_mid, w_roar = np.cos(a * np.pi / 2), np.sin(a * np.pi / 2) * np.cos(b * np.pi / 2), np.sin(b * np.pi / 2)
    bed = soft * w_soft[:, None] + mid * w_mid[:, None] + roar * w_roar[:, None]
    return balance(bed, p) * np.sqrt(e)[:, None], e


def duck(music, sfx, energy):
    """Music yields to foley and leaves the flame its band."""
    lvl = per_sample(follow(block_peaks(sfx, 48), 0.03, 0.2, 48), len(sfx), 48)
    dip = np.clip((20 * np.log10(lvl + 1e-9) - DUCK_FLOOR_DB) * DUCK_SLOPE, 0, DUCK_MAX_DB)
    hold = np.zeros(len(sfx))
    for t, _ in HITS:
        hold[int((t - 0.02) * SR):int((t + HIT_HOLD) * SR)] = 1.0
    dip *= 1 - np.convolve(hold, np.ones(960) / 960, "same")
    music = music * 10 ** (-dip / 20)[:, None]
    band = butter(music, "band", (1800, 5500))
    depth = 1 - 10 ** (-SLOT_DB * energy / 20)
    return music - band * depth[:, None]


def master(x):
    x = compress(butter(x, "high", 22), -14, 1.4, 0.015, 0.2, knee=8)
    return finish(x, TARGET_LUFS, CEILING_DB, MAX_TP_DB, drive=0.85)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--cues", default=str(ROOT / "scenes"), help="folder of <scene>.cues.json")
    ap.add_argument("--music", default=str(ROOT / "assets" / "audio" / "music.wav"))
    ap.add_argument("--sfx", default=str(ROOT / "assets" / "audio" / "sfx"))
    ap.add_argument("--out", default=str(ROOT / "assets" / "audio" / "mix.wav"))
    a = ap.parse_args()
    sfx_dir = Path(a.sfx)
    index = json.loads((sfx_dir / "index.json").read_text())
    paths = sorted(Path(a.cues).glob("*.cues.json"))
    try:
        cues, flame = cue_file.load(paths, index)
    except cue_file.CueError as e:
        raise SystemExit(f"cue errors:\n{e}")

    music = read_wav(a.music)
    if music.shape != (N, 2):
        raise SystemExit(f"{a.music}: expected {N} stereo samples, got {music.shape}")
    shots = place_cues(cues, index, sfx_dir)
    bed, energy = flame_bed(flame, sfx_dir)
    x = duck(music, shots + bed * 0.5, energy) + shots + bed
    t = secs(N)
    x *= (np.cos(np.clip((t - FADE[0]) / (FADE[1] - FADE[0]), 0, 1) * np.pi / 2) ** 2)[:, None]
    x = master(x)
    x[t >= FADE[1]] = 0.0
    write_wav(a.out, x)
    scenes = sorted({c[0] for c in cues})
    print(f"{len(cues)} cues from {', '.join(scenes) or 'no scenes'}, {len(flame)} flame points; {a.out}: "
          f"{lufs(x):.2f} LUFS, true peak {db(true_peak(x)):.2f} dBTP")


if __name__ == "__main__":
    main()
