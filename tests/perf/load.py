#!/usr/bin/env python3
"""PERF-10: many documents open and edited at once, per app."""

import argparse
import concurrent.futures
import ctypes
import datetime
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import threading
import time

import drive
import feedback
import loadfiles
import richfiles
import run
import search

from loadfiles import TOKEN
from run import APPS, App, percentile

FRAMES = re.compile(r"ui: frames (\d+) \(lost (\d+)\):.*dirty to shown p50 \S+ p95 (\S+) max (\S+) ms; over 8\.3 ms (\d+)")
WORKER = re.compile(r"lok-worker: (\d+) documents open")
KEY_WAIT = 10
TARGET = 100
# The user's floor: stop before the machine swaps.
FLOOR_GB = 12
# Clicks and pointer moves the bench never makes while typing.
OTHERS = re.compile(r"ui: mouse_down to frame|lok: input \d+ mouse")
RETRIES = 2


class Interfered(RuntimeError):
    """Someone else used the desktop: the run is void."""


class NoFeedback(RuntimeError):
    """A key showed nothing in time."""


class Memory(ctypes.Structure):
    _fields_ = [("size", ctypes.c_ulong), ("load", ctypes.c_ulong)] + [
        (name, ctypes.c_ulonglong) for name in ("total", "free", "page", "page_free", "virtual", "virtual_free", "extended")]


def free_now():
    status = Memory()
    status.size = ctypes.sizeof(Memory)
    if not ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(status)):
        raise RuntimeError("free memory could not be read")
    return status.free / 2**30


class Floor(threading.Thread):
    """Free memory twice a second, through every phase."""

    def __init__(self):
        super().__init__(daemon=True)
        self.lowest, self.stop = free_now(), None

    def run(self):
        while self.stop is None:
            try:
                free = free_now()
            except RuntimeError as error:
                self.stop = str(error)
                return
            self.lowest = min(self.lowest, free)
            if free < FLOOR_GB:
                self.stop = f"stopped: free memory {free:.1f} GB under {FLOOR_GB} GB"
            time.sleep(0.5)


FLOOR = None


def watch_memory():
    global FLOOR
    FLOOR = Floor()
    FLOOR.start()


def free_gb():
    """Lowest free memory seen; raises once under the floor."""
    if FLOOR.stop:
        raise RuntimeError(FLOOR.stop)
    return FLOOR.lowest


def finish_enter(pid):
    run.HOST.enter(pid)


def finish_escape(pid):
    run.HOST.key(pid, "escape")


def finish_nothing(pid):
    """The typed text is already in the document."""


def replaced(path, r):
    return [["text", "replace", path, f"{TOKEN}{r - 1}", f"{TOKEN}{r}"]]


def set_cells(path, r):
    """H1 counts rounds; A1 feeds every formula."""
    return [["cell", "set", path, "Data!H1", str(r)], ["cell", "set", path, "Data!A1", str(1000 + r)]]


class Recipe(feedback.Keys):
    def __init__(self, slug, answer, into, finish, edits, key="x"):
        super().__init__(slug, [], None, answer, into, key)
        self.finish, self.edits = finish, edits


RECIPES = {
    "ely-doc": Recipe("ely-doc", feedback.TILE, feedback.into_document, finish_nothing, replaced),
    "ely-table": Recipe("ely-table", feedback.TILE, feedback.into_document, finish_enter, set_cells, key="1"),
    "ely-show": Recipe("ely-show", feedback.TILE, feedback.into_box, finish_escape, replaced),
    "ely-mark": Recipe("ely-mark", feedback.FRAME, feedback.into_paragraph, finish_nothing, replaced),
}


def ask(app, words):
    """Ms the app's command line took; raises on refusal."""
    began = time.monotonic()
    out = subprocess.run([run.HOST.exe(app.slug, True), *words], env=app.env, capture_output=True, text=True,
                         timeout=300, creationflags=drive.QUIET)
    took = (time.monotonic() - began) * 1000
    if out.returncode:
        raise RuntimeError(f"{' '.join(words[:2])} {os.path.basename(words[2])}: exit {out.returncode}: {out.stderr.strip()}")
    return took


def refused_for(app, words, most=300):
    """Retries a refused save: seconds refused, the refusal."""
    began, why = time.monotonic(), ""
    while True:
        try:
            ask(app, words)
            return time.monotonic() - began, why
        except RuntimeError as error:
            if time.monotonic() - began > most:
                raise
            why = str(error)[-90:]
            time.sleep(5)


def at_once(app, jobs):
    """Each job's words asked together; times and failures."""
    times, failures = [], []
    with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, len(jobs))) as pool:
        for future in [pool.submit(ask, app, words) for words in jobs]:
            try:
                times.append(future.result())
            except (RuntimeError, OSError, subprocess.TimeoutExpired) as error:
                failures.append(str(error))
    return times, failures


def edit_rounds(app, recipe, files, rounds, out):
    """Every document edited `rounds` times, all of them at once."""
    for r in range(1, rounds + 1):
        times, failures = at_once(app, [words for path in files for words in recipe.edits(path, r)])
        out["edits"] += times
        out["problems"] += [f"edit round {r}: {failure}" for failure in failures]


def wait_opened(app, count, timeout, since=0):
    """Until `count` tabs are open; a failed open raises."""
    pattern = re.compile(rf"{app.slug}: opened .+ as tab \d+, risk .*, (\d+) tabs open")
    until = time.monotonic() + timeout
    while time.monotonic() < until:
        lines = app.lines()[since:]
        failed = [line for line in lines if " did not open" in line]
        if failed:
            raise RuntimeError(f"{app.slug}: {failed[0][-160:]}")
        if any(int(m.group(1)) >= count for line in lines if (m := pattern.search(line))):
            return (time.monotonic() - app.start) * 1000
        if app.process.poll() is not None:
            raise app.failed(pattern.pattern, timeout)
        free_gb()
        time.sleep(0.05)
    raise app.failed(pattern.pattern, timeout)


# Kernels before patch 0080 abort at 149 characters.
PROFILE_MOST = 140


def data_home(work, name):
    """A data root short enough for the LibreOffice profile."""
    home = os.path.join(work, "h-" + name[-6:])
    profile = os.path.join(home, "ely-table", "lok-profile")
    if len(profile) > PROFILE_MOST:
        raise RuntimeError(f"{profile} is over {PROFILE_MOST} characters; run from a shorter folder")
    return home


def first_place(app):
    """The window's screen rectangle once it is shown."""
    app.wait(r"ui: first frame presented", timeout=30)
    # A pointer resting on the new window feeds it moves.
    run.HOST.park(app.process.pid)
    app.place = run.HOST.rect(app.process.pid)


def interfered(app, since, moves=False):
    """Someone else clicked, or moved the window when `moves`."""
    others = [line[-90:] for line in app.lines()[since:] if OTHERS.search(line)]
    moved = run.HOST.rect(app.process.pid) if moves else app.place
    if others or moved != app.place:
        return f"{app.slug}: someone else clicked or moved the window: {others[:1] or [app.place, moved]}"
    return None


def repainted(app, since):
    """Did the engine take the key, and the screen redraw?"""
    lines = app.lines()[since:]
    took = [line for line in lines if re.search(r"lok: input \d+ text", line)]
    drawn = [line for line in lines if "lok: screen complete" in line]
    later = [run.logged_at(line) for line in drawn if took and run.logged_at(line) >= run.logged_at(took[-1])]
    if not later:
        return ""
    after = later[0] - run.logged_at(took[-1])
    return f"; the engine took it and the screen redrew {after:.0f} ms later: the edit clock missed it"


def keys_answered(app, since, recipe):
    """Keys answered; one that never drew is no feedback."""
    try:
        return feedback.answers(app.lines()[since:], recipe.answer)
    except RuntimeError as error:
        raise NoFeedback(f"{app.slug}: {error}") from None


def type_keys(app, recipe, count, busy=None):
    """Keys to feedback, `count` at least and while `busy` runs."""
    run.HOST.park(app.process.pid)
    time.sleep(0.5)
    since = len(app.lines())
    n = 0
    while n < count or (busy is not None and busy.is_alive()):
        recipe.send(app.process.pid)
        until = time.monotonic() + KEY_WAIT
        while keys_answered(app, since, recipe) <= n:
            if interfered(app, since):
                break
            if time.monotonic() > until:
                raise NoFeedback(f"{app.slug}: key {n + 1} showed nothing in {KEY_WAIT} s{repainted(app, since)}")
            time.sleep(0.01)
        n += 1
        free_gb()
        if why := interfered(app, since, moves=True):
            raise Interfered(why)
        time.sleep(0.15)
    lines = app.lines()[since:]
    return [int(m.group(1)) for line in lines if (m := re.search(recipe.answer, line))], lines


def landed(app, recipe):
    """One key's time, or none when it showed nothing."""
    try:
        return type_keys(app, recipe, 1)[0]
    except NoFeedback:
        return []


def frames(lines):
    """Frames, lost, late, worst dirty-to-shown p95 and max."""
    found = [FRAMES.search(line) for line in lines]
    found = [m for m in found if m]
    if not found:
        return None
    return (sum(int(m.group(1)) for m in found), sum(int(m.group(2)) for m in found), sum(int(m.group(5)) for m in found),
            max(float(m.group(3)) for m in found), max(float(m.group(4)) for m in found))


def zooms(lines):
    """Each zoom the view logged, in percent."""
    return [int(m.group(1)) for line in lines if (m := re.search(r"lok: zoom (\d+)%", line))]


def walk(ups, wheel):
    """Zooms logged stepping up from 25 %, then a wheel."""
    steps = [25, 50, 67, 80, 100, 125, 150, 200, 300, 400]
    seen = steps[1:ups + 1]
    if wheel:
        now = steps.index(seen[-1] if seen else 25)
        seen.append(steps[min(len(steps) - 1, max(0, now + (1 if wheel > 0 else -1)))])
    return seen


def frames_failed(seen):
    """Why frames break PERF-02, or None."""
    if seen is None:
        return "no frame line was logged"
    return f"a frame took {seen[4]:.0f} ms from dirty to shown" if seen[4] > TARGET else None


def stalls(lines):
    return [int(m.group(1)) for line in lines if (m := re.search(feedback.STALL, line))]


def one_app(slug, folder, work, args):
    """Opens, types, edits, saves and reads back one app's documents."""
    recipe, out = RECIPES[slug], {"edits": [], "problems": []}
    FLOOR.lowest = free_now()
    home, place = data_home(work, slug), os.path.join(work, slug)
    os.makedirs(place, exist_ok=True)
    files, front = loadfiles.documents(folder, place, slug, args.docs)
    app = App(slug, home, files, frames=True)
    try:
        first_place(app)
        out["open all ms"] = wait_opened(app, len(files), 60 + 15 * len(files))
        since = len(app.lines())
        handed = time.monotonic()
        drive.sh(run.HOST.exe(slug, True), front, env=app.env, timeout=120)
        wait_opened(app, len(files) + 1, 120, since)
        out["open one more ms"] = (time.monotonic() - handed) * 1000
        time.sleep(2)
        out["memory open MB"] = app.memory_mb()
        app.front()
        recipe.into(app.process.pid)
        # Clicks reach the log at the next two-second flush.
        time.sleep(2.5)
        typing = len(app.lines())
        out["idle"] = type_keys(app, recipe, args.keys)
        busy = threading.Thread(target=edit_rounds, args=(app, recipe, files, args.rounds, out))
        busy.start()
        try:
            out["editing"] = type_keys(app, recipe, args.keys, busy)
        finally:
            busy.join()
        recipe.finish(app.process.pid)
        time.sleep(1)
        out["saves"], failures = at_once(app, [["text" if slug != "ely-table" else "cell", "save", path] for path in files + [front]])
        out["problems"] += [f"save: {failure}" for failure in failures]
        out["memory edited MB"] = app.memory_mb()
        # The log flushes every two seconds.
        time.sleep(2.5)
        if why := interfered(app, typing, moves=True):
            raise Interfered(why)
        free_gb()
        lines = app.lines()
    finally:
        app.stop()
    counts = [int(m.group(1)) for line in lines if (m := WORKER.search(line))]
    if slug != "ely-mark" and (not counts or counts[-1] != len(files) + 1):
        out["problems"].append(f"the worker holds {counts[-1:] or 'no'} documents, not {len(files) + 1}")
    failed = [line for line in lines if re.search(run.PAINT_FAILED, line)]
    out["problems"] += [f"painting: {failed[0][-160:]}"] if failed else []
    keys = len(out["idle"][0]) + len(out["editing"][0])
    if not richfiles.typed(slug, front, keys):
        out["problems"].append(f"front: the {keys} keys typed are not all saved")
    for path in files:
        out["problems"] += richfiles.saved(slug, path, args.rounds)
    free_gb()
    out["stalls"], out["free GB"] = stalls(lines), FLOOR.lowest
    return out


def row(slug, out):
    """One record line; PERF-02's verdicts join the problems."""
    cells = [APPS[slug], f"{out['open all ms']:.0f}", f"{out['open one more ms']:.0f}",
             f"{out['memory open MB']:.0f} / {out['memory edited MB']:.0f}", f"{out['free GB']:.0f}"]
    for phase in ("idle", "editing"):
        ticks, lines = out[phase]
        p95, slow = percentile(ticks, 0.95), [tick for tick in ticks if tick > TARGET]
        if slow:
            out["problems"].append(f"PERF-02: {len(slow)} of {len(ticks)} keys over {TARGET} ms {phase}, max {max(slow)} ms")
        seen = frames(lines)
        if why := frames_failed(seen):
            out["problems"].append(f"PERF-02: {why} {phase}")
        late = "no frame line" if seen is None else f"{seen[2]}/{seen[0]}, p95 ≤{seen[3]:.1f}, max {seen[4]:.1f}"
        cells += [f"{len(ticks)}: {percentile(ticks, 0.5)} / {p95} / {max(ticks)}", late, str(len(stalls(lines)))]
    cells += [f"{percentile(out['edits'], 0.5):.0f} / {percentile(out['edits'], 0.95):.0f}" if out["edits"] else "none",
              f"{percentile(out['saves'], 0.5):.0f} / {max(out['saves']):.0f}" if out["saves"] else "none",
              f"{len(out['stalls'])}, max {max(out['stalls'], default=0)}"]
    if out["stalls"]:
        out["problems"].append(f"PERF-02: main thread busy over 100 ms {len(out['stalls'])} times, max {max(out['stalls'])} ms")
    return "| " + " | ".join(cells) + " |"


def machine():
    return re.sub(r"[^A-Za-z0-9]+", "-", f"{run.HOST.environment()['machine']} {run.HOST.name}").strip("-")


def build(apps):
    """The frames release builds of `apps` and the LOK worker."""
    build = ("cargo", "build", "--release", "-q", "--bins", "--target-dir", drive.target_dir(frames=True))
    # Alone: gpui's manifest would make the worker DPI-aware.
    drive.sh(*build, "-p", search.spec("crates/ely-lok"), cwd=run.ROOT)
    drive.sh(*build, "--features", "ely-ui/frames", *[a for slug in apps for a in ("-p", search.spec("apps/" + slug))],
             cwd=run.ROOT)


def environment(args):
    builds = {name: hashlib.sha256(open(run.HOST.exe(name, True), "rb").read()).hexdigest()[:16]
              for name in [*args.apps, "ely-lok-worker"]}
    env = {"date": datetime.date.today().isoformat(), **run.HOST.environment(), "machine": machine()}
    return {**env, "docs": args.docs, "rounds": args.rounds, "keys": args.keys, "builds": builds,
            "lok": os.environ.get("ELY_LOK_DIR", "engines/")}


def write(out_dir, env, rows, problems):
    os.makedirs(out_dir, exist_ok=True)
    path = os.path.join(out_dir, f"{env['date']}-{env['machine']}-load-{env['docs']}.md")
    lines = [f"# PERF-10 load {env['date']}, {env['docs']} documents per app", "", "Format: docs/perf/README.md.", "",
             "```json", json.dumps(env, indent=2), "```", "",
             "| App | Open all ms | Open one more ms | Memory MB open / edited | Free GB, lowest | Keys idle: P50 / P95 / max ms | "
             "Frames idle: over 8.3 ms / all, dirty to shown | Stalls idle | Keys editing: P50 / P95 / max ms | "
             "Frames editing | Stalls editing | Command edit P50 / P95 ms | Save P50 / max ms | Stalls whole run |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |", *rows, "",
             "Problems:", ""]
    lines += [f"- {problem}" for problem in problems] or ["- none"]
    with open(path, "w", encoding="utf-8", newline="\n") as record:
        record.write("\n".join(lines) + "\n")
    print(path)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--docs", type=int, default=100)
    parser.add_argument("--rounds", type=int, default=3)
    parser.add_argument("--keys", type=int, default=30)
    parser.add_argument("--apps", nargs="*", default=list(RECIPES))
    parser.add_argument("--out", default=os.path.join(run.ROOT, "docs", "perf"))
    args = parser.parse_args()
    if sys.platform != "win32":
        sys.exit("SKIP: PERF-10 reads free memory on Windows only")
    if not 2 <= args.docs or args.rounds > 9:
        parser.error("--docs needs 2 or more; --rounds at most 9")
    work = os.path.join(run.ROOT, ".work", f"L{datetime.datetime.now():%H%M%S}")
    os.makedirs(work)
    run.HOST = drive.host(work)
    watch_memory()
    build(args.apps)
    folder = loadfiles.made(os.path.join(run.ROOT, ".work", "load-samples"))
    rows, problems = [], []
    for slug in args.apps:
        for attempt in range(RETRIES + 1):
            try:
                out = one_app(slug, folder, os.path.join(work, str(attempt)), args)
                rows.append(row(slug, out))
                problems += [f"{APPS[slug]}: {problem}" for problem in out["problems"]]
            except Interfered as error:
                print(f"{error}; {'again in 30 s' if attempt < RETRIES else 'giving up'}", flush=True)
                if attempt < RETRIES:
                    time.sleep(30)
                    continue
                problems.append(f"{APPS[slug]}: run voided {RETRIES + 1} times: {error}")
            except (RuntimeError, OSError, subprocess.TimeoutExpired) as error:
                problems.append(f"{APPS[slug]}: run stopped: {error}")
            break
        print(f"{slug}: done, {len(problems)} problems so far", flush=True)
    write(args.out, environment(args), rows, problems)
    if problems:
        print(f"the logs: {work}")
        raise SystemExit("\n".join(problems))
    shutil.rmtree(work)


if __name__ == "__main__":
    main()
