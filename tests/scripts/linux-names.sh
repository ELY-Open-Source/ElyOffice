#!/usr/bin/env bash
# Every control AT-SPI reaches in seven apps has a name.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname -s)" = Linux ] || { echo "SKIP: not Linux"; exit 0; }
for tool in docker dbus-run-session xwininfo git; do
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

work=$root/target/a11y-names-linux
bin=${CARGO_TARGET_DIR:-$root/target}/debug
rm -rf "$work"
mkdir -p "$work"
(cd "$root" && cargo build -q -p ely-doc -p ely-table -p ely-show -p ely-pdf -p "path+file://$root/apps/ely-mail" \
  -p ely-tree -p ely-mark -p ely-lok -p ely-pdf-worker --bins)
. "$root/tests/scripts/a11y-samples.sh"
a11y_samples "$work"

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

# What Orca reads, through AT-SPI.
cat >"$work/walk.py" <<'EOF'
import ctypes, re, subprocess, sys, time

import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi, Gio, GLib

R = Atspi.Role
ACTS = {R.PUSH_BUTTON, R.TOGGLE_BUTTON, R.CHECK_BOX, R.RADIO_BUTTON, R.COMBO_BOX, R.ENTRY, R.PASSWORD_TEXT,
        R.LIST_ITEM, R.MENU_ITEM, R.PAGE_TAB, R.TREE_ITEM, R.SLIDER, R.SPIN_BUTTON, R.LINK, R.DOCUMENT_FRAME}
TAB, F6 = 0xFF09, 0xFFC3
exe, path, page, out = sys.argv[1:5]
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


def focused(app):
    return next((n for n in nodes(app) if n.get_state_set().contains(Atspi.StateType.FOCUSED)), None)


def is_page(node):
    return bool(page) and node.get_name() == page and node.get_role() in (R.ENTRY, R.DOCUMENT_FRAME)


def page_of(node):
    while node is not None:
        if is_page(node):
            return node
        node = node.get_parent()
    return None


def key_of(node):
    parent = node.get_parent()
    return (node.get_role_name(), node.get_name(), node.get_index_in_parent(), parent.get_name() if parent else "")


def press(key):
    Atspi.generate_keyboard_event(key, None, Atspi.KeySynthType.PRESSRELEASE | Atspi.KeySynthType.SYM)
    time.sleep(0.4)
    return focused(app)


def top():
    tree = subprocess.run(["xwininfo", "-root", "-children"], capture_output=True, text=True, check=True).stdout
    tops = re.findall(r'^\s+(0x[0-9a-f]+) "Ely', tree, re.M)
    if len(tops) != 1:
        raise SystemExit(f"error {len(tops)} top windows")
    return tops[0]


def placed():
    """The frame's screen place is the X window's: Orca points there."""
    info = subprocess.run(["xwininfo", "-id", top()], capture_output=True, text=True, check=True).stdout
    x, y = (int(re.search(rf"Absolute upper-left {axis}:\s+(-?\d+)", info).group(1)) for axis in "XY")
    if (x, y) == (0, 0):
        raise SystemExit("error the window sits at the screen origin; placement proves nothing")
    box = app.get_child_at_index(0).get_extents(Atspi.CoordType.SCREEN)
    # gpui-linux creates the window 2 px right of its bounds.
    if abs(box.x - x) > 2 or abs(box.y - y) > 1:
        log.append(f"misplaced frame at {box.x},{box.y}; the window is at {x},{y}")


def give_focus():
    """Focuses the app's window, as a window manager would."""
    x11 = ctypes.CDLL("libX11.so.6")
    x11.XOpenDisplay.restype = ctypes.c_void_p
    display = ctypes.c_void_p(x11.XOpenDisplay(None))
    x11.XSetInputFocus(display, ctypes.c_ulong(int(top(), 16)), 1, 0)
    x11.XCloseDisplay(display)
    time.sleep(0.5)


proc = subprocess.Popen([exe] + ([path] if path != "-" else []), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
try:
    for _ in range(120):
        app = app_of(proc.pid)
        if app is not None and app.get_child_count() > 0:
            break
        time.sleep(0.5)
    else:
        raise SystemExit("error no app on the bus")
    time.sleep(6)
    pages = 0
    for node in nodes(app):
        pages += is_page(node)
        if node.get_role() in ACTS and not node.get_name():
            log.append(f"unnamed {node.get_role_name()}")
    if page and not pages:
        log.append(f"no page [{page}]")
    placed()
    give_focus()
    seen, last, stops, reached = set(), None, 0, False
    # Where F6 landed: a stop, checked before Tab moves on.
    landed = None
    for _ in range(150):
        tabbed = landed is None
        node = press(TAB) if tabbed else landed
        landed = None
        # A document keeps Tab; F6 leaves it.
        if tabbed and node is not None and key_of(node) == last:
            node = press(F6)
            if node is not None and key_of(node) == last:
                log.append(f"trapped in {last}")
                break
        if node is None:
            log.append(f"no focus after stop {stops}")
            break
        key = key_of(node)
        if key in seen:
            break
        seen.add(key)
        last, stops = key, stops + 1
        if not key[1]:
            log.append(f"nameless stop {stops} {key[0]}")
        # At the page, F6 must take focus out of it.
        if not reached and page_of(node) is not None:
            reached = True
            after = press(F6)
            if after is None:
                log.append("no focus after F6")
                break
            if page_of(after) is not None:
                log.append(f"F6 stays in page [{page}] at {key_of(after)}")
            landed = after
    # Tab must lead back to the page.
    if page and not reached:
        log.append(f"unreached page [{page}]")
    log.append(f"stops {stops}")
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
launcher=$1 work=$2 bin=$3 apps=$4
"$launcher" --launch-immediately >"$work/atspi.log" 2>&1 &
for _ in $(seq 40); do
  python3 -I -c "
from gi.repository import Gio, GLib
Gio.bus_get_sync(Gio.BusType.SESSION).call_sync(\"org.a11y.Bus\", \"/org/a11y/bus\", \"org.freedesktop.DBus.Properties\",
    \"Set\", GLib.Variant(\"(ssv)\", (\"org.a11y.Status\", \"IsEnabled\", GLib.Variant(\"b\", True))), None, 0, -1, None)
" 2>/dev/null && break
  sleep 0.25
done
failed=0
while IFS="|" read -r app sample page; do
  mkdir -p "$work/data-$app"
  file=-
  [ "$sample" = - ] || file=$work/$sample
  ELY_DATA_ROOT=$work/data-$app python3 -I "$work/walk.py" "$bin/$app" "$file" "$page" "$work/$app.log" 2>/dev/null </dev/null
  stops=$(sed -n "s/^stops //p" "$work/$app.log")
  if grep -qE "^(unnamed|nameless|unreached|no page|trapped|no focus|misplaced|F6 stays|error)" "$work/$app.log" || [ "${stops:-0}" -lt 3 ]; then
    echo "FAIL: $app"
    cat "$work/$app.log"
    failed=1
  else
    echo "ok: $app, $stops Tab stops"
  fi
done <<<"$apps"
exit $failed'
unset WAYLAND_DISPLAY
# The page names are English.
ELY_GIT=${ELY_GIT:-$(command -v git)} ELY_NO_DIALOG=1 ELY_LOCALE=en-US dbus-run-session -- bash -c "$inside" _ "$launcher" "$work" "$bin" "$(a11y_apps)" 2>"$work/session.log" ||
  { echo "FAIL: see $work" >&2; exit 1; }
echo "PASS: seven apps name every control and every Tab stop, at its screen place; Tab reaches each document, F6 leaves it"
