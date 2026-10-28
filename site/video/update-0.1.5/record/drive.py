"""Real input, window placement and capture for the video."""
import ctypes
import json
import math
import os
import shutil
import subprocess
import time
import tkinter
from contextlib import contextmanager
from ctypes import wintypes
from pathlib import Path

from PIL import ImageGrab

user32 = ctypes.windll.user32
user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))
user32.SetWindowPos.argtypes = [wintypes.HWND, wintypes.HWND, ctypes.c_int, ctypes.c_int, ctypes.c_int,
                                ctypes.c_int, wintypes.UINT]
ctypes.windll.winmm.timeBeginPeriod(1)

# The captured region, in physical pixels.
X0, Y0, W, H = 1536, 504, 2048, 1152
APPS = Path(os.environ["LOCALAPPDATA"]) / "Programs" / "Ely Office"
DATA = Path(os.environ["LOCALAPPDATA"]) / "ElyOffice"
KEEP = DATA.with_name("ElyOffice.video-keep")
FFMPEG = shutil.which("ffmpeg")


class MOUSEINPUT(ctypes.Structure):
    _fields_ = [("dx", wintypes.LONG), ("dy", wintypes.LONG), ("mouseData", wintypes.DWORD),
                ("dwFlags", wintypes.DWORD), ("time", wintypes.DWORD), ("dwExtraInfo", ctypes.c_size_t)]


class KEYBDINPUT(ctypes.Structure):
    _fields_ = [("wVk", wintypes.WORD), ("wScan", wintypes.WORD), ("dwFlags", wintypes.DWORD),
                ("time", wintypes.DWORD), ("dwExtraInfo", ctypes.c_size_t)]


class _U(ctypes.Union):
    _fields_ = [("mi", MOUSEINPUT), ("ki", KEYBDINPUT), ("pad", ctypes.c_byte * 32)]


class INPUT(ctypes.Structure):
    _fields_ = [("type", wintypes.DWORD), ("u", _U)]


def _send(*inputs):
    arr = (INPUT * len(inputs))(*inputs)
    if user32.SendInput(len(inputs), arr, ctypes.sizeof(INPUT)) != len(inputs):
        raise OSError("SendInput was blocked")


def _mouse(flags, data=0):
    i = INPUT(type=0)
    i.u.mi = MOUSEINPUT(0, 0, data, flags, 0, 0)
    return i


def _key(vk=0, scan=0, flags=0):
    i = INPUT(type=1)
    i.u.ki = KEYBDINPUT(vk, scan, flags, 0, 0)
    return i


VK = {"ctrl": 0x11, "shift": 0x10, "alt": 0x12, "enter": 0x0D, "esc": 0x1B, "tab": 0x09,
      "up": 0x26, "down": 0x28, "left": 0x25, "right": 0x27, "back": 0x08, "space": 0x20,
      "home": 0x24, "end": 0x23, "delete": 0x2E, "=": 0xBB, "-": 0xBD}
EXTENDED = {0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x2D, 0x2E}


class Take:
    """One recorded clip: the capture and its input log."""

    def __init__(self, name, out: Path):
        self.name, self.out = name, out
        self.events = []
        self.zero = None
        self.proc = None

    # Positions are region pixels; the log keeps them so.
    def log(self, kind, **fields):
        self.events.append({"t": round(time.perf_counter() - self.zero, 4), "kind": kind, **fields})

    def start(self):
        path = self.out / f"{self.name}.mp4"
        grab = (f"ddagrab=output_idx=0:framerate=60:offset_x={X0}:offset_y={Y0}:"
                f"video_size={W}x{H}:draw_mouse=0,hwdownload,format=bgra")
        self.proc = subprocess.Popen(
            [FFMPEG, "-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", grab,
             "-c:v", "h264_nvenc", "-preset", "p7", "-tune", "hq", "-rc", "vbr", "-cq", "14",
             "-pix_fmt", "yuv420p", str(path)], stdin=subprocess.PIPE)
        self.zero = time.perf_counter()
        time.sleep(1.5)
        if self.proc.poll() is not None:
            raise RuntimeError("the capture stopped at once")
        # A corner square flashes; a frame diff finds it.
        front = user32.GetForegroundWindow()
        flash = tkinter.Tk()
        flash.overrideredirect(True)
        flash.attributes("-topmost", True)
        flash.configure(bg="#ff00ff")
        flash.geometry(f"60x60+{X0 + W - 90}+{Y0 + H - 90}")
        flash.update()
        self.log("sync", x=W - 60, y=H - 60)
        time.sleep(0.3)
        flash.destroy()
        _send(_key(0x12), _key(0x12, 0, 2))
        user32.SetForegroundWindow(front)
        time.sleep(0.5)
        if user32.GetForegroundWindow() != front:
            raise RuntimeError("the app lost the foreground to the sync flash")

    def stop(self):
        time.sleep(0.8)
        self.proc.communicate(b"q")
        if self.proc.returncode != 0:
            raise RuntimeError(f"the capture failed: {self.proc.returncode}")
        (self.out / f"{self.name}.json").write_text(json.dumps(self.events, ensure_ascii=False, indent=1),
                                                    encoding="utf-8")


TAKE: Take | None = None


def _log(kind, **fields):
    if TAKE is not None and TAKE.zero is not None:
        TAKE.log(kind, **fields)


def where():
    p = wintypes.POINT()
    user32.GetCursorPos(ctypes.byref(p))
    return p.x - X0, p.y - Y0


def _ease(t):
    return 0.5 - 0.5 * math.cos(math.pi * t)


def move_to(x, y, dur=0.6, path=None):
    """Moves the pointer to region (x, y); `path` bends it."""
    sx, sy = where()
    steps = max(1, int(dur * 240))
    start = time.perf_counter()
    for i in range(1, steps + 1):
        f = _ease(i / steps)
        px, py = (path(f) if path else (sx + (x - sx) * f, sy + (y - sy) * f))
        user32.SetCursorPos(int(round(X0 + px)), int(round(Y0 + py)))
        _log("move", x=round(px, 1), y=round(py, 1))
        target = start + i * dur / steps
        while time.perf_counter() < target:
            time.sleep(0.0005)
    user32.SetCursorPos(X0 + int(x), Y0 + int(y))


def press(right=False):
    x, y = where()
    down, up = (0x8, 0x10) if right else (0x2, 0x4)
    _send(_mouse(down))
    _log("down", x=x, y=y, right=right)
    time.sleep(0.07)
    _send(_mouse(up))
    _log("up", x=x, y=y, right=right)


def click(x, y, dur=0.6, right=False, settle=0.12):
    move_to(x, y, dur)
    time.sleep(settle)
    press(right)


def wheel(notches, gap=0.09):
    """Scrolls `notches` (negative is down), one notch at a time."""
    step = -120 if notches < 0 else 120
    for _ in range(abs(notches)):
        _send(_mouse(0x0800, step & 0xFFFFFFFF))
        x, y = where()
        _log("wheel", x=x, y=y, down=notches < 0)
        time.sleep(gap)


def drag(x, y, dur=1.0):
    sx, sy = where()
    _send(_mouse(0x2))
    _log("down", x=sx, y=sy, right=False)
    time.sleep(0.15)
    move_to(x, y, dur)
    time.sleep(0.15)
    _send(_mouse(0x4))
    _log("up", x=x, y=y, right=False)


def key(chord, label=None):
    codes = [VK[k] if k in VK else ord(k.upper()) for k in chord.split("+")]
    downs = [_key(c, 0, 1 if c in EXTENDED else 0) for c in codes]
    ups = [_key(c, 0, (1 if c in EXTENDED else 0) | 2) for c in reversed(codes)]
    _send(*downs)
    _log("key", key=label or chord)
    time.sleep(0.05)
    _send(*ups)


def type_text(text, cps=9.0):
    """Types `text` as Unicode characters at a human pace."""
    for ch in text:
        if ch == "\n":
            key("enter")
        else:
            code = ord(ch)
            _send(_key(0, code, 4), _key(0, code, 4 | 2))
            _log("char", ch=ch)
        time.sleep(1 / cps)


def window_of(pid, timeout=60):
    found = []

    @ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    def each(h, _):
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(h, ctypes.byref(owner))
        if owner.value == pid and user32.IsWindowVisible(h) and not user32.GetWindow(h, 4):
            found.append(h)
        return True

    deadline = time.time() + timeout
    while time.time() < deadline:
        found.clear()
        user32.EnumWindows(each, 0)
        if found:
            return found[0]
        time.sleep(0.2)
    raise RuntimeError(f"process {pid} showed no window")


def place(h):
    user32.ShowWindow(h, 9)
    if not user32.SetWindowPos(h, -1, X0, Y0, W, H, 0x40):
        raise ctypes.WinError()
    # The visible frame, without DWM's invisible border, fills the region.
    seen, outer = wintypes.RECT(), wintypes.RECT()
    ctypes.windll.dwmapi.DwmGetWindowAttribute(h, 9, ctypes.byref(seen), ctypes.sizeof(seen))
    user32.GetWindowRect(h, ctypes.byref(outer))
    left, top = seen.left - outer.left, seen.top - outer.top
    right, bottom = outer.right - seen.right, outer.bottom - seen.bottom
    user32.SetWindowPos(h, -1, X0 - left, Y0 - top, W + left + right, H + top + bottom, 0x40)
    # Alt lets this process take the foreground.
    _send(_key(0x12), _key(0x12, 0, 2))
    user32.SetForegroundWindow(h)
    time.sleep(0.4)
    if user32.GetForegroundWindow() != h:
        raise RuntimeError("the app is not in front; no input sent")


def shot(path, scale=0.5):
    img = ImageGrab.grab(bbox=(X0, Y0, X0 + W, Y0 + H), all_screens=True)
    img.resize((int(W * scale), int(H * scale))).save(path)


def launch(app, exe, *files, wait=6.0):
    env = dict(os.environ, ELY_LOCALE="zh-CN", ELY_APPEARANCE="dark", ELY_LOG="info", ELY_NO_DIALOG="1")
    proc = subprocess.Popen([str(APPS / app / exe), *map(str, files)], env=env)
    h = window_of(proc.pid)
    place(h)
    time.sleep(wait)
    place(h)
    return proc, h


def quit_all():
    for exe in ("ely-doc", "ely-table", "ely-show", "ely-mark", "ely-pdf", "ely-tree"):
        subprocess.run(["taskkill", "/F", "/IM", f"{exe}.exe"], capture_output=True)
    for exe in ("ely-lok-worker", "ely-pdf-worker"):
        subprocess.run(["taskkill", "/F", "/IM", f"{exe}.exe"], capture_output=True)
    time.sleep(1.5)


@contextmanager
def fresh_data():
    """The user's ElyOffice data waits aside; recordings start clean."""
    if KEEP.exists():
        raise RuntimeError(f"{KEEP} exists: restore it by hand first")
    quit_all()
    if DATA.exists():
        DATA.rename(KEEP)
    try:
        yield
    finally:
        quit_all()
        if DATA.exists():
            shutil.rmtree(DATA)
        if KEEP.exists():
            KEEP.rename(DATA)


@contextmanager
def take(name, out: Path):
    global TAKE
    TAKE = Take(name, out)
    TAKE.start()
    try:
        yield TAKE
    finally:
        TAKE.stop()
        TAKE = None
