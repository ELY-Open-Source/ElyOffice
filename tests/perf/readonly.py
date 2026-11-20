#!/usr/bin/env python3
"""QA-06: a read-only folder refuses the save; nothing is lost."""

import os
import shutil
import signal
import sys
import tempfile
import time

import drive
import pause
import run


def logged(app, since, text, seconds=30):
    """The first line after `since` holding `text`, and where."""
    for _ in range(seconds * 4):
        found = [(at, line) for at, line in enumerate(app.lines()) if at >= since and text in line]
        if found:
            return found[0]
        time.sleep(0.25)
    raise RuntimeError(f"no log line with {text!r} in {seconds} s")


def main():
    if not sys.platform.startswith("linux"):
        sys.exit("SKIP: the folder's mode and the screenshot are Linux's")
    work = tempfile.mkdtemp()
    run.HOST = drive.host(work)
    folder, home = os.path.join(work, "docs"), os.path.join(work, "home")
    os.makedirs(folder)
    path = os.path.join(folder, "a.docx")
    shutil.copy(os.path.join(run.ROOT, "tests", "fixtures", "office", "t.docx"), path)
    shot = os.environ.get("ELY_SHOT")
    app = run.App("ely-doc", home, [path])
    try:
        app.wait("opened a.docx")
        app.front()
        run.HOST.type(app.process.pid, " Before ")
        pause.saved(app, path, "Before")
        with open(path, "rb") as file:
            kept = file.read()
        os.chmod(folder, 0o555)
        since = len(app.lines())
        run.HOST.type(app.process.pid, " After ")
        time.sleep(0.5)
        run.HOST.chord(app.process.pid, "s")
        print("the app logged:", logged(app, since, "save failed")[1][-200:], flush=True)
        # The toast stays a moment; the frame is evidence.
        time.sleep(1)
        if shot:
            drive.sh("import", "-window", "root", shot)
            print(f"screenshot: {shot}", flush=True)
        with open(path, "rb") as file:
            if file.read() != kept:
                raise RuntimeError("the refused save changed the file")
        if os.listdir(folder) != ["a.docx"]:
            raise RuntimeError(f"the refused save left files: {os.listdir(folder)}")
        print("the file is as last saved; nothing else in the folder", flush=True)
        # A recovery point holds the unsaved words.
        logged(app, since, " point at ", 40)
        for pid in run.HOST.pids(app.process.pid):
            os.kill(pid, signal.SIGKILL)
        app.process.wait()
        app = run.App("ely-doc", home)
        logged(app, 0, "recovery: the last run ended Crashed", 60)
        back, _ = logged(app, 0, "recovery: reopening a docx", 60)
        # Its notice names the file, so goes unlogged.
        print("after a crash:", logged(app, back, ": Some(Warning)", 60)[1][-160:], flush=True)
        # Only the window shows an untitled copy's text.
        time.sleep(2)
        if shot:
            drive.sh("import", "-window", "root", shot.replace(".png", "-recovered.png"))
        with open(path, "rb") as file:
            if file.read() != kept:
                raise RuntimeError("the recovery changed the original file")
        print("the original file is still as last saved", flush=True)
    except Exception:
        # The app's log outlives a failed run.
        kept_log = tempfile.mkdtemp(prefix="ely-readonly-failed-")
        shutil.copy(app.log, kept_log)
        print(f"the app's log is in {kept_log}", flush=True)
        raise
    finally:
        os.chmod(folder, 0o755)
        app.stop()
        left = run.stop_engines(work)
        shutil.rmtree(work, ignore_errors=True)
    if left:
        raise RuntimeError(f"engines outlived the run: {left}")
    print("ok: a read-only folder refuses the save; the file is kept, the edits come back as a copy")


if __name__ == "__main__":
    main()
