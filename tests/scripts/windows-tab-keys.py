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


def marks(app, files):
    """Each open file's count of the typed mark."""
    time.sleep(1.5)
    return {name: ask(app, "text", "get", path).count(MARK) for name, path in files.items()}


def first_key(app, files, into, expected, how):
    """One key after `how`; it must land in `expected` alone."""
    before = marks(app, files)
    pid = app.process.pid
    if into:
        # Impress: Tab picks a box, Enter edits it.
        run.HOST.key(pid, "tab")
        run.HOST.enter(pid)
    run.HOST.type(pid, MARK)
    if into:
        run.HOST.key(pid, "escape")
        run.HOST.key(pid, "escape")
    after = marks(app, files)
    grew = [name for name in files if after[name] > before[name]]
    if grew != [expected]:
        sys.exit(f"FAIL: {app.slug}: after {how} the key reached {grew or 'nothing'}, not {expected}")
    print(f"ok: {app.slug}: after {how} the first key landed in {expected}")


def check(slug, ext, into, sample):
    work = tempfile.mkdtemp()
    files = {}
    for name in NAMES:
        files[name] = os.path.join(work, f"{name}.{ext}")
        shutil.copy(sample, files[name])
    shown = {name: files[name] for name in NAMES[:4]}
    app = run.App(slug, os.path.join(work, "home"), args=list(shown.values()))
    try:
        opened = r": opened (\w)\.\w+ as tab \d+, risk .*, 4 tabs open"
        app.wait(opened, timeout=120)
        time.sleep(2)
        app.front()
        # The last load to end is in front.
        last = next(re.search(opened, line).group(1) for line in app.lines() if re.search(opened, line))
        first_key(app, shown, into, last, "opening four files")
        run.HOST.tab(app.process.pid, f"a.{ext}")
        first_key(app, shown, into, "a", "a click on a")
        # Clean, so Ctrl+W closes it without asking.
        gone = next(name for name in "cbd" if name != last)
        run.HOST.tab(app.process.pid, f"{gone}.{ext}")
        time.sleep(1)
        since = len(app.lines())
        run.HOST.chord(app.process.pid, "w")
        app.wait(rf"office: closed {gone}\.{ext}", since=since)
        del shown[gone]
        # Tabs keep load order; the last one comes forward.
        first_key(app, shown, into, max(shown), "Ctrl+W on the front tab")
        # Every click switches: there and back.
        for name in sorted(shown) + sorted(shown)[-2::-1]:
            run.HOST.tab(app.process.pid, f"{name}.{ext}")
            first_key(app, shown, into, name, f"a click on {name}")
        since = len(app.lines())
        ask(app, files["e"])
        app.wait(rf"opened e\.{ext} as tab", since=since)
        time.sleep(1)
        shown["e"] = files["e"]
        first_key(app, shown, into, "e", "opening a file from outside")
        run.HOST.tab(app.process.pid, f"a.{ext}")
        first_key(app, shown, into, "a", "a click on a")
    finally:
        app.stop()


fixtures = os.path.join(drive.ROOT, "tests", "fixtures", "office")
check("ely-doc", "docx", False, os.path.join(fixtures, "t.docx"))
check("ely-show", "pptx", True, os.path.join(fixtures, "t.pptx"))
print("ok: every tab switch handed the keys to the front document")
