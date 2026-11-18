"""Real window: every show key turns one slide, both windows."""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "perf"))
import drive
import run

NEXT = ["right", "down", "space", "pagedown", "n"]
BACK = ["left", "up", "pageup", "backspace", "p"]
# Keysyms as wtype names them.
SYMS = {"right": "Right", "left": "Left", "up": "Up", "down": "Down", "space": "space", "pagedown": "Next",
        "pageup": "Prior", "backspace": "BackSpace", "n": "n", "p": "p", "f5": "F5", "escape": "Escape",
        "tab": "Tab", "enter": "Return"}
# A desktop giving new show windows no keyboard.
NESTED = """monitor = , 1920x1080@60, 0x0, 1
misc {
    disable_hyprland_logo = true
    disable_splash_rendering = true
    focus_on_activate = true
}
animations {
    enabled = false
}
windowrule = match:class ^ely-show$, match:title ^$, no_initial_focus on
"""
# Set inside linux.sh's Xvfb, which has no window manager.
INSIDE_X = "ELY_SHOW_KEYS_X"


def fail(why):
    sys.exit(f"FAIL: {why}")


class Nested:
    """Linux: a Hyprland inside the user's, on a hidden workspace."""

    def __init__(self, work):
        if not os.environ.get("HYPRLAND_INSTANCE_SIGNATURE") or not shutil.which("wtype"):
            sys.exit(print("SKIP: the Wayland half runs inside a Hyprland session with wtype"))
        config = os.path.join(work, "hypr.conf")
        with open(config, "w") as out:
            out.write(NESTED)
        before = {i["instance"] for i in self.instances()}
        command = f"Hyprland -c {config} > {os.path.join(work, 'hypr.log')} 2>&1"
        # Hidden, never the user's focus; Lua configs take Lua.
        hidden = "special:ely-show-keys silent"
        for line in (f"exec [workspace {hidden}] {command}", f"hl.dsp.exec_cmd('{command}', {{ workspace = '{hidden}' }})"):
            said = subprocess.run(["hyprctl", "dispatch", line], capture_output=True, text=True).stdout.strip()
            if said == "ok":
                break
        else:
            fail(f"the nested Hyprland did not start: {said}")
        until = time.monotonic() + 20
        while not (new := [i for i in self.instances() if i["instance"] not in before]):
            if time.monotonic() > until:
                fail("no nested Hyprland in 20 s")
            time.sleep(0.2)
        self.sig, self.pid, self.socket = new[0]["instance"], new[0]["pid"], new[0]["wl_socket"]
        # A second screen gives the presenter its own.
        self.hypr("output", "create", "headless")

    @staticmethod
    def instances():
        return json.loads(drive.sh("hyprctl", "instances", "-j"))

    def hypr(self, *args):
        return drive.sh("hyprctl", "-i", self.sig, *args)

    def active(self):
        return json.loads(self.hypr("activewindow", "-j")).get("address")

    def key(self, pid, name):
        shift = name.startswith("shift-")
        down, up = (["-M", "shift"], ["-m", "shift"]) if shift else ([], [])
        name = name.removeprefix("shift-")
        env = dict(os.environ, WAYLAND_DISPLAY=self.socket)
        subprocess.run(["wtype", *down, "-k", SYMS[name], *up], env=env, check=True)

    def press(self, pid, name):
        if name != "Start":
            fail(f"no keys press {name}")
        # Start is the screen dialog's last stop.
        self.key(pid, "shift-tab")
        self.key(pid, "enter")

    def presenter(self, pid):
        """Focuses the show window without the keyboard, as a click."""
        shows = [c for c in json.loads(self.hypr("clients", "-j")) if c["pid"] == pid and not c["title"]]
        other = next(c["address"] for c in shows if c["address"] != self.active())
        self.hypr("dispatch", "focuswindow", f"address:{other}")
        if self.active() != other:
            fail(f"the nested Hyprland did not focus {other}")

    def stop(self):
        os.kill(self.pid, 15)


def turns(app, pid, keys, part):
    """Each key moves the show one slide from `part`."""
    step = 1 if keys is NEXT else -1
    for key in keys:
        since = len(app.lines())
        run.HOST.key(pid, key)
        part += step
        try:
            app.wait(rf"show: goes to Slide \{{ part: {part}, clicks: 0 \}}", since=since, timeout=10)
        except RuntimeError:
            gone = [line for line in app.lines()[since:] if "show:" in line]
            fail(f"{key} did not turn to slide {part + 1}: {gone or 'the show logged nothing'}")
        print(f"ok: {key} turns to slide {part + 1}")
    return part


def play(nested):
    """F5, the ten keys in each show window, Escape."""
    deck = os.path.join(work, "deck.pptx")
    shutil.copy(os.path.join(drive.ROOT, "tests", "fixtures", "office", "t.pptx"), deck)
    app = run.App("ely-show", os.path.join(work, "home"), [deck])
    pid = app.process.pid
    try:
        app.wait(r"opened deck\.pptx as tab", timeout=120)
        # Hidden, the nested deck paints no tile.
        app.wait(r"lok: first tile drawn|show: tab 0 has \d+ parts", timeout=60)
        if not nested:
            app.front()
        run.HOST.key(pid, "f5")
        try:
            app.wait(r"show: \d+ windows open", timeout=15)
        except RuntimeError:
            # Two screens ask where the show goes.
            run.HOST.press(pid, "Start")
            app.wait(r"show: \d+ windows open", timeout=30)
        windows = int(re.findall(r"show: (\d+) windows open", "\n".join(app.lines()))[-1])
        app.wait(r"show: the slide is drawn", timeout=30)
        print(f"ok: the show opens {windows} windows")
        print("audience window:")
        turns(app, pid, BACK, turns(app, pid, NEXT, 0))
        if windows == 1:
            print("SKIP: one display here; no presenter window")
        elif not sys.platform.startswith("linux"):
            print(f"SKIP: no presenter window focus on {run.HOST.name}")
        else:
            # As a click on it would.
            if nested:
                nested.presenter(pid)
            else:
                run.HOST.cover(pid, 1)
            print("presenter window:")
            turns(app, pid, BACK, turns(app, pid, NEXT, 0))
        since = len(app.lines())
        run.HOST.key(pid, "escape")
        app.wait(r"show: ends", since=since, timeout=10)
        print("ok: escape ends the show")
    finally:
        app.stop()
        # The show's idle successor outlives the app.
        subprocess.run([run.HOST.exe("ely-show", False), "engine", "stop"], env=app.env, capture_output=True,
                       creationflags=drive.QUIET, check=True)


for args in (["-p", "ely-lok", "--bin", "ely-lok-worker"], ["-p", "ely-show"]):
    subprocess.run(["cargo", "build", "-q", *args, "--manifest-path", os.path.join(drive.ROOT, "Cargo.toml")], check=True)
meta = subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1", "--manifest-path",
                       os.path.join(drive.ROOT, "Cargo.toml")], capture_output=True, text=True, check=True).stdout
target = json.loads(meta)["target_directory"]
linux, inside_x = sys.platform.startswith("linux"), os.environ.get(INSIDE_X) == "1"
if linux and not inside_x:
    # X11 first: two Xvfb screens, keys by XTest.
    print("X11 without a window manager:")
    tool = os.path.relpath(os.path.abspath(__file__), os.path.join(drive.ROOT, "tests", "perf"))
    env = dict(os.environ, ELY_PERF_TOOL=tool, ELY_PERF_SCREENS="2", ELY_PERF_DISPLAY="21", **{INSIDE_X: "1"})
    code = subprocess.run(["bash", os.path.join(drive.ROOT, "tests", "perf", "linux.sh")], env=env).returncode
    if code:
        sys.exit(code)
    print("Wayland in a nested Hyprland:")
work = tempfile.mkdtemp()
nested = Nested(work) if linux and not inside_x else None
run.HOST = drive.Linux() if linux else drive.host(work)
run.HOST.exe = lambda slug, frames: os.path.join(target, "debug", slug + (".exe" if os.name == "nt" else ""))
if nested:
    run.HOST.key, run.HOST.press = nested.key, nested.press
    os.environ["WAYLAND_DISPLAY"] = nested.socket
    os.environ.pop("DISPLAY", None)
try:
    play(nested)
finally:
    if nested:
        nested.stop()
shutil.rmtree(work, ignore_errors=True)
