"""Real window: a name typed without extension saves as .docx."""
import os
import sys
import tempfile
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "perf"))
import drive
import run

if sys.platform != "win32":
    sys.exit(print("SKIP: the Windows save dialog"))
work = tempfile.mkdtemp()
run.HOST = drive.host(work)
run.HOST.exe = lambda slug, frames=False: os.path.join(run.ROOT, "target", "debug", slug + ".exe")
target = os.path.join(work, "report")
kept = target + ".docx"


def save_as(app):
    """New document, Ctrl+S, `target` typed with no extension."""
    since = len(app.lines())
    run.HOST.chord(app.process.pid, "s")
    app.wait("office: asks where to save", since=since)
    time.sleep(2)
    run.HOST.type(app.process.pid, target)
    time.sleep(0.5)
    run.HOST.enter(app.process.pid)


app = run.App("ely-doc", os.path.join(work, "home"))
try:
    app.wait("opened Untitled 1 ")
    app.front()
    time.sleep(1)
    save_as(app)
    app.wait(r"office: saved .*report\.docx", timeout=30)
    if not os.path.isfile(kept) or os.path.exists(target):
        sys.exit(f"FAIL: wrote {sorted(os.listdir(work))}, not report.docx")
    # The dialog asks before replacing the .docx it added.
    since = len(app.lines())
    run.HOST.chord(app.process.pid, "w")
    app.wait("office: closed report.docx", since=since)
    with open(kept, "wb") as old:
        old.write(b"OLD")
    since = len(app.lines())
    run.HOST.chord(app.process.pid, "n")
    run.HOST.settle(app.process.pid, "Use template")
    run.HOST.press(app.process.pid, "Use template")
    app.wait("opened Untitled 2 ", since=since)
    since = len(app.lines())
    save_as(app)
    time.sleep(3)
    if open(kept, "rb").read() != b"OLD" or any("office: saved" in line for line in app.lines()[since:]):
        sys.exit("FAIL: report.docx was replaced without asking")
    run.HOST.type(app.process.pid, "y")
    app.wait(r"office: saved .*report\.docx", since=since, timeout=30)
    if open(kept, "rb").read() == b"OLD":
        sys.exit("FAIL: confirming did not replace report.docx")
finally:
    app.stop()
print("ok: Save As added .docx to a bare name and asked before replacing it")
