#!/usr/bin/env python3
"""Composites the launch film from scenes, recordings, layers and captions."""
import json
import math
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE.parent / "out"
SCENES = OUT / "scenes"
REC = OUT / "rec"
EXTRA = OUT / "extra"
WORK = OUT / "edit"
FILM = OUT / "ElyOffice-launch.mp4"
FPS = 30
BEAT = 60 / 112
TOTAL = 2700
RGB_IN = "scale=in_color_matrix=bt709:in_range=tv,format=rgb24"
# Recording is shrunk into a padded frame for clean edges.
PAD = 4


def run(args):
    print("+", " ".join(str(a) for a in args[:6]), "...", flush=True)
    subprocess.run(["ffmpeg", "-hide_banner", "-v", "error", "-y", *map(str, args)], check=True)


def lerp(points, x):
    """Piecewise linear through points, extended past both ends."""
    i = 0
    while i < len(points) - 2 and x > points[i + 1][0]:
        i += 1
    (x0, y0), (x1, y1) = points[i], points[i + 1]
    return y0 + (x - x0) * (y1 - y0) / (x1 - x0)


def piecewise(points, var):
    """An ffmpeg expression for lerp over var."""
    if len(points) == 1:
        return f"{points[0][1]:.4f}"
    expr = None
    for i in range(len(points) - 2, -1, -1):
        (x0, y0), (x1, y1) = points[i], points[i + 1]
        seg = f"({y0:.4f}+({var}-{x0:.4f})*{(y1 - y0) / (x1 - x0):.6f})"
        expr = seg if expr is None else f"if(lt({var},{x1:.4f}),{seg},{expr})"
    return expr


def knots(values):
    """Breakpoints where a per-frame series stops being linear."""
    keep = [(0, values[0])]
    for k in range(1, len(values) - 1):
        if abs(values[k - 1] - 2 * values[k] + values[k + 1]) > 1e-3:
            keep.append((k, values[k]))
    if len(values) > 1:
        keep.append((len(values) - 1, values[-1]))
    return keep


def frames_expr(values):
    """Per-frame value; perspective counts input frames from one."""
    return "+".join(f"eq(in,{k + 1})*{v:.3f}" for k, v in enumerate(values))


def solve(a, b):
    """Gaussian elimination for a small dense system."""
    n = len(b)
    m = [row[:] + [b[i]] for i, row in enumerate(a)]
    for c in range(n):
        p = max(range(c, n), key=lambda r: abs(m[r][c]))
        m[c], m[p] = m[p], m[c]
        for r in range(n):
            if r != c:
                f = m[r][c] / m[c][c]
                m[r] = [x - f * y for x, y in zip(m[r], m[c])]
    return [m[i][n] / m[i][i] for i in range(n)]


def homography(src, dst):
    """3x3 map taking four src points to four dst points."""
    a, b = [], []
    for (x, y), (u, v) in zip(src, dst):
        a.append([x, y, 1, 0, 0, 0, -u * x, -u * y]); b.append(u)
        a.append([0, 0, 0, x, y, 1, -v * x, -v * y]); b.append(v)
    h = solve(a, b) + [1]
    return lambda x, y: ((h[0] * x + h[1] * y + h[2]) / (h[6] * x + h[7] * y + 1),
                         (h[3] * x + h[4] * y + h[5]) / (h[6] * x + h[7] * y + 1))


def padded(corners):
    """Pins for the padded frame so the content meets `corners`."""
    content = [(PAD, PAD), (1920 - PAD, PAD), (PAD, 1080 - PAD), (1920 - PAD, 1080 - PAD)]
    h = homography(content, corners)
    return [h(0, 0), h(1920, 0), h(0, 1080), h(1920, 1080)]


def piece_chain(label, piece, start, f0, f1, out):
    """Retimes one recording piece into frames f0..f1 of a section."""
    anchors = [((start + b) * BEAT, s) for b, s in piece["anchors"]]
    inverse = [(s, t) for t, s in anchors]
    u0 = max(lerp(anchors, f0 / FPS) - 0.1, 0)
    u1 = lerp(anchors, f1 / FPS) + 0.1
    pts = piecewise(inverse, f"({u0:.6f}+T)")
    n = f1 - f0
    return (
        f"[{label}:v]trim=start={u0:.4f}:end={u1:.4f},setpts=PTS-STARTPTS,"
        f"setpts='({pts}-{f0 / FPS:.6f})/TB',fps={FPS}:start_time=0,"
        f"trim=end_frame={n},tpad=stop_mode=clone:stop={n},trim=end_frame={n},setpts=N/{FPS}/TB,"
        f"{RGB_IN},setsar=1[{out}]"
    )


def section(s, scene, end, plan):
    """Renders one app section to a lossless file."""
    app, first, n_plate = s["app"], scene["first"], scene["frames"]
    length = end - first
    start = scene["bars"][0] * 4 - first / FPS / BEAT
    inputs, chains, parts = [], [], []
    bounds = []
    for i, p in enumerate(s["pieces"]):
        a = p["anchors"]
        f0 = 0 if i == 0 else math.ceil((start + a[0][0]) * BEAT * FPS - 1e-6)
        bounds.append(f0)
    bounds.append(length)
    for i, p in enumerate(s["pieces"]):
        inputs += ["-i", REC / f"{p['src']}.mp4"]
        chains.append(piece_chain(i, p, start, bounds[i], bounds[i + 1], f"p{i}"))
        parts.append(f"[p{i}]")
    k = len(inputs) // 2
    inputs += ["-framerate", FPS, "-i", SCENES / scene["name"] / "%05d.png"]
    inputs += ["-loop", 1, "-framerate", FPS, "-i", WORK / "cards" / "frame.png"]
    inputs += ["-loop", 1, "-framerate", FPS, "-i", WORK / "cards" / f"{app}.png"]
    inputs += ["-loop", 1, "-framerate", FPS, "-i", WORK / "cards" / "frame.png"]
    chains.append(f"{''.join(parts)}concat=n={len(parts)}:v=1:a=0[rec]")
    chains.append("[rec]split[ra][rb]")
    chains.append(plate_chain(scene, "[ra]trim=end_frame=%d,setpts=PTS-STARTPTS" % n_plate, k, f"[{k + 3}:v]format=rgba", "plate"))
    chains.append(full_chain(app, plan, "[rb]trim=start_frame=%d,setpts=PTS-STARTPTS" % n_plate,
                             first + n_plate, length - n_plate, k, f"[{k + 1}:v]", "full"))
    chains.append("[plate][full]concat=n=2:v=1:a=0,format=rgb24[v]")
    target = WORK / f"{app}.mkv"
    run([*inputs, "-filter_complex", ";".join(chains), "-map", "[v]", "-frames:v", length,
         "-c:v", "libx264rgb", "-qp", 0, "-preset", "ultrafast", target])
    return target


def plate_chain(scene, rec, k, card, out):
    """Pins the recording to the screen, flattened into the window."""
    data = json.loads((SCENES / f"{scene['name']}.json").read_text())["frames"]
    landed = next(f["corners"] for f in data if f["corners"])
    corners = [padded(f["corners"] or landed) for f in data]
    pins = ":".join(
        f"{axis}{c}='{frames_expr([fr[c][j] for fr in corners])}'"
        for c in range(4) for j, axis in enumerate("xy")
    )
    fades = "".join(
        f",colorchannelmixer=aa={f['opacity']:.3f}:enable='eq(n,{f['local']})'"
        for f in data if f["opacity"] < 0.999
    )
    flats = "".join(
        f",colorchannelmixer=aa={f['flat']:.4f}:enable='eq(n,{f['local']})'"
        for f in data if f["flat"] < 0.9999
    )
    return (
        f"{rec},scale=1920-{2 * PAD}:1080-{2 * PAD}:flags=area,format=rgba,"
        f"pad=1920:1080:{PAD}:{PAD}:color=black@0,setsar=1,"
        f"perspective={pins}:sense=destination:eval=frame:interpolation=cubic{fades}[pin];"
        f"{card}{flats}[mask];"
        f"[{k}:v]format=rgba[bg];[bg][mask]overlay=format=rgb:shortest=1[bm];"
        f"[bm][pin]overlay=format=rgb:eof_action=repeat,format=rgb24,setsar=1[{out}]"
    )


def view_expr(app, first, n):
    """Per-frame crop from the fx layer, or the whole recording."""
    path = EXTRA / f"{app}-fx" / "view.json"
    if not path.exists():
        return [[0, 0, 1920, 1080]] * n
    view = json.loads(path.read_text())
    rects = view["rects"]
    if view["first"] != first or len(rects) < n:
        raise SystemExit(f"{path}: frames {view['first']}+{len(rects)} do not cover {first}+{n}")
    return rects[:n]


def full_chain(app, plan, rec, first, n, k, card, out):
    """Letterboxes the recording with the fx punch-ins and the caption."""
    w = plan["window"]
    rects = view_expr(app, first, n)
    series = {
        "left": [r[0] for r in rects], "top": [r[1] for r in rects],
        "right": [r[0] + r[2] for r in rects], "bottom": [r[1] + r[3] for r in rects],
    }
    e = {name: piecewise(knots(v), "(in-1)") for name, v in series.items()}
    quad = (f"x0='{e['left']}':y0='{e['top']}':x1='{e['right']}':y1='{e['top']}':"
            f"x2='{e['left']}':y2='{e['bottom']}':x3='{e['right']}':y3='{e['bottom']}'")
    rise = "round(12*pow(max(0,1-t/0.2),2))"
    return (
        f"{rec},perspective={quad}:sense=source:eval=frame:interpolation=cubic,"
        f"scale={w['w']}:{w['h']}:flags=lanczos,pad=1920:1080:{w['x']}:{w['y']}:color=0xfbfbfa,setsar=1,format=rgba[lb];"
        f"[lb]{card}overlay=format=rgb:shortest=1[lf];"
        f"[{k + 2}:v]format=rgba,fade=in:st=0:d=0.2:alpha=1[cap];"
        f"[lf][cap]overlay=x=0:y='{rise}':format=rgb:eval=frame,trim=end_frame={n},format=rgb24,setsar=1[{out}]"
    )


def scene_input(name):
    return ["-framerate", FPS, "-i", SCENES / name / "%05d.png"]


def layers():
    """Delivered layers, later starts on top; grid under its fx."""
    found = []
    for path in sorted(EXTRA.glob("*/manifest.json")):
        m = json.loads(path.read_text())
        if m["fps"] != FPS or m["kind"] not in ("base", "over"):
            raise SystemExit(f"{path}: unexpected manifest")
        have = len(list(path.parent.glob("[0-9]*.png")))
        if have != m["count"]:
            raise SystemExit(f"{path}: {have} frames, manifest says {m['count']}")
        found.append(m | {"dir": path.parent})
    rank = lambda m: (m["first"], m["kind"] != "base", m["name"] != "fx-grid")
    return sorted(found, key=rank)


def starts():
    """Each app name layer starts no earlier than its plate."""
    scenes = json.loads((SCENES / "scenes.json").read_text())["scenes"]
    return {f"type-{s['name'][6:]}": s["first"] for s in scenes if s["name"].startswith("plate-")}


def film(made):
    """Joins the parts, lays delivered layers on top, adds music."""
    order = ["opening", "rise", *made, "files", "props", "end"]
    inputs = []
    for part in order:
        inputs += ["-i", part] if isinstance(part, Path) else scene_input(part)
    graph = ["".join(f"[{i}:v]" for i in range(len(order)))
             + f"concat=n={len(order)}:v=1:a=0,format=rgb24,settb=1/{FPS},setpts=N[base0]"]
    last = "base0"
    floor = starts()
    for j, m in enumerate(layers()):
        if m["first"] < floor.get(m["name"], 0):
            m = m | {"first": floor[m["name"]], "count": m["count"] - (floor[m["name"]] - m["first"])}
        index = sum(1 for a in inputs if a == "-i")
        inputs += ["-framerate", FPS, "-start_number", m["first"], "-i", m["dir"] / "%05d.png"]
        hold = ",tpad=stop_mode=clone:stop=1" if m["name"] == "opening" else ""
        graph.append(f"[{index}:v]format=rgba{hold},settb=1/{FPS},setpts=N+{m['first']}[l{j}]")
        graph.append(f"[{last}][l{j}]overlay=format=rgb:eof_action=pass:repeatlast=0[base{j + 1}]")
        print(f"layer {m['name']}: {m['kind']} {m['first']}..{m['first'] + m['count'] - 1}")
        last = f"base{j + 1}"
    music = sum(1 for a in inputs if a == "-i")
    inputs += ["-i", OUT / "music.wav"]
    graph.append(f"[{last}]setpts=N/{FPS}/TB,scale=out_color_matrix=bt709:out_range=tv,format=yuv420p[v]")
    run([*inputs, "-filter_complex", ";".join(graph), "-map", "[v]", "-map", f"{music}:a",
         "-frames:v", TOTAL, "-c:v", "libx264", "-profile:v", "high", "-preset", "slow", "-crf", 18,
         "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-color_range", "tv",
         "-r", FPS, "-c:a", "aac", "-b:a", "192k", "-t", "90", "-movflags", "+faststart", FILM])


def main():
    plan = json.loads((HERE / "plan.json").read_text())
    scenes = json.loads((SCENES / "scenes.json").read_text())["scenes"]
    WORK.mkdir(parents=True, exist_ok=True)
    subprocess.run(["node", HERE / "cards.mjs"], check=True)
    only = sys.argv[1:]
    made = []
    for s in plan["sections"]:
        i = next(j for j, sc in enumerate(scenes) if sc["name"] == f"plate-{s['app']}")
        target = WORK / f"{s['app']}.mkv"
        if not only or s["app"] in only:
            target = section(s, scenes[i], scenes[i + 1]["first"], plan)
        made.append(target)
    if not only or "film" in only:
        film(made)


if __name__ == "__main__":
    main()
