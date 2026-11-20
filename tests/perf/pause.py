#!/usr/bin/env python3
"""An open document outlives a frozen app or worker."""

import argparse
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time

import child
import drive
import run
import soak


def frozen(app, who):
    """The pids to freeze: the app, its worker, or both."""
    pids = [pid for pid in run.HOST.pids(app.process.pid) if pid != app.process.pid]
    workers = [pid for pid in pids if open(f"/proc/{pid}/comm").read().strip() == "ely-lok-worker"]
    if len(workers) != 1:
        raise RuntimeError(f"{len(workers)} workers under the app: {pids}")
    return {"app": [app.process.pid], "worker": workers, "both": [app.process.pid] + workers}[who]


def saved(app, path, word):
    """Saves, then waits for `word` in the file."""
    run.HOST.chord(app.process.pid, "s")
    for _ in range(80):
        if word in soak.words(path):
            return
        time.sleep(0.25)
    raise RuntimeError(f"{word} never reached the file: {soak.words(path)[:12]}; log: {soak.saves(app)}")


class Logind:
    """A private system bus whose logind is ours (`logind.py`)."""

    def __init__(self, work):
        self.address = f"unix:path={os.path.join(work, 'system-bus')}"
        self.bus = subprocess.Popen(["dbus-daemon", "--session", "--nofork", f"--address={self.address}"])
        for _ in range(40):
            if os.path.exists(os.path.join(work, "system-bus")):
                break
            time.sleep(0.05)
        self.seen = open(os.path.join(work, "monitor.log"), "w+", encoding="utf-8")
        self.monitor = subprocess.Popen(["dbus-monitor", "--address", self.address], stdout=self.seen)
        self.logind = subprocess.Popen([sys.executable, "-I", os.path.join(drive.HERE, "logind.py"), self.address],
                                       stdin=subprocess.PIPE, stdout=subprocess.PIPE, encoding="utf-8",
                                       errors="strict")
        if child.line(self.logind).strip() != "ready":
            raise RuntimeError("logind.py never owned org.freedesktop.login1")
        # Only gpui's power listener uses the system bus.
        os.environ["DBUS_SYSTEM_BUS_ADDRESS"] = self.address

    def listener(self):
        """The pid that asked for PrepareForSleep, once one has."""
        for _ in range(100):
            self.seen.seek(0)
            found = re.search(r"sender=(:\S+) -> destination=org\.freedesktop\.DBus.*member=AddMatch\n\s+string \"[^\"]*PrepareForSleep",
                              self.seen.read())
            if found:
                out = drive.sh("gdbus", "call", "--address", self.address, "--dest", "org.freedesktop.DBus",
                               "--object-path", "/org/freedesktop/DBus", "--method",
                               "org.freedesktop.DBus.GetConnectionUnixProcessID", found.group(1))
                return int(re.search(r"\d+", out.split(",")[0].replace("uint32", "")).group())
            time.sleep(0.1)
        raise RuntimeError("nothing on the bus listened for PrepareForSleep")

    def say(self, what):
        self.logind.stdin.write(what + "\n")
        self.logind.stdin.flush()
        if child.line(self.logind).strip() != f"sent {what}":
            raise RuntimeError(f"logind.py did not send {what}")

    def sent(self):
        """The PrepareForSleep values the bus carried, in order."""
        self.seen.seek(0)
        return re.findall(r"member=PrepareForSleep\n\s+boolean (\w+)", self.seen.read())

    def close(self):
        for process in (self.logind, self.monitor, self.bus):
            process.kill()
            process.wait()
        self.seen.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--minutes", type=float, default=10)
    parser.add_argument("--who", choices=["app", "worker", "both"], required=True)
    # gpui hears sleep and wake from logind.
    parser.add_argument("--logind", action="store_true")
    args = parser.parse_args()
    if not sys.platform.startswith("linux"):
        sys.exit("SKIP: the freeze reads /proc")
    work = tempfile.mkdtemp()
    run.HOST = drive.host(work)
    path = os.path.join(work, "a.docx")
    shutil.copy(os.path.join(run.ROOT, "tests", "fixtures", "office", "t.docx"), path)
    logind = Logind(work) if args.logind else None
    app = run.App("ely-doc", os.path.join(work, "home"), [path])
    try:
        app.wait("opened a.docx")
        if logind:
            heard = logind.listener()
            if heard != app.process.pid:
                raise RuntimeError(f"the PrepareForSleep listener is process {heard}, not the app")
            print(f"logind: the app ({heard}) listens for PrepareForSleep", flush=True)
        app.front()
        run.HOST.type(app.process.pid, " Before ")
        saved(app, path, "Before")
        pids, before = frozen(app, args.who), app.private_mb()
        since = len(app.lines())
        if logind:
            logind.say("sleep")
            # logind's inhibitor delay, before the freeze.
            time.sleep(1)
        for pid in pids:
            os.kill(pid, signal.SIGSTOP)
        print(f"froze {args.who} {pids} for {args.minutes} min", flush=True)
        # The freeze itself: a sleeping machine's gap.
        time.sleep(args.minutes * 60)
        for pid in pids:
            os.kill(pid, signal.SIGCONT)
        if logind:
            logind.say("wake")
        app.front()
        run.HOST.type(app.process.pid, " After ")
        saved(app, path, "After")
        found = soak.words(path)
        if found.index("Before") > found.index("After"):
            raise RuntimeError(f"the words came back out of order: {found[:12]}")
        troubles = [line[-160:] for line in app.lines()[since:] if " ERROR " in line or " WARN " in line]
        print(f"ok: {args.who} frozen {args.minutes} min; both words saved; private memory {before:.0f} → {app.private_mb():.0f} MB")
        if logind:
            print(f"logind: the bus carried PrepareForSleep {logind.sent()}", flush=True)
        print("after the freeze, warnings and errors:", *(troubles or ["none"]), sep="\n  ")
    finally:
        app.stop()
        if logind:
            logind.close()
        left = run.stop_engines(work)
        shutil.rmtree(work, ignore_errors=True)
    if left:
        raise RuntimeError(f"engines outlived the run: {left}")


if __name__ == "__main__":
    main()
