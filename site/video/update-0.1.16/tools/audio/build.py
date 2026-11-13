"""Regenerates assets/audio: music.wav and sfx/; needs numpy and scipy."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from dsp import db, lufs, N, SR, true_peak, write_wav  # noqa: E402
from score import build_music  # noqa: E402
from sfx import build_sfx  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "audio"


def main():
    what = set(sys.argv[1:]) or {"music", "sfx"}
    unknown = what - {"music", "sfx"}
    if unknown:
        raise SystemExit(f"unknown target {sorted(unknown)}; use music, sfx or nothing for both")
    if "music" in what:
        x, stems = build_music()
        if len(x) != N:
            raise SystemExit(f"music is {len(x)} samples, expected {N}")
        write_wav(OUT / "music.wav", x)
        print(f"music: {len(x) / SR:.3f} s, {lufs(x):.2f} LUFS, true peak {db(true_peak(x)):.2f} dBTP")
        print("stem RMS 8-20 s, dBFS pre-master:", stems)
    if "sfx" in what:
        index = build_sfx(OUT / "sfx")
        shots = sum(v.get("variants", 0) for v in index.values())
        print(f"sfx: {shots} one-shots in {len(index)} sounds, {OUT / 'sfx'}")


if __name__ == "__main__":
    main()
