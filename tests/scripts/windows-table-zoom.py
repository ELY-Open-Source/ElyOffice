"""Real window: ElyTable zooms keep the anchor cell still."""
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

from PIL import Image

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "perf"))
import drive
import run

if sys.platform != "win32":
    sys.exit(print("SKIP: real mouse and keys on Windows"))
if len(sys.argv) != 2:
    sys.exit("usage: windows-table-zoom.py <work folder>")
WORK = os.path.abspath(sys.argv[1])
HEADER = r"^(Column [A-Z]+|Row \d+)$"
FAILED = []
for args in (["-p", "ely-lok", "--bin", "ely-lok-worker"], ["-p", "ely-table"]):
    subprocess.run(["cargo", "build", "-q", *args, "--manifest-path", os.path.join(drive.ROOT, "Cargo.toml")], check=True)
target = os.environ.get("CARGO_TARGET_DIR", os.path.join(drive.ROOT, "target"))
run.HOST = drive.host(tempfile.mkdtemp())
run.HOST.exe = lambda slug, frames: os.path.join(target, "debug", f"{slug}.exe")


class Missed(Exception):
    """A step went unchecked: the window unread, or no zoom."""


def headers(app):
    """Shown column and row headers, first first: (name, rect)."""
    found = run.HOST.named(app.process.pid, HEADER)
    columns = sorted(((n, r) for n, r in found.items() if n.startswith("Column")), key=lambda c: c[1][0])
    rows = sorted(((n, r) for n, r in found.items() if n.startswith("Row")), key=lambda c: c[1][1])
    if len(columns) < 3 or len(rows) < 7:
        raise Missed(f"too few headers read: {sorted(found)}")
    return columns, rows


def zoomed(app, since, percent):
    try:
        app.wait(rf"lok: zoom {percent}%", since=since, timeout=30)
    except RuntimeError as why:
        raise Missed(str(why).splitlines()[0])
    # Tiles and headers follow the zoom.
    time.sleep(2)
    return app.lines()[since:]


def fail(why):
    FAILED.append(why)
    print(f"FAIL: {why}")


def shot(app, name):
    """The window's pixels, read by screen point."""
    path = os.path.join(WORK, f"{name.replace(' ', '-')}.png")
    run.HOST.shot(app.process.pid, path)
    window = tuple(map(int, run.HOST.rect(app.process.pid).split(",")))
    image = Image.open(path).convert("RGB")

    def at(x, y):
        (x, y) = (x - window[0], y - window[1])
        if not (0 <= x < image.width and 0 <= y < image.height):
            raise Missed(f"the scan left the window ({path})")
        return image.getpixel((x, y))
    return path, at


def grid_line(pixel):
    return 140 <= pixel[0] <= 230 and max(pixel) - min(pixel) <= 12


def lines(at, start, step, count):
    """Gridline offsets from `start` along `step`; tiles draw them."""
    found = []
    for d in range(1, 2000):
        if grid_line(at(start[0] + step[0] * d, start[1] + step[1] * d)):
            found.append(d)
            if len(found) == count:
                break
    return found


def edge(at, start, step):
    """How far the dark strip runs before the canvas."""
    d = 0
    while min(at(start[0] + step[0] * d, start[1] + step[1] * d)) < 140:
        d += 1
    return d


def a1(app, how, logged):
    """A1 whole at the top left, framed by the cursor."""
    path, at = shot(app, how)
    columns, rows = headers(app)
    names = (columns[0][0], rows[0][0])
    # Empty row 3 and column C: no glyph, no cursor.
    (c3, r3) = (columns[2][1], rows[2][1])
    (across, down) = (c3[0] + c3[2] // 2, r3[1] + r3[3] // 2)
    x0 = rows[0][1][0] + rows[0][1][2] - 4
    x0 += edge(at, (x0, down), (1, 0))
    y0 = columns[0][1][1] + columns[0][1][3] - 4
    y0 += edge(at, (across, y0), (0, 1))
    xs, ys = lines(at, (x0, down), (1, 0), 2), lines(at, (across, y0), (0, 1), 2)
    if len(xs) < 2 or len(ys) < 2:
        return fail(f"{how}: gridlines not found: {xs}, {ys} ({path})")
    sizes = ((xs[0], xs[1] - xs[0]), (ys[0], ys[1] - ys[0]))
    start = re.findall(r"view starts at \((-?\d+), (-?\d+)\)", "".join(logged))
    print(f"{how}: first {names}; A, B wide {sizes[0]}; rows 1, 2 high {sizes[1]}; logged start {start} ({path})")
    if names != ("Column A", "Row 1"):
        fail(f"{how}: the view starts at {names}")
    # Equal cells; A1 counts from the canvas, 1 px short.
    if sizes[0][1] - sizes[0][0] not in (0, 1) or sizes[1][1] - sizes[1][0] not in (0, 1):
        fail(f"{how}: A1 is cut: A, B {sizes[0]}, rows 1, 2 {sizes[1]}")
    # The frame lies on A1's lines, 3 px inward.
    (gx, gy) = (x0 + xs[0], y0 + ys[0])
    frame = at(x0 + 1, y0 + ys[0] // 4)
    same = lambda pixel: all(abs(a - b) <= 30 for a, b in zip(pixel, frame))
    sides = {"top": at(x0 + xs[0] // 4, y0 + 1), "right": at(gx - 1, y0 + ys[0] // 4), "bottom": at(x0 + xs[0] // 4, gy - 1)}
    beyond = {"right": at(gx + 3, y0 + 1), "below": at(x0 + 1, gy + 3)}
    if frame[2] < frame[0] + 80 or not all(map(same, sides.values())) or any(map(same, beyond.values())):
        fail(f"{how}: the cell cursor is not A1's: left {frame}, {sides}, beyond {beyond}")
    if start[-1:] != [("0", "0")]:
        fail(f"{how}: the view logged its start at {start}, not (0, 0)")


def step(app, how, percent, act):
    since = len(app.lines())
    act()
    try:
        a1(app, how, zoomed(app, since, percent))
    except Missed as why:
        fail(f"{how}: {why}")


def reset(app):
    """Back to 100 % and A1; a precondition."""
    since = len(app.lines())
    run.HOST.chord(app.process.pid, "0")
    zoomed(app, since, 100)
    # A still cursor scrolls nothing: away, then home.
    run.HOST.key(app.process.pid, "pagedown")
    run.HOST.chord(app.process.pid, "home")
    time.sleep(2)
    columns, rows = headers(app)
    if (columns[0][0], rows[0][0]) != ("Column A", "Row 1"):
        sys.exit(f"FAIL: Ctrl+0 and Ctrl+Home left the view at {columns[0][0]}, {rows[0][0]}")


def floor(app, act):
    """Down to 25 %, where A1 is the top left."""
    since = len(app.lines())
    act()
    zoomed(app, since, 25)


def under(app, at):
    """The column and row header spanning screen point `at`."""
    columns, rows = headers(app)
    column = next(n for n, c in columns if c[0] <= at[0] < c[0] + c[2])
    row = next(n for n, r in rows if r[1] <= at[1] < r[1] + r[3])
    return column, row


def corner(app, name, spot):
    """Pixels from `spot` back to its cell's lines."""
    path, at = shot(app, name)
    return [lines(at, spot, way, 1) for way in ((-1, 0), (0, -1))], path


os.makedirs(WORK, exist_ok=True)
book = os.path.join(WORK, "t.xlsx")
shutil.copy(os.path.join(drive.ROOT, "tests", "fixtures", "office", "t.xlsx"), book)
app = run.App("ely-table", os.path.join(WORK, "home"), args=[book])
try:
    app.wait(r"opened t\.xlsx as tab", timeout=120)
    time.sleep(2)
    app.front()
    pid = app.process.pid
    reset(app)
    keys = lambda name, times: lambda: [run.HOST.chord(pid, name) for _ in range(times)]
    # Past the double-click time: each press is its own.
    clicks = lambda name, times: lambda: [(run.HOST.click(pid, name), time.sleep(0.7)) for _ in range(times)]
    # Zooming out clamps at A1 anyway: every step zooms in.
    for how, act in (("keys", keys), ("control", clicks)):
        out, into = ("minus", "plus") if how == "keys" else ("Zoom out", "Zoom in")
        floor(app, act(out, 4))
        step(app, f"{how} 67", 67, act(into, 2))
        step(app, f"{how} 125", 125, act(into, 3))
        step(app, f"{how} 400", 400, act(into, 4))
        reset(app)
    # Ctrl+wheel up from 25 %, over A1's middle each notch.
    floor(app, keys("minus", 4))
    for percent in (50, 67, 80, 100, 125, 150, 200, 300, 400):
        columns, rows = headers(app)
        middle = (columns[0][1][0] + columns[0][1][2] // 2, rows[0][1][1] + rows[0][1][3] // 2)
        step(app, f"wheel {percent}", percent, lambda: run.HOST.wheel(pid, 1, middle))
    # Elsewhere, the pointed cell's corner stays put.
    reset(app)
    columns, rows = headers(app)
    spot = (columns[2][1][0] + 5, rows[6][1][1] + 5)
    for percent, notch in ((125, 1), (150, 1), (125, -1), (100, -1)):
        since = len(app.lines())
        try:
            before, (was, _) = under(app, spot), corner(app, f"before {percent}", spot)
            run.HOST.wheel(pid, notch, spot)
            zoomed(app, since, percent)
            (now, path), after = corner(app, f"pointed {percent}", spot), under(app, spot)
        except Missed as why:
            fail(f"pointed {percent}: {why}")
            break
        print(f"pointed {percent}: {before} became {after}; its lines {was} px away became {now} ({path})")
        if after != before or now != was:
            fail(f"wheel to {percent}% moved {before} from the pointer")
except Missed as why:
    # Later steps chain on this zoom: stop.
    fail(f"the run stopped: {why}")
finally:
    app.stop()
if FAILED:
    sys.exit(f"FAIL: {len(FAILED)} checks failed")
print("ok: every zoom kept its anchor cell")
