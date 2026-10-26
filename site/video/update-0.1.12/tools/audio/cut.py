"""The 0.1.12 music: the 0.1.10 score, 0-28 s then 58-90 s, joined on a bar line.

Usage: python tools/audio/cut.py [SOURCE]   SOURCE defaults to ../update-0.1.10/assets/audio/music.wav
A bar of section B, then the tom fill into the F# chorus, which lands at 30 s.
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))

from dsp import N, SR, db, lufs, read_wav, true_peak, write_wav  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT.parent / "update-0.1.10" / "assets" / "audio" / "music.wav"
JOIN, RESUME = 28.0, 58.0
FADE = 0.02


def main():
    src = Path(sys.argv[1]) if len(sys.argv) > 1 else SOURCE
    x = read_wav(src)
    if len(x) != int(90 * SR):
        raise SystemExit(f"{src} is {len(x) / SR:.3f} s, not the 90 s score; run update-0.1.10/tools/audio/build.py")
    j, r, f = int(JOIN * SR), int(RESUME * SR), int(FADE * SR)
    head, tail = x[: j + f // 2].copy(), x[r - f // 2 :].copy()
    w = np.sin(np.linspace(0, np.pi / 2, f))[:, None]
    head[-f:] *= np.cos(np.linspace(0, np.pi / 2, f))[:, None]
    tail[:f] *= w
    out = np.concatenate([head[:-f], head[-f:] + tail[:f], tail[f:]])
    if len(out) != N:
        raise SystemExit(f"the cut is {len(out)} samples, expected {N}")
    write_wav(ROOT / "assets" / "audio" / "music.wav", out)
    print(f"music: {len(out) / SR:.3f} s, {lufs(out):.2f} LUFS, true peak {db(true_peak(out)):.2f} dBTP")


if __name__ == "__main__":
    main()
