"""Real window: a name typed without extension saves as .docx."""
import os
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "perf"))
import drive
import run

if sys.platform != "win32":
    sys.exit(print("SKIP: the Windows save dialog"))
# The build under test, wherever Cargo puts it.
for args in (["-p", "ely-lok", "--bin", "ely-lok-worker"], ["-p", "ely-doc"]):
    subprocess.run(["cargo", "build", "-q", *args, "--manifest-path", os.path.join(drive.ROOT, "Cargo.toml")], check=True)
exe = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(drive.ROOT, "target")), "debug", "ely-doc.exe")
if not os.path.isfile(exe):
    sys.exit(f"FAIL: no build at {exe}")
work = tempfile.mkdtemp()
run.HOST = drive.host(work)
run.HOST.exe = lambda slug, frames: exe
target = os.path.join(work, "report")
kept = target + ".docx"


def save_as(app, after):
    """Ctrl+S on a new document, `target` typed with no extension."""
    run.HOST.chord(app.process.pid, "s")
    app.wait("office: asks where to save", after=after)
    time.sleep(2)
    run.HOST.type(app.process.pid, target)
    time.sleep(0.5)
    run.HOST.enter(app.process.pid)


def saved(app, after):
    return [line for line in app.lines()[after:] if "office: saved" in line]


app = run.App("ely-doc", os.path.join(work, "home"))
try:
    app.wait("opened Untitled 1 ")
    app.front()
    save_as(app, "opened Untitled 1 ")
    app.wait(r"office: saved .*report\.docx", timeout=30)
    if not os.path.isfile(kept) or os.path.exists(target):
        sys.exit(f"FAIL: wrote {sorted(os.listdir(work))}, not report.docx")
    # The dialog asks before replacing the .docx it added.
    run.HOST.chord(app.process.pid, "w")
    app.wait("office: closed report.docx")
    with open(kept, "wb") as old:
        old.write(b"OLD")
    run.HOST.chord(app.process.pid, "n")
    for _ in range(100):
        if run.HOST.shows(app.process.pid, "Use template"):
            break
        time.sleep(0.1)
    else:
        sys.exit("FAIL: the new document dialog never showed")
    run.HOST.press(app.process.pid, "Use template")
    app.wait("opened Untitled 2 ")
    since = len(app.lines())
    save_as(app, "opened Untitled 2 ")
    time.sleep(3)
    with open(kept, "rb") as now:
        if now.read() != b"OLD" or saved(app, since):
            sys.exit("FAIL: report.docx was replaced without asking")
    run.HOST.type(app.process.pid, "y")
    app.wait(r"office: saved .*report\.docx", after="opened Untitled 2 ", timeout=30)
    with open(kept, "rb") as now:
        if now.read() == b"OLD":
            sys.exit("FAIL: confirming did not replace report.docx")
finally:
    app.stop()
print("ok: Save As added .docx to a bare name and asked before replacing it")
