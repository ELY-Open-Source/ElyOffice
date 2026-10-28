#!/usr/bin/env python3
"""PERF-01 to PERF-08 on release builds, written as a record."""

import argparse
import collections
import datetime
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import time

import samples

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
APPS = {"ely-doc": "ElyDoc", "ely-table": "ElyTable", "ely-show": "ElyShow",
        "ely-pdf": "ElyPDF", "ely-tree": "ElyTree", "ely-mark": "ElyMark"}
TARGETS = {"PERF-01": 3000, "PERF-02": 100, "PERF-03": 5000, "PERF-04": 5000,
           "PERF-05": 3000, "PERF-06": 1000, "PERF-07": 2000}
# Total memory budgets, MB, at the opening mark (H-02).
BUDGETS = {"PERF-07": 400}


def sh(*args, **kw):
    return subprocess.run(args, check=True, capture_output=True, text=True, **kw).stdout


def tools(folder):
    for tool in ["ax-values", "ax-press", "click", "chord"]:
        sh("swiftc", "-O", "-o", os.path.join(folder, tool), os.path.join(ROOT, "tests", "scripts", tool + ".swift"))


class App:
    """A release app run, timed from spawn, read via log."""

    def __init__(self, slug, home, args=(), smoke="hold"):
        self.slug, self.home, self.tools = slug, home, os.path.dirname(home)
        self.log = os.path.join(home, "Library", "Application Support", "ElyOffice", slug, "ely.log")
        # Starts may trim the log; each run starts fresh.
        if os.path.exists(self.log):
            os.remove(self.log)
        self.seen = 0
        env = dict(os.environ, HOME=home, ELY_SMOKE=smoke, ELY_LOCALE="en", ELY_NO_DIALOG="1",
                   ELY_LOG="debug", ELY_STALLS="1")
        self.start = time.monotonic()
        self.process = subprocess.Popen([os.path.join(ROOT, "target", "release", slug), *args], env=env,
                                        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)

    def lines(self):
        if not os.path.exists(self.log):
            return []
        with open(self.log, errors="replace") as log:
            log.seek(self.seen)
            return log.read().splitlines()

    def wait(self, pattern, after=None, timeout=60):
        """Milliseconds from spawn until a line matches, past `after`."""
        called = time.monotonic()
        while time.monotonic() - called < timeout:
            lines = self.lines()
            if after is not None:
                at = next((i for i, line in enumerate(lines) if re.search(after, line)), None)
                lines = [] if at is None else lines[at + 1:]
            if any(re.search(pattern, line) for line in lines):
                return (time.monotonic() - self.start) * 1000
            time.sleep(0.01)
        tail = "\n".join(self.lines()[-12:])
        raise RuntimeError(f"{self.slug}: no {pattern!r} in {timeout} s; the log ends:\n{tail}")

    def ready(self, timeout=60):
        """ELY_SMOKE=1: milliseconds until the first frame."""
        out, _ = self.process.communicate(timeout=timeout)
        took = (time.monotonic() - self.start) * 1000
        if f"ready {APPS[self.slug]}" not in out:
            raise RuntimeError(f"{self.slug} did not report ready: {out!r}")
        # Its engine may still be starting; the next run waits.
        for _ in range(300):
            if subprocess.run(["pgrep", "-f", self.home], capture_output=True).returncode != 0:
                break
            time.sleep(0.05)
        return took

    def family(self):
        out = sh("ps", "-A", "-o", "pid=,ppid=,rss=")
        rows = [tuple(map(int, line.split())) for line in out.splitlines()]
        ours, grew = {self.process.pid}, True
        while grew:
            grew = False
            for pid, ppid, _ in rows:
                if ppid in ours and pid not in ours:
                    ours.add(pid)
                    grew = True
        return ours, sum(rss for pid, _, rss in rows if pid in ours)

    def memory_mb(self):
        return self.family()[1] / 1024

    def handles(self):
        """Open handles of the app and its children, as type and name."""
        ours, _ = self.family()
        found = collections.Counter()
        for pid in ours:
            kind = None
            for line in sh("lsof", "-p", str(pid), "-F", "tn").splitlines():
                if line[:1] == "t":
                    kind = line[1:]
                elif line[:1] == "n":
                    found[(kind, line[1:])] += 1
        return found

    def front(self):
        sh("osascript", "-e", f'tell application "System Events" to set frontmost of (first process whose unix id is {self.process.pid}) to true')
        time.sleep(0.5)

    def ax(self):
        return sh(os.path.join(self.tools, "ax-values"), str(self.process.pid))

    def stop(self):
        ours, _ = self.family()
        for pid in ours:
            try:
                os.kill(pid, 9)
            except ProcessLookupError:
                pass
        self.process.wait()


def keys(script):
    sh("osascript", "-e", 'tell application "System Events"', "-e", script, "-e", "end tell")


def percentile(values, p):
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(len(ordered) * p))]


# A page that failed to paint voids the run.
PAINT_FAILED = r"lok: tile \(.*\) failed|lok: tile paint failed|pdf: drawing a page failed"


def opened(slug, home, sample, marks, runs):
    """Spawn with a file until `marks` all appear, in order."""
    times, memory = [], []
    for _ in range(runs):
        app = App(slug, home, [sample])
        try:
            after = None
            for mark in marks:
                took = app.wait(mark, after)
                after = mark
            failed = [line for line in app.lines() if re.search(PAINT_FAILED, line)]
            if failed:
                raise RuntimeError(f"{slug}: {sample} did not paint: {failed[0]}")
            times.append(took)
            memory.append(app.memory_mb())
        finally:
            app.stop()
    return times, memory


def laid_out(work, sample, pages):
    """The kernel lays `sample` out to exactly `pages` pages."""
    app = App("ely-doc", os.path.join(work, "home-pages"), [sample])
    try:
        app.wait(rf"lok: {pages} pages$", timeout=120)
        time.sleep(3)
        counts = [line for line in app.lines() if re.search(r"lok: \d+ pages$", line)]
        if not counts[-1].endswith(f" {pages} pages"):
            raise RuntimeError(f"{sample} lays out as {counts[-1]}, not {pages} pages")
    finally:
        app.stop()


def perf01(work, runs):
    rows = []
    for slug in APPS:
        home = os.path.join(work, "home-" + slug)
        first = App(slug, home, smoke="1").ready()
        daily = [App(slug, home, smoke="1").ready() for _ in range(runs)]
        rows.append((slug, first, daily))
    return rows


def perf02(work, runs):
    """Key to new tile in ElyDoc, and main-thread stalls meanwhile."""
    app = App("ely-doc", os.path.join(work, "home-typing"))
    try:
        app.wait("opened Untitled")
        app.front()
        tick = r"lok: edit to tile (\d+) ms"
        for n in range(runs):
            keys('keystroke "x"')
            # Each key must show; a lost one fails the run.
            for _ in range(500):
                if sum(1 for line in app.lines() if re.search(tick, line)) > n:
                    break
                time.sleep(0.01)
            else:
                raise RuntimeError(f"ely-doc: key {n + 1} of {runs} never reached a tile")
            time.sleep(0.15)
        lines = app.lines()
    finally:
        app.stop()
    failed = [line for line in lines if re.search(PAINT_FAILED, line)]
    if failed:
        raise RuntimeError(f"ely-doc: a tile failed to draw while typing: {failed[0]}")
    ticks = [int(m.group(1)) for line in lines if (m := re.search(tick, line))]
    if len(ticks) != runs:
        raise RuntimeError(f"ely-doc: {runs} keys gave {len(ticks)} timings")
    stalls = [int(m.group(1)) for line in lines if (m := re.search(r"main thread was busy (\d+) ms", line))]
    return ticks, stalls


def perf06(work, repo, runs):
    """History first screen, then a common query's first results."""
    home = os.path.join(work, "home-ely-tree")
    first, found = [], []
    for _ in range(runs):
        app = App("ely-tree", home, [repo])
        try:
            app.wait("tree: opened")
            app.front()
            pressed = (time.monotonic() - app.start) * 1000
            sh(os.path.join(work, "ax-press"), str(app.process.pid), "History")
            first.append(app.wait(r"history shows \d+ commits, searched false") - pressed)
            field = next(line for line in app.ax().splitlines() if line.startswith("AXTextField"))
            x, y = field.rsplit("at=", 1)[1].split(",")
            sh(os.path.join(work, "click"), x, y)
            keys('keystroke "part 42"')
            asked = (time.monotonic() - app.start) * 1000
            keys("key code 36")
            found.append(app.wait(r"searched true") - asked)
        finally:
            app.stop()
    return first, found


def raise_main(app):
    """The document window as key, once a dialog closed."""
    sh("osascript", "-e", f'tell application "System Events" to tell (first process whose unix id is {app.process.pid}) '
       f'to perform action "AXRaise" of window "{APPS[app.slug]}"')
    time.sleep(0.3)


def documents_open(lines):
    """The kernel's last document count; none logged fails."""
    counts = [int(m.group(1)) for line in lines if (m := re.search(r"lok-worker: (\d+) documents open", line))]
    if not counts:
        raise RuntimeError("ely-doc: the worker logged no document count")
    return counts[-1]


def handles_kept(before, now, extra, cycle):
    """Handles past the baseline: only system ones, the same each cycle."""
    grown = now - before
    # Read-only system resources load once, on first use.
    left = sorted(handle for handle in grown if not handle[1].startswith("/System/Library/"))
    if left:
        raise RuntimeError(f"ely-doc: cycle {cycle} leaves handles: {left}")
    if extra is not None and grown != extra:
        raise RuntimeError(f"ely-doc: cycle {cycle} changed the system handles: {sorted((grown - extra) + (extra - grown))}")
    return grown


def perf08(work, cycles=40):
    """New and close a document; memory, handles and live documents."""
    app = App("ely-doc", os.path.join(work, "home-cycles"))

    def open_docs(since=0):
        return documents_open(app.lines()[since:])

    try:
        app.wait("opened Untitled")
        app.front()
        time.sleep(2)
        before = app.handles()
        baseline = (app.memory_mb(), sum(before.values()), open_docs())
        seen, extra = [], None
        for n in range(cycles):
            cursor = len(app.lines())
            sh(os.path.join(work, "chord"), str(app.process.pid), "45")
            for _ in range(40):
                if "title=Use template" in app.ax():
                    break
                time.sleep(0.25)
            sh(os.path.join(work, "ax-press"), str(app.process.pid), "Use template", "AXButton")
            app.wait(f"opened Untitled {n + 2} ", timeout=30)
            # This cycle's own report, not an earlier one.
            if open_docs(cursor) != baseline[2] + 1:
                raise RuntimeError(f"ely-doc: cycle {n + 1} opened, yet {open_docs(cursor)} documents")
            cursor = len(app.lines())
            time.sleep(1)
            # A keystroke can miss a window still taking focus.
            for _ in range(3):
                app.front()
                raise_main(app)
                sh(os.path.join(work, "chord"), str(app.process.pid), "13")
                try:
                    app.wait(f"office: closed Untitled {n + 2}$", timeout=5)
                    break
                except RuntimeError:
                    # A keystroke that lost its Command edits the document.
                    if "title=Don't Save" in app.ax():
                        raise RuntimeError(f"ely-doc: Untitled {n + 2} was edited by a stray key; it asks to save")
                    continue
            else:
                said = [line for line in app.lines() if re.search("clos|dialog|opened|focus", line)]
                raise RuntimeError(f"ely-doc: Untitled {n + 2} never closed: {said[-8:]}")
            # The kernel lets go later; a fresh report must come.
            for _ in range(100):
                try:
                    if open_docs(cursor) == baseline[2]:
                        break
                except RuntimeError:
                    pass
                time.sleep(0.05)
            else:
                raise RuntimeError(f"ely-doc: cycle {n + 1} closed, yet no report of {baseline[2]} documents")
            time.sleep(0.5)
            now = app.handles()
            extra = handles_kept(before, now, extra, n + 1)
            seen.append((app.memory_mb(), sum(now.values()), open_docs(cursor)))
    finally:
        app.stop()
    return baseline, seen

def environment(found):
    gpu = re.findall(r"Chipset Model: (.+)", sh("system_profiler", "SPDisplaysDataType"))
    fonts = sum(len(files) for folder in ["/System/Library/Fonts", "/Library/Fonts"] for _, _, files in os.walk(folder))
    builds = {slug: hashlib.sha256(open(os.path.join(ROOT, "target", "release", slug), "rb").read()).hexdigest()[:16]
              for slug in APPS}
    return {
        "date": datetime.date.today().isoformat(),
        "machine": sh("sysctl", "-n", "hw.model").strip(),
        "cpu": sh("sysctl", "-n", "machdep.cpu.brand_string").strip(),
        "cores": sh("sysctl", "-n", "hw.ncpu").strip(),
        "memory_gb": int(sh("sysctl", "-n", "hw.memsize")) // 2**30,
        "gpu": ", ".join(gpu),
        "system": f"macOS {platform.mac_ver()[0]}",
        "fonts": fonts,
        "builds": builds,
        "samples": {name: value[1] for name, value in found.items()},
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=30)
    parser.add_argument("--only", nargs="*")
    parser.add_argument("--out", default=os.path.join(ROOT, "docs", "perf"))
    args = parser.parse_args()
    want = lambda perf: not args.only or perf in args.only
    if sys.platform != "darwin":
        sys.exit("SKIP: the bench drives macOS windows")
    sh("cargo", "build", "--release", "-q", "--bins", "-p", "ely-lok", "-p", "ely-pdf-engine",
       *[arg for slug in APPS for arg in ("-p", slug)], cwd=ROOT)
    git = os.path.join(ROOT, "engines", "git", "bin", "git")
    found = samples.made(os.path.join(ROOT, "engines", "bench"), git)
    work = tempfile.mkdtemp()
    tools(work)
    rows = []
    try:
        if want("PERF-01"):
            for slug, first, daily in perf01(work, args.runs):
                rows.append(("PERF-01", f"{slug} first run", [first], None))
                rows.append(("PERF-01", f"{slug} daily start", daily, None))
            # The window opens first; the blank document follows the engine.
            for slug in ["ely-doc", "ely-table", "ely-show"]:
                blank = []
                for _ in range(args.runs):
                    app = App(slug, os.path.join(work, "home-" + slug))
                    try:
                        blank.append(app.wait("opened Untitled"))
                    finally:
                        app.stop()
                rows.append(("PERF-01", f"{slug} blank document ready", blank, None))
        if want("PERF-02"):
            ticks, stalls = perf02(work, args.runs)
            rows.append(("PERF-02", "ElyDoc key to new tile", ticks, None))
            rows.append(("PERF-02", f"main thread busy >100 ms: {len(stalls)} times", stalls or [0], None))
        cases = [("PERF-03", "ely-doc", "doc.docx", ["opened doc.docx", r"lok: first tile drawn"]),
                 ("PERF-04", "ely-table", "sheet.xlsx", ["opened sheet.xlsx", r"lok: first tile drawn"]),
                 ("PERF-05", "ely-pdf", "pages.pdf", ["opened pages.pdf", r"pdf: first tile drawn, page 0$"]),
                 ("PERF-07", "ely-mark", "notes.md", ["opened notes.md", r"mark: shown as \"visual\""])]
        if want("PERF-03"):
            laid_out(work, found["doc.docx"][0], 100)
        for perf, slug, name, marks in cases:
            if want(perf):
                times, memory = opened(slug, os.path.join(work, "home-" + slug), found[name][0], marks, args.runs)
                rows.append((perf, f"{APPS[slug]} {name}", times, max(memory)))
        if want("PERF-06"):
            first, searched = perf06(work, found["repo"][0], args.runs)
            rows.append(("PERF-06", "ElyTree history, press to first screen", first, None))
            rows.append(("PERF-06", "ElyTree query 'part 42', first results", searched, None))
        held = perf08(work) if want("PERF-08") else None
    finally:
        shutil.rmtree(work, ignore_errors=True)
    write(args.out, environment(found), rows, held, args.runs)


def write(out, env, rows, held, runs):
    os.makedirs(out, exist_ok=True)
    path = os.path.join(out, f"{env['date']}-{env['machine']}.md")
    lines = [f"# Benchmark {env['date']}", "", "Format: docs/perf/README.md.", "", "```json",
             json.dumps(env, indent=2), "```", "",
             "| PERF | Case | Runs | P50 ms | P95 ms | Target ms | Total memory MB | Budget MB |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for perf, case, times, memory in rows:
        target = TARGETS.get(perf)
        p95 = percentile(times, 0.95)
        verdict = "" if target is None or "busy" in case else (" ✓" if p95 <= target else " ✗")
        budget = BUDGETS.get(perf) if memory is not None else None
        within = "" if budget is None else (" ✓" if memory <= budget else " ✗")
        mem = "" if memory is None else f"{memory:.0f}{within}"
        lines.append(f"| {perf} | {case} | {len(times)} | {percentile(times, 0.5):.0f} | {p95:.0f}{verdict} | "
                     f"{target or ''} | {mem} | {budget or ''} |")
    if held:
        (memory, files, docs), seen = held
        lines += ["", f"PERF-08, ElyDoc creating and closing a document {len(seen)} times "
                  "(total memory MB / open files / kernel documents):", "",
                  f"before: {memory:.0f}/{files}/{docs}", "",
                  " ".join(f"{memory:.0f}/{files}/{docs}" for memory, files, docs in seen)]
    lines += ["", "PERF-09 (Agent sessions) belongs to the second release.", ""]
    with open(path, "w") as record:
        record.write("\n".join(lines))
    print(path)


if __name__ == "__main__":
    main()
