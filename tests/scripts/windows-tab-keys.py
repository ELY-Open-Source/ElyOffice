"""Real window: the first key after a tab switch lands."""
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

if sys.platform != "win32":
    sys.exit(print("SKIP: real mouse and keys on Windows"))
NAMES = ["a", "b", "c", "d", "e"]
MARK = "Q"
# Ely's own build, wherever Cargo puts it.
for args in (["-p", "ely-lok", "--bin", "ely-lok-worker"], ["-p", "ely-doc"], ["-p", "ely-show"]):
    subprocess.run(["cargo", "build", "-q", *args, "--manifest-path", os.path.join(drive.ROOT, "Cargo.toml")], check=True)
target = os.environ.get("CARGO_TARGET_DIR", os.path.join(drive.ROOT, "target"))
run.HOST = drive.host(tempfile.mkdtemp())
run.HOST.exe = lambda slug, frames: os.path.join(target, "debug", f"{slug}.exe")


def ask(app, *words):
    out = subprocess.run([run.HOST.exe(app.slug, False), *words], env=app.env, capture_output=True,
                         text=True, encoding="utf-8", timeout=120, creationflags=drive.QUIET)
    if out.returncode:
        sys.exit(f"FAIL: {app.slug} {words[0]} {words[1]}: exit {out.returncode}: {out.stderr.strip()}")
    return out.stdout


def loaded(app):
    """The files loaded; reading one loads it."""
    return {m.group(1) for m in (re.search(r": opened (\w)\.\w+ as tab", line) for line in app.lines()) if m}


def marks(app, files, base):
    """Each loaded file's mark count; waiting ones keep `base`."""
    time.sleep(1.5)
    done = loaded(app)
    return {name: ask(app, "text", "get", path).count(MARK) if name in done else base for name, path in files.items()}


def first_key(app, files, into, expected, how, base, act=None):
    """One key right after `act`; it lands in `expected` alone."""
    waiting = expected not in loaded(app)
    before = marks(app, files, base)
    since = len(app.lines())
    if act:
        act()
    if waiting:
        # A click on a waiting tab: its keys wait.
        app.wait(r": tab \d+ picked while it loads; its keys wait", since=since)
    pid = app.process.pid
    if into:
        # Impress: Tab picks a box, Enter edits it.
        run.HOST.key(pid, "tab")
        run.HOST.enter(pid)
    run.HOST.type(pid, MARK)
    if into:
        run.HOST.key(pid, "escape")
        run.HOST.key(pid, "escape")
    held = False
    if waiting:
        app.wait(rf": opened {expected}\.\w+ as tab", since=since)
        held = any("typed while it loaded" in line for line in app.lines()[since:])
    after = marks(app, files, base)
    grew = [name for name in files if after[name] > before[name]]
    if grew != [expected]:
        sys.exit(f"FAIL: {app.slug}: after {how} the key reached {grew or 'nothing'}, not {expected}")
    print(f"ok: {app.slug}: after {how} the first key landed in {expected}{' as it loaded' if held else ''}")


def check(slug, ext, into, sample):
    work = tempfile.mkdtemp()
    files = {}
    for name in NAMES:
        files[name] = os.path.join(work, f"{name}.{ext}")
        shutil.copy(sample, files[name])
    shown = {name: files[name] for name in NAMES[:4]}
    app = run.App(slug, os.path.join(work, "home"), args=list(shown.values()))
    try:
        # The last file asked loads in front; the rest wait.
        opened = r": opened (\w)\.\w+ as tab \d+, .*, (\d+) tabs open, (\d+) waiting, in front true"
        app.wait(rf": opened {NAMES[3]}\.\w+ as tab \d+, .*, in front true", timeout=120)
        time.sleep(2)
        app.front()
        landed = [re.search(opened, line) for line in app.lines() if re.search(opened, line)]
        last = landed[-1].group(1)
        if sum(map(int, landed[-1].groups()[1:])) != 4:
            sys.exit(f"FAIL: {app.slug}: not four tabs: {landed[-1].group(0)}")
        base = ask(app, "text", "get", shown[last]).count(MARK)
        key = lambda expected, how, act=None: first_key(app, shown, into, expected, how, base, act)
        key(last, "opening four files")
        click = lambda name: lambda: run.HOST.tab(app.process.pid, f"{name}.{ext}")
        key("a", "a click on a", click("a"))
        # Clean, so Ctrl+W closes it without asking.
        gone = next(name for name in "cbd" if name != last)
        since = len(app.lines())
        run.HOST.tab(app.process.pid, f"{gone}.{ext}")
        if gone not in loaded(app):
            app.wait(rf": opened {gone}\.\w+ as tab", since=since)
        time.sleep(1)
        since = len(app.lines())
        run.HOST.chord(app.process.pid, "w")
        app.wait(rf"office: closed {gone}\.{ext}", since=since)
        del shown[gone]
        # Tabs keep load order; the last one comes forward.
        key(max(shown), "Ctrl+W on the front tab")
        # Every click switches: there and back.
        for name in sorted(shown) + sorted(shown)[-2::-1]:
            key(name, f"a click on {name}", click(name))
        since = len(app.lines())
        ask(app, files["e"])
        app.wait(rf"opened e\.{ext} as tab", since=since)
        time.sleep(1)
        shown["e"] = files["e"]
        key("e", "opening a file from outside")
        key("a", "a click on a", click("a"))
    finally:
        app.stop()


fixtures = os.path.join(drive.ROOT, "tests", "fixtures", "office")
check("ely-doc", "docx", False, os.path.join(fixtures, "t.docx"))
check("ely-show", "pptx", True, os.path.join(fixtures, "t.pptx"))
print("ok: every tab switch handed the keys to the front document")
