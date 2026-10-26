"""Regenerate assets/audio/music.wav (cut from the 0.1.10 score) and assets/audio/sfx/*.wav.

Usage: python tools/audio/build.py   (needs numpy and scipy)
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import cut  # noqa: E402
from sfx import build_sfx  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]


def main():
    cut.main()
    names = build_sfx(ROOT / "assets" / "audio" / "sfx")
    print(f"sfx: {len(names)} sounds")


if __name__ == "__main__":
    main()
