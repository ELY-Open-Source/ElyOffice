#!/usr/bin/env bash
# Orca reaches ElyDoc's and ElyPDF's text and selection, through AT-SPI.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname -s)" = Linux ] || { echo "SKIP: not Linux"; exit 0; }
for tool in docker dbus-run-session xwininfo; do
  command -v "$tool" >/dev/null || { echo "SKIP: no $tool"; exit 0; }
done
python3 -I -c 'import gi; gi.require_version("Atspi", "2.0")' 2>/dev/null || { echo "SKIP: no AT-SPI for Python"; exit 0; }
launcher=
for at in /usr/libexec/at-spi-bus-launcher /usr/lib/at-spi-bus-launcher /usr/lib/at-spi2-core/at-spi-bus-launcher; do
  [ -x "$at" ] && launcher=$at && break
done
[ -n "$launcher" ] || { echo "SKIP: no at-spi-bus-launcher"; exit 0; }
[ -n "${ELY_LOK_DIR:-}" ] || [ -d "$root/engines/lo-src/instdir/program" ] || { echo "SKIP: no LOK build"; exit 0; }
[ -n "${ELY_MUPDF_DIR:-}" ] || [ -d "$root/engines/mupdf-build" ] || { echo "SKIP: no MuPDF build"; exit 0; }

work=$root/target/a11y-text-linux
bin=${CARGO_TARGET_DIR:-$root/target}/debug
rm -rf "$work"
mkdir -p "$work/data"
(cd "$root" && cargo build -q -p ely-doc -p ely-lok -p ely-pdf -p ely-pdf-worker --bins)
. "$root/tests/scripts/a11y-samples.sh"
a11y_text_samples "$work"

# An X server with XTest; no desktop's focus rules.
docker build -q -t ely-xvfb - >/dev/null <<'EOF'
FROM ubuntu:26.04@sha256:f144425ff09be612d6d9ad965196e9cdc23dae1f42110a8a11a3e9a8198759f7
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends xvfb \
  && rm -rf /var/lib/apt/lists/*
EOF
xvfb=$(docker run -d --rm --network host ely-xvfb Xvfb :9 -listen tcp -ac -screen 0 1280x800x24)
trap 'docker stop -t 1 "$xvfb" >/dev/null' EXIT
export DISPLAY=127.0.0.1:9
for _ in $(seq 40); do xwininfo -root >/dev/null 2>&1 && break; sleep 0.25; done
xwininfo -root >/dev/null || { docker logs "$xvfb" >&2; exit 1; }

# AT-SPI's Text interface is what Orca reads.
cat >"$work/read.py" <<'EOF'
import ctypes, re, subprocess, sys, time

import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi

kind, exe, path, out = sys.argv[1:5]
CTRL, SHIFT, HOME, END, RIGHT = 0xFFE3, 0xFFE1, 0xFF50, 0xFF57, 0xFF53
log = []


def nodes(node):
    yield node
    for i in range(node.get_child_count()):
        child = node.get_child_at_index(i)
        if child is not None:
            yield from nodes(child)


def app_of(pid):
    desktop = Atspi.get_desktop(0)
    kids = (desktop.get_child_at_index(i) for i in range(desktop.get_child_count()))
    return next((kid for kid in kids if kid is not None and kid.get_process_id() == pid), None)


def text_node():
    for _ in range(120):
        app = app_of(proc.pid)
        found = app and next((n for n in nodes(app) if "Text" in n.get_interfaces()), None)
        if found is not None:
            return found
        time.sleep(0.5)
    raise SystemExit("error no node offers text")


def xtest(send):
    """Real input through XTest: AT-SPI's modifier presses never hold."""
    x11, xtst = ctypes.CDLL("libX11.so.6"), ctypes.CDLL("libXtst.so.6")
    x11.XOpenDisplay.restype = ctypes.c_void_p
    display = ctypes.c_void_p(x11.XOpenDisplay(None))
    send(x11, xtst, display)
    x11.XFlush(display)
    x11.XCloseDisplay(display)
    time.sleep(0.3)


def keys(*held_then_key, times=1):
    *held, key = held_then_key

    def send(x11, xtst, display):
        code = lambda sym: x11.XKeysymToKeycode(display, ctypes.c_ulong(sym))
        for down in held:
            xtst.XTestFakeKeyEvent(display, code(down), 1, 0)
        for _ in range(times):
            for down in (1, 0):
                xtst.XTestFakeKeyEvent(display, code(key), down, 0)
        for down in reversed(held):
            xtst.XTestFakeKeyEvent(display, code(down), 0, 0)

    xtest(send)


def state(label):
    time.sleep(1.5)
    node = text_node()
    text = Atspi.Text.get_text(node, 0, Atspi.Text.get_character_count(node))
    picks = (Atspi.Text.get_selection(node, i) for i in range(Atspi.Text.get_n_selections(node)))
    chosen = "+".join(Atspi.Text.get_text(node, r.start_offset, r.end_offset) for r in picks)
    row = f"{label}|{node.get_role_name()}|{text.replace(chr(10), '/')}|{chosen.replace(chr(10), '/')}"
    # The caret in characters, as Orca reads it.
    log.append(row + (f"|{Atspi.Text.get_caret_offset(node)}" if kind == "doc" else ""))


def focus_window():
    """Focuses the app's window, as a window manager would."""
    tree = subprocess.run(["xwininfo", "-root", "-children"], capture_output=True, text=True, check=True).stdout
    tops = re.findall(r'^\s+(0x[0-9a-f]+) "Ely', tree, re.M)
    if len(tops) != 1:
        raise SystemExit(f"error {len(tops)} top windows")
    x11 = ctypes.CDLL("libX11.so.6")
    x11.XOpenDisplay.restype = ctypes.c_void_p
    display = ctypes.c_void_p(x11.XOpenDisplay(None))
    x11.XSetInputFocus(display, ctypes.c_ulong(int(tops[0], 16)), 1, 0)
    x11.XCloseDisplay(display)
    time.sleep(0.5)


def drag(box, x0, y0, x1, y1):
    """From and to, in widths of `box`."""
    at = lambda x, y: (int(box.x + box.width * x), int(box.y + box.width * y))

    def send(x11, xtst, display):
        xtst.XTestFakeMotionEvent(display, -1, *at(x0, y0), 0)
        xtst.XTestFakeButtonEvent(display, 1, 1, 200)
        for step in range(1, 11):
            xtst.XTestFakeMotionEvent(display, -1, *at(x0 + (x1 - x0) * step / 10, y0 + (y1 - y0) * step / 10), 50)
        xtst.XTestFakeButtonEvent(display, 1, 0, 50)

    xtest(send)


def click(box):
    def send(x11, xtst, display):
        xtst.XTestFakeMotionEvent(display, -1, box.x + box.width // 2, box.y + box.height // 2, 0)
        for down in (1, 0):
            xtst.XTestFakeButtonEvent(display, 1, down, 0)

    xtest(send)


proc = subprocess.Popen([exe, path], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
try:
    view = text_node()
    time.sleep(3)
    focus_window()
    box = view.get_extents(Atspi.CoordType.SCREEN)
    if kind == "doc":
        click(box)
        keys(CTRL, HOME)
        state("caret")
        keys(SHIFT, END)
        state("line")
        keys(HOME)
        keys(RIGHT, times=9)
        state("moved")
        keys(SHIFT, RIGHT)
        state("emoji")
        keys(RIGHT)
        keys(SHIFT, RIGHT, times=4)
        state("end")
    elif kind == "pdf":
        state("page")
        drag(box, 0.1, box.height / 2 / box.width, 0.9, box.height / 2 / box.width)
        state("picked")
    else:
        # At fit width a page is 0.4 widths tall.
        drag(box, 0.1, 0.2, 0.9, 0.6)
        state("across")
        keys(END)
        state("scrolled")
except SystemExit as stop:
    log.append(str(stop))
finally:
    proc.kill()
    proc.wait()
    with open(out, "w", encoding="utf-8") as f:
        f.write("\n".join(log) + "\n")
EOF

# A private session bus with accessibility on.
inside='set -euo pipefail
launcher=$1 work=$2 bin=$3
"$launcher" --launch-immediately >"$work/atspi.log" 2>&1 &
for _ in $(seq 40); do
  python3 -I -c "
from gi.repository import Gio, GLib
Gio.bus_get_sync(Gio.BusType.SESSION).call_sync(\"org.a11y.Bus\", \"/org/a11y/bus\", \"org.freedesktop.DBus.Properties\",
    \"Set\", GLib.Variant(\"(ssv)\", (\"org.a11y.Status\", \"IsEnabled\", GLib.Variant(\"b\", True))), None, 0, -1, None)
" 2>/dev/null && break
  sleep 0.25
done
read_app() { ELY_DATA_ROOT=$work/data python3 -I "$work/read.py" "$1" "$bin/$2" "$work/$3" "$work/$1.log" </dev/null; }
read_app doc ely-doc probe.fodt
read_app pdf ely-pdf probe.pdf
read_app pages ely-pdf pages.pdf'
unset WAYLAND_DISPLAY
ELY_NO_DIALOG=1 ELY_LOCALE=en-US dbus-run-session -- bash -c "$inside" _ "$launcher" "$work" "$bin" 2>"$work/session.log" ||
  { echo "FAIL: see $work" >&2; exit 1; }
cat >"$work/want.log" <<'EOF'
caret|entry|Hello 世界 😀 end||0
line|entry|Hello 世界 😀 end|Hello 世界 😀 end|14
moved|entry|Hello 世界 😀 end||9
emoji|entry|Hello 世界 😀 end|😀|10
end|entry|Hello 世界 😀 end|end|14
page|document frame|Hello PDF reader/second line/|
picked|document frame|Hello PDF reader/second line/|Hello PDF reader
across|document frame|page one/page two/|page one/page two
scrolled|document frame|page two/page three/|page two
EOF
cat "$work/doc.log" "$work/pdf.log" "$work/pages.log" | diff "$work/want.log" - || { echo "FAIL: the reader saw other text" >&2; exit 1; }
echo "PASS: ElyDoc's paragraph and ElyPDF's page, with caret and selection, reach AT-SPI"
