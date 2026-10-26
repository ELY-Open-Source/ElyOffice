"""Regenerate assets/audio/music.wav and assets/audio/sfx/*.wav deterministically.

Usage: python tools/audio/build.py   (needs numpy and scipy)
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))

from dsp import N, SR, db, lufs, true_peak, write_wav  # noqa: E402
from score import build_music  # noqa: E402
from sfx import build_sfx  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "audio"


def main():
    x, stems = build_music()
    if len(x) != N:
        raise SystemExit(f"music is {len(x)} samples, expected {N}")
    write_wav(OUT / "music.wav", x)
    print(f"music: {len(x) / SR:.3f} s, {lufs(x):.2f} LUFS, true peak {db(true_peak(x)):.2f} dBTP")
    print("stem RMS, peak 12-26 s (dBFS, pre-master):", {k: v for k, v in stems.items() if v[0] > -200})
    names = build_sfx(OUT / "sfx")
    print(f"sfx: {len(names)} sounds in {OUT / 'sfx'}")


if __name__ == "__main__":
    main()
