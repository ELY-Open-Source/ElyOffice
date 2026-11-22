"""Reads and checks every scene's cue file."""
import json
import subprocess
import sys
import zlib
from pathlib import Path

KEYS = {"t", "sfx", "gain", "pan", "var"}
GRID = 16            # eighth of a beat: 0.0625 s
ROOT = Path(__file__).resolve().parents[2]


class CueError(Exception):
    pass


LOADER = """
const out = {};
for (const id of process.argv.slice(1)) {
  try {
    const s = (await import(`./scenes/${id}.js`)).default;
    out[id] = s.id === id ? [s.start, s.end] : `exports id ${s.id}`;
  } catch (e) {
    out[id] = String(e.message ?? e).split('\\n')[0];
  }
}
process.stdout.write(JSON.stringify(out));
"""


def windows(ids, root=ROOT):
    """{id: (start, end) or error text}, importing only these scenes."""
    run = subprocess.run(["node", "--input-type=module", "-e", LOADER, *ids], cwd=root, capture_output=True,
                         text=True)
    if run.returncode:
        raise CueError(f"node could not read scene windows:\n{run.stderr.strip()}")
    return {k: tuple(v) if isinstance(v, list) else v for k, v in json.loads(run.stdout).items()}


def _num(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def _check(c, where, lo, hi, index, errs):
    if not isinstance(c, dict):
        errs.append(f"{where}: a cue is an object, got {c!r}")
        return None
    bad = len(errs)
    extra = set(c) - KEYS
    if extra:
        errs.append(f"{where}: unknown keys {sorted(extra)}")
    t, name = c.get("t"), c.get("sfx")
    gain, p, var = c.get("gain", 1.0), c.get("pan", 0.0), c.get("var")
    if not _num(t):
        errs.append(f"{where}: \"t\" must be film seconds, got {t!r}")
    else:
        if not lo <= t < hi:
            errs.append(f"{where}: t {t} is outside the scene, {lo}-{hi}")
        if abs(t * GRID - round(t * GRID)) > 1e-6:
            errs.append(f"{where}: t {t} is off the eighth-beat grid (multiples of {1 / GRID} s)")
    if not isinstance(name, str) or name not in index:
        errs.append(f"{where}: unknown sfx {name!r}; known: {', '.join(sorted(index))}")
        return None
    if not _num(gain) or not 0 < gain <= 2:
        errs.append(f"{where}: \"gain\" must be in (0, 2], got {gain!r}")
    if not _num(p) or not -1 <= p <= 1:
        errs.append(f"{where}: \"pan\" must be in [-1, 1], got {p!r}")
    count = index[name]["variants"]
    if var is not None and (not isinstance(var, int) or isinstance(var, bool) or not 1 <= var <= count):
        errs.append(f"{where}: \"var\" for {name} must be 1-{count}, got {var!r}")
    return None if len(errs) > bad else (t, name, gain, p, var)


def pick(scene, i, name, count, previous):
    """Fixed choice per cue; never the same variant twice running."""
    k = zlib.crc32(f"{scene}:{i}:{name}".encode()) % count + 1
    if count > 1 and k == previous:
        k = k % count + 1
    return k


def load(paths, index):
    """Cue tuples; CueError lists every problem."""
    errs, cues = [], []
    ids = [p.name.removesuffix(".cues.json") for p in paths]
    spans = windows([i for i in ids if (ROOT / "scenes" / f"{i}.js").exists()]) if ids else {}
    for path, scene in zip(paths, ids):
        if scene not in spans:
            errs.append(f"{path.name}: no scenes/{scene}.js")
            continue
        if isinstance(spans[scene], str):
            errs.append(f"{path.name}: scenes/{scene}.js does not load: {spans[scene]}")
            continue
        lo, hi = spans[scene]
        try:
            doc = json.loads(path.read_text(encoding="utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as e:
            errs.append(f"{path.name}: not UTF-8 JSON ({e})")
            continue
        if not isinstance(doc, dict) or set(doc) != {"cues"} or not isinstance(doc["cues"], list):
            errs.append(f"{path.name}: expected {{\"cues\": [...]}}")
            continue
        last = {}
        for i, c in enumerate(doc["cues"]):
            got = _check(c, f"{path.name} cue {i}", lo, hi, index, errs)
            if got is None:
                continue
            t, name, gain, p, var = got
            if var is None:
                var = pick(scene, i, name, index[name]["variants"], last.get(name))
            last[name] = var
            cues.append((scene, i, t, name, var, gain, p))
    if errs:
        raise CueError("\n".join(errs))
    return sorted(cues, key=lambda c: c[2])


def main():
    """Checks every cue file against the built library."""
    index = json.loads((ROOT / "assets" / "audio" / "sfx" / "index.json").read_text())
    try:
        cues = load(sorted((ROOT / "scenes").glob("*.cues.json")), index)
    except CueError as e:
        sys.exit(f"cue errors:\n{e}")
    print(f"{len(cues)} cues valid")


if __name__ == "__main__":
    main()
