"""Syncs takes to their logs: sync.py <takes dir> <name> ..."""
import json
import subprocess
import sys
from pathlib import Path

import av
import numpy as np


def first_change(video: Path, x: int, y: int, after: float, until: float):
    """First frame time when the patch at (x, y) changes."""
    with av.open(str(video)) as box:
        stream = box.streams.video[0]
        start = float(stream.start_time * stream.time_base) if stream.start_time else 0.0
        base = None
        for frame in box.decode(stream):
            t = float(frame.pts * stream.time_base) - start
            if t < after:
                continue
            if t > until:
                break
            patch = frame.to_ndarray(format="gray")[y - 24:y + 24, x - 24:x + 24].astype(np.int16)
            if base is None:
                base = patch
                continue
            if np.abs(patch - base).mean() > 6:
                return t, start
    raise RuntimeError(f"{video.name}: no sync jump between {after} and {until} s")


root = Path(sys.argv[1])
for name in sys.argv[2:]:
    events = json.loads((root / f"{name}.json").read_text(encoding="utf-8"))
    sync = next(e for e in events if e["kind"] == "sync")
    seen, start = first_change(root / f"{name}.mp4", sync["x"], sync["y"], sync["t"] - 0.5, sync["t"] + 1.5)
    offset = round(seen - sync["t"], 4)
    (root / f"{name}.sync.json").write_text(json.dumps({"offset": offset, "start": start}), encoding="utf-8")
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(root / f"{name}.mp4"),
                    "-vf", "fps=60", "-c:v", "h264_nvenc", "-preset", "p7", "-rc", "vbr", "-cq", "14",
                    "-g", "30", "-pix_fmt", "yuv420p", str(root / f"{name}.60.mp4")], check=True)
    print(f"{name}: jump seen at {seen:.3f} s, logged at {sync['t']:.3f} s, offset {offset:+.3f} s")
