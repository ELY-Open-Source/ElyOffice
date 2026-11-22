"""The pen on paper, driven frame by frame by .work/pen.json."""
import json

import numpy as np

from dsp import N, SR, balance, butter, convolve, eq, impulse, ns, secs, tv

KEYS = {"t", "x", "y", "speed", "pressure", "drawing", "scene"}
DX = 1 / 16          # texture cell, screen pixels
CELLS = 1 << 21
FULL = 1000.0        # px/s that reads as a full stroke


class PenError(Exception):
    pass


def load(path):
    """Per-frame arrays from pen.json; any malformed frame is an error."""
    doc = json.loads(path.read_text(encoding="utf-8"))
    fps, frames = doc.get("fps"), doc.get("frames")
    if not isinstance(fps, (int, float)) or fps <= 0 or not isinstance(frames, list) or not frames:
        raise PenError(f"{path}: expected {{\"fps\": number, \"frames\": [...]}}")
    errs = []
    for i, f in enumerate(frames):
        if not isinstance(f, dict) or set(f) != KEYS:
            errs.append(f"frame {i}: keys must be {sorted(KEYS)}")
        elif abs(f["t"] - i / fps) > 0.5 / fps:
            errs.append(f"frame {i}: t {f['t']} is not {i}/{fps}")
        elif f["drawing"] and f["x"] is None:
            errs.append(f"frame {i}: drawing with no position")
        if len(errs) > 20:
            break
    if len(frames) / fps < N / SR - 1 / fps:
        errs.append(f"{len(frames)} frames at {fps} fps end before {N / SR} s")
    if errs:
        raise PenError(f"{path}:\n" + "\n".join(errs))
    xs = [f["x"] for f in frames]
    known = [x for x in xs if x is not None]
    x = np.array([v if v is not None else (known[0] if known else 960.0) for v in xs], float)
    return {"fps": float(fps), "x": x, "speed": np.array([f["speed"] for f in frames], float),
            "pressure": np.array([f["pressure"] for f in frames], float),
            "drawing": np.array([bool(f["drawing"]) for f in frames], float),
            "y": np.array([f["y"] if f["y"] is not None else 540.0 for f in frames], float)}


def _per_sample(v, fps, smooth=0.0):
    t = np.arange(len(v)) / fps
    out = np.interp(secs(N), t, v)
    if smooth:
        k = max(1, ns(smooth))
        out = np.convolve(out, np.ones(k) / k, "same")
    return out


def _texture():
    """Paper tooth along the stroke: grain plus scattered fibres."""
    g = np.random.default_rng(1170)
    w = g.standard_normal(CELLS)
    tooth = butter(w, "low", SR * 0.18) + 0.3 * w
    fibres = g.standard_normal(CELLS) * (g.uniform(size=CELLS) < 0.02) * 4.0
    return tooth + fibres


def _corners(pen, fps):
    """Clicks where the stroke turns sharply at speed."""
    dx, dy = np.gradient(pen["x"]), np.gradient(pen["y"])
    ang = np.unwrap(np.arctan2(dy, dx))
    turn = np.abs(np.gradient(ang))
    hit = (turn > 0.9) & (pen["speed"] > 120) & (pen["drawing"] > 0)
    out = np.zeros(N)
    k = ns(0.004)
    shape = np.exp(-secs(k) / 0.0008)
    g = np.random.default_rng(1171)
    for i in np.nonzero(hit)[0]:
        s = ns(i / fps)
        if s + k < N:
            out[s:s + k] += g.standard_normal(k) * shape * min(1.0, turn[i] / 2.5) * 0.6
    return butter(out, "band", (1800, 7000))


def voice(pen):
    """Stereo scratch: level and brightness follow speed and pressure."""
    fps = pen["fps"]
    speed = _per_sample(pen["speed"], fps, 0.02)
    press = _per_sample(pen["pressure"], fps, 0.02)
    down = _per_sample(pen["drawing"], fps, 0.012)
    pan = np.clip((_per_sample(pen["x"], fps, 0.03) - 960) / 960, -1, 1) * 0.7
    pos = np.cumsum(speed * down) / SR / DX
    tex = _texture()
    i0 = np.floor(pos).astype(np.int64)
    fr = pos - i0
    raw = tex[i0 % CELLS] * (1 - fr) + tex[(i0 + 1) % CELLS] * fr
    raw = butter(raw, "high", 180)
    bright = 1800 + 6500 * np.clip(speed / 2500, 0, 1) * (1.25 - 0.35 * press / 1.35)
    bright = np.clip(np.minimum(bright, 0.45 * speed / DX), 300, None)
    y = tv(tv(raw, "low", bright, 0.7, block=96), "low", bright, 0.7, block=96)
    y = eq(eq(y, "peak", 3300, 1.4, 4.0), "peak", 420, 0.9, 2.0)
    body = butter(raw, "band", (260, 700)) * (press / 1.35) * 0.4
    level = np.sqrt(np.clip(speed / FULL, 0, 2.6)) * (0.55 + 0.45 * press / 1.35) * down
    y = butter((y + body) * level + _corners(pen, fps) * down, "high", 240, 4)
    dry = balance(np.stack([y, y], 1), pan)
    room = convolve(dry * 0.25, impulse(0.35, (0.25, 0.2, 0.12), 1172, predelay=0.003, damp=7000, build=0.002))
    return dry + butter(room, "high", 200)
