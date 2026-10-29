"""A release app for the bench, read through its log."""

import datetime
import os
import re
import subprocess
import time

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
APPS = {"ely-doc": "ElyDoc", "ely-table": "ElyTable", "ely-show": "ElyShow", "ely-pdf": "ElyPDF",
        "ely-mail": "ElyMail", "ely-tree": "ElyTree", "ely-mark": "ElyMark"}
# A page that failed to paint voids the run.
PAINT_FAILED = r"lok: tile \(.*\) failed|lok: tile paint failed|pdf: drawing a page failed"
NOISE = re.compile(r"Z (?:DEBUG|TRACE) +gpui::window\]|Z INFO +zbus::")
STAMP = re.compile(r"^\[(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3})Z ")

# This platform's processes and input; set by the caller.
HOST = None


def logged_at(line):
    """Epoch ms a log line was written."""
    found = STAMP.match(line)
    if not found:
        raise RuntimeError(f"a log line without a millisecond stamp: {line[:80]!r}")
    at = datetime.datetime.fromisoformat(found.group(1)).replace(tzinfo=datetime.timezone.utc)
    return at.timestamp() * 1000


def answered(lines, pattern, sent):
    """Ms from `sent` to the first matching line's stamp."""
    line = next((line for line in lines if re.search(pattern, line)), None)
    if line is None:
        return None
    took = logged_at(line) - sent
    if took < 0:
        raise RuntimeError(f"{pattern!r} was logged {-took:.0f} ms before its input")
    return took


def percentile(values, p):
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(len(ordered) * p))]


class App:
    """Timed from spawn, read via its log."""

    def __init__(self, slug, home, args=(), smoke="hold", frames=False):
        self.slug, self.home = slug, home
        os.makedirs(home, exist_ok=True)
        self.log = os.path.join(home, slug, "ely.log")
        # Starts may trim the log; each run starts fresh.
        if os.path.exists(self.log):
            os.remove(self.log)
        env = dict(os.environ, ELY_DATA_ROOT=home, ELY_SMOKE=smoke, ELY_LOCALE="en", ELY_NO_DIALOG="1",
                   ELY_LOG="debug", ELY_STALLS="1")
        if frames:
            env["ELY_FRAMES"] = "1"
        self.env, self.start = env, time.monotonic()
        self.process = subprocess.Popen([HOST.exe(slug, frames), *args], env=env,
                                        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)

    def lines(self):
        if not os.path.exists(self.log):
            return []
        with open(self.log, encoding="utf-8", errors="replace") as log:
            return log.read().splitlines()

    def failed(self, pattern, timeout):
        # Accessibility updates and D-Bus pings hide the app's lines.
        tail = "\n".join([line for line in self.lines() if not NOISE.search(line)][-12:])
        size = os.path.getsize(self.log) if os.path.exists(self.log) else "no file"
        return RuntimeError(f"{self.slug}: no {pattern!r} in {timeout} s; exit {self.process.poll()}, "
                            f"log {size}; it ends:\n{tail}")

    def wait(self, pattern, after=None, timeout=60, since=0):
        """Ms from spawn to a match past `after`, line `since`."""
        called = time.monotonic()
        while time.monotonic() - called < timeout:
            lines = self.lines()[since:]
            if after is not None:
                at = next((i for i, line in enumerate(lines) if re.search(after, line)), None)
                lines = [] if at is None else lines[at + 1:]
            if any(re.search(pattern, line) for line in lines):
                return (time.monotonic() - self.start) * 1000
            time.sleep(0.01)
        raise self.failed(pattern, timeout)

    def answer(self, pattern, sent, since, timeout=30):
        """Ms from the input `sent` to its answer, by clock."""
        called = time.monotonic()
        while time.monotonic() - called < timeout:
            took = answered(self.lines()[since:], pattern, sent)
            if took is not None:
                return took
            time.sleep(0.01)
        raise self.failed(pattern, timeout)

    def ready(self, timeout=60):
        """ELY_SMOKE=1: ms until the first frame."""
        out, _ = self.process.communicate(timeout=timeout)
        took = (time.monotonic() - self.start) * 1000
        if f"ready {APPS[self.slug]}" not in out:
            raise RuntimeError(f"{self.slug} did not report ready: {out!r}")
        # Its engine may still be starting; the next run waits.
        for _ in range(300):
            if not HOST.lingering(self.home):
                break
            time.sleep(0.05)
        return took

    def family(self):
        """Pid to KB of the running app and its children."""
        if self.process.poll() is not None:
            raise RuntimeError(f"{self.slug} exited {self.process.returncode} before it was sampled")
        return HOST.family(self.process.pid)

    def memory_mb(self):
        return sum(self.family().values()) / 1024

    def handles(self):
        return HOST.handles(self.family())

    def front(self):
        HOST.front(self.process.pid, APPS[self.slug])
        time.sleep(0.5)

    def stop(self):
        for pid in HOST.pids(self.process.pid):
            try:
                os.kill(pid, 9)
            except OSError:
                pass
        self.process.wait()
