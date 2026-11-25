"""Window input for the Linux perf tools, by AT-SPI."""

import ctypes
import os
import re
import sys
import time

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402

# Run isolated (-I): only its own folder is added.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import child  # noqa: E402

R = Atspi.Role
SYM = Atspi.KeySynthType.SYM
CONTROL, ENTER = 0xFFE3, 0xFF0D
KEYS = {"pagedown": 0xFF56, "pageup": 0xFF55, "tab": 0xFF09, "escape": 0xFF1B, "space": 0x20, "backspace": 0xFF08,
        "left": 0xFF51, "up": 0xFF52, "right": 0xFF53, "down": 0xFF54, "f5": 0xFFC2, "n": ord("n"), "p": ord("p"),
        "end": 0xFF57}
SHIFT = 0xFFE1
# When the last input left, for bench.py to time from.
SENT = []


def sent():
    SENT.append(time.time() * 1000)


def nodes(node):
    yield node
    for i in range(node.get_child_count()):
        child = node.get_child_at_index(i)
        if child is not None:
            yield from nodes(child)


def app_of(pid):
    desktop = Atspi.get_desktop(0)
    kids = (desktop.get_child_at_index(i) for i in range(desktop.get_child_count()))
    found = next((kid for kid in kids if kid is not None and kid.get_process_id() == pid), None)
    if found is None:
        sys.exit(f"no process {pid} on the accessibility bus")
    return found


def named(pid, name):
    return next((node for node in nodes(app_of(pid)) if node.get_name() == name), None)


def settled(pid, name, timeout=10):
    """The control, once its place holds still inside the window."""
    last, deadline = None, time.monotonic() + timeout
    while time.monotonic() < deadline:
        node = named(pid, name)
        if node is not None:
            box = node.get_extents(Atspi.CoordType.WINDOW)
            place = (box.x, box.y, box.width, box.height)
            if place == last and box.x >= 0 and box.y >= 0:
                return node
            last = place
        time.sleep(0.1)
    if last is None:
        sys.exit(f"no control named {name} in {timeout} s")
    sys.exit(f"{name} never held still inside the window: last at {last}")


def xtest(send):
    """Real input through XTest, as macOS and Windows send."""
    x11, xtst = ctypes.CDLL("libX11.so.6"), ctypes.CDLL("libXtst.so.6")
    x11.XOpenDisplay.restype = ctypes.c_void_p
    display = ctypes.c_void_p(x11.XOpenDisplay(None))
    if not display:
        sys.exit("no X display")
    sent()
    send(x11, xtst, display)
    x11.XFlush(display)
    x11.XCloseDisplay(display)


def click(node):
    box = node.get_extents(Atspi.CoordType.SCREEN)

    def send(x11, xtst, display):
        xtst.XTestFakeMotionEvent(display, -1, box.x + box.width // 2, box.y + box.height // 2, 0)
        for down in (1, 0):
            xtst.XTestFakeButtonEvent(display, 1, down, 0)

    xtest(send)


def chord(letter):
    """Control held while a key goes down and up."""

    def send(x11, xtst, display):
        named = KEYS[letter] if len(letter) > 1 else ord(letter)
        control, key = (x11.XKeysymToKeycode(display, ctypes.c_ulong(sym)) for sym in (CONTROL, named))
        for code, down in ((control, 1), (key, 1), (key, 0), (control, 0)):
            xtst.XTestFakeKeyEvent(display, code, down, 0)

    xtest(send)


def key(name):
    """One named key down and up; `shift-` holds Shift."""
    held, name = (SHIFT, name[6:]) if name.startswith("shift-") else (None, name)

    def send(x11, xtst, display):
        code = x11.XKeysymToKeycode(display, ctypes.c_ulong(KEYS[name]))
        shift = held and x11.XKeysymToKeycode(display, ctypes.c_ulong(held))
        steps = [(shift, 1), (code, 1), (code, 0), (shift, 0)] if shift else [(code, 1), (code, 0)]
        for at, down in steps:
            xtst.XTestFakeKeyEvent(display, at, down, 0)

    xtest(send)


def frame(pid):
    """The app's window on screen: x, y, width, height."""
    app = app_of(pid)
    top = next((node for node in nodes(app) if node.get_role() == R.FRAME), None)
    if top is None:
        sys.exit(f"{app.get_name()} shows no window")
    box = top.get_extents(Atspi.CoordType.SCREEN)
    return box.x, box.y, box.width, box.height


def park(pid):
    """The pointer beside the window, never resting on it."""
    x, y, width, height = frame(pid)
    beside = x - 10 if x > 20 else x + width + 10
    xtest(lambda x11, xtst, display: xtst.XTestFakeMotionEvent(display, -1, beside, y + height // 2, 0))


def front(title):
    """Focuses the window, as a window manager would."""
    tree = child.run(["xwininfo", "-root", "-children"], capture_output=True, check=True).stdout
    tops = re.findall(rf'^\s+(0x[0-9a-f]+) "{re.escape(title)}', tree, re.M)
    if len(tops) != 1:
        sys.exit(f"{len(tops)} top windows named {title}")
    x11 = ctypes.CDLL("libX11.so.6")
    x11.XOpenDisplay.restype = ctypes.c_void_p
    display = ctypes.c_void_p(x11.XOpenDisplay(None))
    x11.XRaiseWindow(display, ctypes.c_ulong(int(tops[0], 16)))
    x11.XSetInputFocus(display, ctypes.c_ulong(int(tops[0], 16)), 1, 0)
    x11.XCloseDisplay(display)
    time.sleep(0.3)


def cover(pid, screen):
    """Gives the app's window on `screen` focus, as WMs do."""
    display = f"{os.environ['DISPLAY']}.{screen}"

    def x(*args):
        return child.run([*args, "-display", display], capture_output=True, check=True).stdout

    size = re.search(r"-geometry (\d+x\d+)", x("xwininfo", "-root")).group(1)
    tops = re.findall(rf"^\s+(0x[0-9a-f]+) .*\s{size}[+-]\d+[+-]\d+\s", x("xwininfo", "-root", "-children"), re.M)
    ours = [top for top in tops if x("xprop", "-id", top, "_NET_WM_PID").split("=")[-1].strip() == str(pid)]
    if len(ours) != 1:
        sys.exit(f"{len(ours)} windows of process {pid} fill screen {screen}")
    x11 = ctypes.CDLL("libX11.so.6")
    x11.XOpenDisplay.restype = ctypes.c_void_p
    handle = ctypes.c_void_p(x11.XOpenDisplay(display.encode()))
    x11.XRaiseWindow(handle, ctypes.c_ulong(int(ours[0], 16)))
    x11.XSetInputFocus(handle, ctypes.c_ulong(int(ours[0], 16)), 1, 0)
    x11.XCloseDisplay(handle)
    time.sleep(0.3)


def scroll(pid, notches, pace):
    """Wheel notches at the window's middle, `pace` ms apart."""
    x, y, width, height = frame(pid)
    button = 5 if notches > 0 else 4

    def send(x11, xtst, display):
        xtst.XTestFakeMotionEvent(display, -1, x + width // 2, y + height // 2, 0)
        x11.XFlush(display)
        for _ in range(abs(notches)):
            for down in (1, 0):
                xtst.XTestFakeButtonEvent(display, button, down, 0)
            x11.XFlush(display)
            time.sleep(pace / 1000)
    xtest(send)


def main():
    command, pid, arg = sys.argv[1], int(sys.argv[2]), (sys.argv[3] if len(sys.argv) > 3 else "")
    if command == "front":
        front(arg)
    elif command == "cover":
        cover(pid, int(arg))
    elif command == "type":
        sent()
        Atspi.generate_keyboard_event(0, arg, Atspi.KeySynthType.STRING)
    elif command == "enter":
        sent()
        Atspi.generate_keyboard_event(ENTER, None, Atspi.KeySynthType.PRESSRELEASE | SYM)
    elif command == "key":
        key(arg)
    elif command == "chord":
        # AT-SPI's Control press never holds: the letter types.
        chord(arg)
    elif command == "settle":
        # gpui clicks a node's centre: a moving one misses.
        settled(pid, arg)
    elif command == "press":
        node = named(pid, arg)
        if node is None:
            sys.exit(f"no control named {arg}")
        action = node.get_action_iface()
        if action is not None and action.get_n_actions() > 0:
            sent()
            action.do_action(0)
        else:
            click(node)
    elif command == "shows":
        print("yes" if named(pid, arg) is not None else "no")
    elif command == "rect":
        print(",".join(map(str, frame(pid))))
    elif command == "park":
        park(pid)
    elif command == "scroll":
        notches, pace = arg.split(",")
        scroll(pid, int(notches), int(pace))
    elif command == "field":
        app, until = app_of(pid), time.monotonic() + 10
        while (field := next((n for n in nodes(app) if n.get_role() == R.ENTRY), None)) is None:
            if time.monotonic() > until:
                sys.exit(f"no text field in {app.get_name()} in 10 s")
            time.sleep(0.1)
        click(field)
    else:
        sys.exit(f"unknown command {command}")
    time.sleep(0.1)
    if SENT:
        print(f"sent {SENT[-1]:.0f}")


main()
