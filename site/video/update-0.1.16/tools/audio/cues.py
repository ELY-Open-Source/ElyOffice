"""Reads and checks scenes/*.cues.json; any problem is an error."""
import json
import zlib
from pathlib import Path

WINDOWS = {"s1": (0, 8), "s2": (8, 20), "s3": (20, 32), "s4": (32, 42), "s5": (42, 54), "s6": (54, 64),
           "s7": (64, 74), "s8": (74, 82), "s9": (82, 90)}
KEYS = {"t", "sfx", "gain", "pan", "var"}


class CueError(Exception):
    pass


def _num(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def _check_cue(c, where, lo, hi, index, errs):
    if not isinstance(c, dict):
        errs.append(f"{where}: a cue is an object")
        return None
    extra = set(c) - KEYS
    if extra:
        errs.append(f"{where}: unknown keys {sorted(extra)}")
    t, name = c.get("t"), c.get("sfx")
    gain, p, var = c.get("gain", 1.0), c.get("pan", 0.0), c.get("var")
    if not _num(t) or not lo <= t <= hi:
        errs.append(f"{where}: \"t\" must be film seconds within {lo}-{hi}, got {t!r}")
    if not isinstance(name, str) or name not in index or name == "crackle":
        errs.append(f"{where}: unknown sfx {name!r}")
        return None
    if not _num(gain) or not 0 < gain <= 2:
        errs.append(f"{where}: \"gain\" must be in (0, 2], got {gain!r}")
    if not _num(p) or not -1 <= p <= 1:
        errs.append(f"{where}: \"pan\" must be in [-1, 1], got {p!r}")
    count = index[name]["variants"]
    if var is not None and (not isinstance(var, int) or isinstance(var, bool) or not 1 <= var <= count):
        errs.append(f"{where}: \"var\" for {name} must be 1-{count}, got {var!r}")
    return t, name, gain, p, var


def _check_flame(points, where, lo, hi, errs):
    out, last = [], None
    if not isinstance(points, list):
        errs.append(f"{where}: \"flame\" is a list of [t, energy] or [t, energy, pan]")
        return out
    for i, pt in enumerate(points):
        w = f"{where}.flame[{i}]"
        if not isinstance(pt, list) or len(pt) not in (2, 3) or not all(_num(v) for v in pt):
            errs.append(f"{w}: expected [t, energy] or [t, energy, pan], got {pt!r}")
            continue
        t, e, p = pt[0], pt[1], pt[2] if len(pt) == 3 else 0.0
        if not lo <= t <= hi:
            errs.append(f"{w}: t {t} is outside {lo}-{hi}")
        if not 0 <= e <= 1 or not -1 <= p <= 1:
            errs.append(f"{w}: energy must be 0-1 and pan -1..1")
        if last is not None and t < last:
            errs.append(f"{w}: t {t} goes back in time (previous {last})")
        last = t
        out.append((t, e, p))
    return out


def pick_variant(scene, i, name, count, previous):
    """Deterministic choice; never the same variant twice running."""
    k = zlib.crc32(f"{scene}:{i}:{name}".encode()) % count + 1
    if count > 1 and k == previous:
        k = k % count + 1
    return k


def load(paths, index):
    """Returns (cues, flame points); raises CueError listing every problem."""
    errs, cues, flame = [], [], []
    for path in paths:
        scene = path.name.removesuffix(".cues.json")
        if scene not in WINDOWS:
            errs.append(f"{path.name}: scene must be one of {', '.join(WINDOWS)}")
            continue
        lo, hi = WINDOWS[scene]
        try:
            doc = json.loads(path.read_text(encoding="utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as e:
            errs.append(f"{path.name}: not UTF-8 JSON ({e})")
            continue
        if not isinstance(doc, dict) or set(doc) - {"cues", "flame"} or not isinstance(doc.get("cues", []), list):
            errs.append(f"{path.name}: expected {{\"cues\": [...], \"flame\": [...]}}")
            continue
        last = {}
        for i, c in enumerate(doc.get("cues", [])):
            got = _check_cue(c, f"{path.name}.cues[{i}]", lo, hi, index, errs)
            if got is None:
                continue
            t, name, gain, p, var = got
            if var is None:
                var = pick_variant(scene, i, name, index[name]["variants"], last.get(name))
            last[name] = var
            cues.append((scene, i, t, name, var, gain, p))
        flame += _check_flame(doc.get("flame", []), path.name, lo, hi, errs)
    if errs:
        raise CueError("\n".join(errs))
    return cues, sorted(flame, key=lambda f: f[0])


def main():
    """Checks scenes/*.cues.json against the built library."""
    root = Path(__file__).resolve().parents[2]
    index = json.loads((root / "assets" / "audio" / "sfx" / "index.json").read_text())
    try:
        cues, flame = load(sorted((root / "scenes").glob("*.cues.json")), index)
    except CueError as e:
        raise SystemExit(f"cue errors:\n{e}")
    off = [c for c in cues if c[5] >= 0.8 and abs(c[2] * 8 - round(c[2] * 8)) > 0.2]
    print(f"{len(cues)} cues, {len(flame)} flame points: valid")
    for scene, i, t, name, _, gain, _ in off:
        print(f"off the eighth-beat grid: {scene}.cues[{i}] {name} x{gain} at {t}")


if __name__ == "__main__":
    main()
