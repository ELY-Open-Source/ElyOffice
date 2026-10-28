"""Places scenes' sound cues over the music; masters the film."""
import argparse
import json
import sys
import zlib
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))

from dsp import N, SR, butter, compress, db, limiter, lufs, read_wav, secs, true_peak, write_wav  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
TARGET_LUFS = -14.0
CEILING_DB = -1.3
MAX_TP_DB = -1.0
DUCK_FLOOR_DB = -26.0      # sfx level where the music starts to dip
DUCK_SLOPE = 0.35          # dB of dip per dB above the floor
DUCK_MAX_DB = 3.0


def warn(msg):
    print(f"warning: {msg}", file=sys.stderr)


def load_cues(cues_dir):
    cues = []
    if not cues_dir.is_dir():
        warn(f"no cue directory {cues_dir}; mixing music only")
        return cues
    for path in sorted(cues_dir.glob("*.cues.json")):
        text = path.read_text(encoding="utf-8").strip()
        if not text:
            continue
        try:
            items = json.loads(text)
        except json.JSONDecodeError as e:
            warn(f"{path.name}: not valid JSON ({e}); skipped")
            continue
        if not isinstance(items, list):
            warn(f"{path.name}: expected a JSON array; skipped")
            continue
        for i, c in enumerate(items):
            if not isinstance(c, dict) or "t" not in c or "sfx" not in c:
                warn(f"{path.name}[{i}]: needs \"t\" and \"sfx\"; ignored")
                continue
            try:
                t, gain = float(c["t"]), float(c.get("gain", 1.0))
            except (TypeError, ValueError):
                warn(f"{path.name}[{i}]: \"t\" and \"gain\" must be numbers; ignored")
                continue
            cues.append((path.name, i, t, str(c["sfx"]), min(max(gain, 0.0), 1.0)))
    return cues


def load_sfx(sfx_dir):
    lib = {}
    for path in sorted(sfx_dir.glob("*.wav")):
        x = read_wav(path)
        lib[path.stem] = x if x.shape[1] == 2 else np.repeat(x, 2, 1) / np.sqrt(2)
    return lib


def vary(x, file, index, t, name):
    """Tiny deterministic pitch, gain and placement changes per cue."""
    r = np.random.default_rng(zlib.crc32(f"{file}:{index}:{t:.4f}:{name}".encode()))
    ratio = 2 ** (r.uniform(-35, 35) / 1200)
    n = int(len(x) / ratio)
    pos = np.arange(n) * ratio
    y = np.stack([np.interp(pos, np.arange(len(x)), x[:, c]) for c in range(2)], 1)
    p = r.uniform(-0.35, 0.35)
    th = (p + 1) * np.pi / 4
    y *= np.array([np.cos(th), np.sin(th)]) * np.sqrt(2)
    return y * 10 ** (r.uniform(-1.2, 1.2) / 20)


def duck_gain(sfx_bus, block=48, attack=0.005, release=0.18):
    lvl = np.abs(sfx_bus).max(1)
    nb = -(-len(lvl) // block)
    peaks = np.pad(lvl, (0, nb * block - len(lvl))).reshape(nb, block).max(1)
    a, r = np.exp(-block / (attack * SR)), np.exp(-block / (release * SR))
    env, acc = np.empty(nb), 0.0
    for i, v in enumerate(peaks):
        acc = v + (acc - v) * (a if v > acc else r)
        env[i] = acc
    dip = np.clip((20 * np.log10(env + 1e-9) - DUCK_FLOOR_DB) * DUCK_SLOPE, 0, DUCK_MAX_DB)
    g = 10 ** (-np.interp(np.arange(len(lvl)), (np.arange(nb) + 0.5) * block, dip) / 20)
    return g[:, None]


def master(x):
    x = compress(butter(x, "high", 20), -12, 1.5, 0.015, 0.2, knee=8)
    g = TARGET_LUFS - lufs(x)
    for _ in range(6):
        y = limiter(np.tanh(x * 10 ** (g / 20) * 0.9) / 0.9, CEILING_DB)
        err = TARGET_LUFS - lufs(y)
        if abs(err) < 0.05:
            break
        g += err
    ceiling = CEILING_DB
    while db(true_peak(y)) > MAX_TP_DB:
        ceiling -= 0.2
        y = limiter(y, ceiling)
    return y


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("cues", nargs="?", default=str(ROOT / "scenes"),
                    help='folder of *.cues.json, each a JSON array of {"t", "sfx", "gain"}')
    ap.add_argument("--music", default=str(ROOT / "assets" / "audio" / "music.wav"))
    ap.add_argument("--sfx", default=str(ROOT / "assets" / "audio" / "sfx"))
    ap.add_argument("--out", default=str(ROOT / "assets" / "audio" / "mix.wav"))
    a = ap.parse_args()

    music = read_wav(a.music)
    if music.shape[1] == 1:
        music = np.repeat(music, 2, 1)
    music = np.pad(music, ((0, max(0, N - len(music))), (0, 0)))[:N]
    lib = load_sfx(Path(a.sfx))
    bus = np.zeros((N, 2))
    placed, unknown = 0, {}
    for file, index, t, name, gain in load_cues(Path(a.cues)):
        if name not in lib:
            unknown[name] = unknown.get(name, 0) + 1
            warn(f"{file}[{index}]: unknown sfx {name!r} at {t:.3f} s; ignored")
            continue
        i = int(round(t * SR))
        if not 0 <= i < N:
            warn(f"{file}[{index}]: {name} at {t:.3f} s is outside the film; ignored")
            continue
        y = vary(lib[name], file, index, t, name) * gain
        k = min(len(y), N - i)
        bus[i:i + k] += y[:k]
        placed += 1

    x = music * duck_gain(bus) + bus
    t = secs(N)
    x *= (np.cos(np.clip((t - 89.6) / 0.38, 0, 1) * np.pi / 2) ** 2)[:, None]
    x = master(x)
    x[t >= 89.98] = 0.0
    write_wav(a.out, x)
    print(f"placed {placed} cues ({sum(unknown.values())} unknown ignored); {a.out}: {len(x)} samples, "
          f"{lufs(x):.2f} LUFS, true peak {db(true_peak(x)):.2f} dBTP")


if __name__ == "__main__":
    main()
