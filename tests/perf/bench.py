#!/usr/bin/env python3
"""PERF-01 to PERF-08 on release builds, written as a record."""

import argparse
import datetime
import glob
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

import baseline
import counters
import cycles
import drive
import run
import samples
import search
import sheets
import feedback
import judge

from run import APPS, App, percentile

TARGETS = {"PERF-01": 3000, "PERF-02": 100, "PERF-03": 5000, "PERF-04": 5000,
           "PERF-05": 3000, "PERF-06": 1000, "PERF-07": 2000}
# Total memory budgets, MB, at the opening mark (H-02).
BUDGETS = {"PERF-07": 400}


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
            failed = [line for line in app.lines() if re.search(run.PAINT_FAILED, line)]
            if failed:
                raise RuntimeError(f"{slug}: {sample} did not paint: {failed[0]}")
            times.append(took)
            memory.append(app.resident_mb())
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


def started(work, slug, runs):
    home = os.path.join(work, "home-" + slug)
    first = App(slug, home, smoke="1").ready()
    daily = [App(slug, home, smoke="1").ready() for _ in range(runs)]
    return [("PERF-01", f"{slug} first run", [first], None), ("PERF-01", f"{slug} daily start", daily, None)]


def blank(work, slug, runs):
    """The window opens first; the blank document follows the engine."""
    times = []
    for _ in range(runs):
        app = App(slug, os.path.join(work, "home-" + slug))
        try:
            times.append(app.wait("opened Untitled"))
        finally:
            app.stop()
    return [("PERF-01", f"{slug} blank document ready", times, None)]


def warmed(work, slug):
    """A redone blank case starts after a first run."""
    home = os.path.join(work, "home-" + slug)
    if not os.path.exists(home):
        App(slug, home, smoke="1").ready()


def pinned(recipe):
    """Seeds before the window; the case reuses it."""
    if recipe.home:
        home = recipe.home()
        recipe.home = lambda: home


def utc():
    return datetime.datetime.now(datetime.timezone.utc)


def stamp(moment):
    return moment.isoformat(timespec="milliseconds")


class Paging:
    """Linux: vmstat each second, stamped in UTC, for judge.py."""

    suffix = "vmstat"

    def __init__(self, path):
        self.path = path
        with open(path, "w", encoding="utf-8") as trace:
            self.process = subprocess.Popen(["vmstat", "-n", "-t", str(int(judge.INTERVAL.total_seconds()))], stdout=trace, env=dict(os.environ, TZ="UTC"))
        self.until(lambda taken: taken)

    def taken(self):
        with open(self.path, encoding="utf-8") as trace:
            return judge.samples(trace.read().splitlines())

    def until(self, enough):
        deadline = time.monotonic() + 10
        while True:
            try:
                if enough(self.taken()):
                    return
            except ValueError:
                pass
            if self.process.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError(f"vmstat gave no samples in 10 s (exit {self.process.poll()})")
            # Polls the trace for its next sample.
            time.sleep(0.2)

    def judged(self, windows):
        """Each window's verdict, once a sample covers the last."""
        if windows:
            self.until(lambda taken: taken and taken[-1][0] >= windows[-1][2])
        self.stop()
        return judge.verdicts(windows, self.taken()) if windows else []

    def stop(self):
        self.process.terminate()
        self.process.wait()


def void_after(before, ran, judged):
    """Still void: earlier cases not redone, and newly judged ones."""
    return sorted(set(before) - set(ran) | {key for key, counts, _ in judged if not counts})


class Cases:
    """Each case's results by key; prints each window."""

    def __init__(self, done, redo):
        self.done, self.redo, self.ran, self.windows = done, set(redo), set(), []

    def wanted(self, key):
        return not self.redo or key in self.redo

    def run(self, key, produce, setup=None, held=False):
        if not self.wanted(key):
            return
        # setup runs outside the window; produce gets its result.
        ready = setup() if setup else None
        began = utc()
        made = produce(ready)
        ended = utc()
        # Swap is judged per case, in this window (R20.28).
        print(f"bench: case {key} from {stamp(began)} to {stamp(ended)}", flush=True)
        self.windows.append((key, began, ended))
        self.done[key] = {"held": made} if held else {"rows": made}
        self.ran.add(key)


def machine():
    """Today and this machine, as each record names it."""
    env = {"date": datetime.date.today().isoformat(), **run.HOST.environment()}
    # One PC runs both systems; each keeps its own record.
    if run.HOST.name != "macOS":
        env["machine"] = re.sub(r"[^A-Za-z0-9]+", "-", f"{env['machine']} {run.HOST.name}").strip("-")
    return env


def built():
    return {slug: hashlib.sha256(open(run.HOST.exe(slug), "rb").read()).hexdigest()[:16] for slug in APPS}


def matched(kept_env, builds, found):
    """A redo times only the kept run's builds and samples."""
    now = (builds, {name: value[1] for name, value in found.items()})
    if now != (kept_env["builds"], kept_env["samples"]):
        raise RuntimeError("--redo needs the same builds and samples as the kept run")


def environment(found, partial):
    builds = built()
    with open(os.path.join(run.ROOT, "Cargo.toml"), encoding="utf-8") as manifest:
        version = re.search(r'^version = "(.+)"', manifest.read(), re.M).group(1)
    env = {**machine(), "version": version}
    # A run of some cases is no release's record.
    if partial:
        env["partial"] = True
    return {**env, "builds": builds, "samples": {name: value[1] for name, value in found.items()},
            "mails": search.MAILS}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=30)
    parser.add_argument("--only", nargs="*")
    parser.add_argument("--apps", nargs="*", default=list(APPS))
    parser.add_argument("--out", default=os.path.join(run.ROOT, "docs", "perf"))
    parser.add_argument("--keep", help="raw results: written after a run, read by --redo")
    parser.add_argument("--redo", nargs="+", default=[], metavar="CASE", help="rerun these cases into --keep's record")
    args = parser.parse_args()
    if args.redo and not args.keep:
        parser.error("--redo needs --keep")
    kept = None
    if args.redo:
        with open(args.keep, encoding="utf-8") as raw:
            kept = json.load(raw)
    cases = Cases(kept["cases"] if kept else {}, args.redo)
    want = lambda perf: not args.only or perf in args.only
    work = tempfile.mkdtemp()
    run.HOST = drive.host(work)
    packages = [arg for slug in APPS for arg in ("-p", search.spec("apps/" + slug))]
    build = ("cargo", "build", "--release", "-q", "--bins")
    # Alone: gpui's manifest would make the worker DPI-aware.
    for worker in ("crates/ely-lok", "crates/ely-pdf-worker"):
        drive.sh(*build, "-p", search.spec(worker), cwd=run.ROOT)
    # Frames first, set aside; PERF-02 reads gpui's journal.
    drive.sh(*build, "--features", "ely-ui/frames", *packages, cwd=run.ROOT)
    drive.frames_apart([*APPS, "ely-lok-worker", "ely-pdf-worker"])
    drive.sh(*build, *packages, cwd=run.ROOT)
    found = samples.made(os.path.join(run.ROOT, "engines", "bench"), run.HOST.git())
    if kept:
        matched(kept["env"], built(), found)
    # Each system's own counters, sampled here (R20.58).
    linux = run.HOST.name == "Linux"
    paging = Paging(os.path.join(work, "vmstat.log")) if linux else counters.Counted(os.path.join(work, "paging.log"))
    try:
        if want("PERF-01"):
            for slug in APPS:
                cases.run(f"PERF-01 {slug}", lambda _, slug=slug: started(work, slug, args.runs))
            for slug in ["ely-doc", "ely-table", "ely-show"]:
                cases.run(f"PERF-01 {slug} blank", lambda _, slug=slug: blank(work, slug, args.runs),
                          setup=lambda slug=slug: warmed(work, slug))
        if want("PERF-02"):
            for recipe in feedback.keys(work, found):
                if recipe.slug in args.apps:
                    cases.run(f"PERF-02 {recipe.slug}", lambda _, recipe=recipe: typed(work, recipe, args.runs),
                              setup=lambda recipe=recipe: pinned(recipe))
        opening = [("PERF-03", "ely-doc", "doc.docx", ["opened doc.docx", r"lok: first tile drawn"]),
                 ("PERF-04", "ely-table", "sheet.xlsx", ["opened sheet.xlsx", r"lok: first tile drawn"]),
                 *[("PERF-04", "ely-table", name, [f"opened {name}", r"lok: first tile drawn"]) for name in sheets.MAKERS],
                 ("PERF-05", "ely-pdf", "pages.pdf", ["opened pages.pdf", r"pdf: first tile drawn, page 0$"]),
                 ("PERF-07", "ely-mark", "notes.md", ["opened notes.md", r"mark: shown as \"visual\""])]
        if want("PERF-03") and cases.wanted("PERF-03 doc.docx"):
            laid_out(work, found["doc.docx"][0], 100)
        for perf, slug, name, marks in opening:
            if want(perf):
                cases.run(f"{perf} {name}", lambda _, perf=perf, slug=slug, name=name, marks=marks:
                          opens(work, perf, slug, found[name][0], name, marks, args.runs))
        if want("PERF-06"):
            cases.run("PERF-06 ely-tree", lambda _: searched_tree(work, found["repo"][0], args.runs))
            cases.run("PERF-06 ely-mail", lambda home: searched_mail(home, args.runs), setup=lambda: search.seeded(work))
        if want("PERF-08"):
            for recipe in cycles.recipes(work):
                if recipe.slug in args.apps:
                    cases.run(f"PERF-08 {recipe.slug}", lambda _, recipe=recipe: [(recipe.slug, *cycles.cycles(work, recipe))],
                              setup=lambda recipe=recipe: pinned(recipe), held=True)
        judged = paging.judged(cases.windows)
        if args.keep:
            shutil.copy(paging.path, f"{args.keep}.{utc():%Y%m%dT%H%M%S}.{paging.suffix}")
    except BaseException:
        logs = tempfile.mkdtemp(prefix="ely-bench-failed-")
        for log in glob.glob(os.path.join(work, "**", "*.log"), recursive=True):
            to = os.path.join(logs, os.path.relpath(log, work))
            os.makedirs(os.path.dirname(to), exist_ok=True)
            shutil.copy(log, to)
        print(f"the failed run's logs: {logs}")
        if kept:
            failed_redo(args, kept, cases, paging)
        else:
            paging.stop()
        raise
    finally:
        left = run.stop_engines(work)
        shutil.rmtree(work, ignore_errors=True)
    if left:
        raise RuntimeError(f"engines outlived the run: {left}")
    if set(args.redo) - cases.ran:
        raise RuntimeError(f"no such case to redo: {sorted(set(args.redo) - cases.ran)}")
    env = environment(found, bool(args.only) or set(args.apps) != set(APPS))
    if kept:
        env = redone(kept["env"], env, args.redo)
    for key, counts, note in judged:
        print(f"bench: paging {'ok' if counts else 'VOID'} {key}: {note}")
    env["void"] = void_after(kept["env"].get("void", []) if kept else [], cases.ran, judged)
    write(args.out, env, *table(cases), args.runs)
    if args.keep:
        with open(args.keep, "w", encoding="utf-8") as raw:
            json.dump({"env": env, "cases": cases.done}, raw)
    if env.get("void"):
        sys.exit(f"bench: timed while the machine paged, redo: {env['void']}")


def table(cases):
    """The record's rows and PERF-08 counts, in case order."""
    rows = [row for case in cases.done.values() for row in case.get("rows", [])]
    held = [one for case in cases.done.values() for one in case.get("held", [])]
    return rows, held


def failed_redo(args, kept, cases, paging):
    """Keeps a failed redo's finished cases; the rest stay void."""
    judged = paging.judged(cases.windows)
    env = {**redone(kept["env"], kept["env"], sorted(cases.ran)),
           "void": void_after(kept["env"].get("void", []), cases.ran, judged)}
    write(args.out, env, *table(cases), args.runs)
    with open(args.keep, "w", encoding="utf-8") as raw:
        json.dump({"env": env, "cases": cases.done}, raw)
    print(f"bench: the failed redo kept {sorted(cases.ran)}; void: {env['void']}")


def typed(work, recipe, runs):
    ticks, stalls, starved = feedback.typing(work, recipe, runs)
    what = "tile" if recipe.answer == feedback.TILE else "frame"
    name = APPS[recipe.slug]
    # Starved: the machine, not the app; shown, not judged.
    return [("PERF-02", f"{name} key to {what}", ticks, None),
            ("PERF-02", f"{name} main thread busy >100 ms: {len(stalls)} times", stalls or [0], None),
            ("PERF-02", f"{name} timers late >100 ms while starved: {len(starved)} times", starved or [0], None)]


def opens(work, perf, slug, sample, name, marks, runs):
    times, memory = opened(slug, os.path.join(work, "home-" + slug), sample, marks, runs)
    return [(perf, f"{APPS[slug]} {name}", times, max(memory))]


def searched_tree(work, repo, runs):
    history, searched = search.tree(work, repo, runs)
    return [("PERF-06", "ElyTree history, press to first screen", history, None),
            ("PERF-06", "ElyTree query 'part 42', Enter to first results", searched, None)]


def searched_mail(home, runs):
    mailed = search.mail(home, runs)
    return [("PERF-06", f"ElyMail query 'report' in {search.MAILS} messages, last key to first results", mailed, None)]


def redone(before, now, redo):
    """The first run's record, its redone cases named."""
    same = lambda env: {key: value for key, value in env.items() if key not in ("date", "rerun", "void")}
    if same(before) != same(now):
        raise RuntimeError("--redo needs the same builds, samples and machine as the kept run")
    return {**before, "rerun": {**before.get("rerun", {}), **{key: stamp(utc()) for key in redo}}}


def write(out, env, rows, held, runs):
    os.makedirs(out, exist_ok=True)
    kind = f"-{env['kind']}" if env.get("kind") else ""
    path = os.path.join(out, f"{env['date']}-{env['machine']}{kind}{'-partial' if env.get('partial') else ''}.md")
    lines = [f"# Benchmark {env['date']}", "", "Format: docs/perf/README.md.", "", "```json",
             json.dumps(env, indent=2), "```", "",
             "| PERF | Case | Runs | P50 ms | P95 ms | Target ms | Resident MB | Budget MB |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for perf, case, times, memory in rows:
        target = TARGETS.get(perf)
        p95 = percentile(times, 0.95)
        # Counts, not times: stalls and starved timers.
        counted = "busy" in case or "starved" in case
        verdict = "" if target is None or counted else (" ✓" if p95 <= target else " ✗")
        budget = BUDGETS.get(perf) if memory is not None else None
        within = "" if budget is None else (" ✓" if memory <= budget else " ✗")
        mem = "" if memory is None else f"{memory:.0f}{within}"
        lines.append(f"| {perf} | {case} | {len(times)} | {percentile(times, 0.5):.0f} | {p95:.0f}{verdict} | "
                     f"{target or ''} | {mem} | {budget or ''} |")
    for slug, (memory, handles, live), seen in held:
        lines += ["", f"PERF-08, {APPS[slug]} opening and closing a document {len(seen)} times "
                  "(private memory MB, swapped included / handles / live documents):", "",
                  f"before: {memory:.0f}/{handles}/{','.join(map(str, live))}", "",
                  " ".join(f"{memory:.0f}/{handles}/{','.join(map(str, live))}" for memory, handles, live in seen)]
    lines += ["", "PERF-09 (Agent sessions) belongs to the second release.", ""]
    with open(path, "w", encoding="utf-8", newline="\n") as record:
        record.write("\n".join(lines))
    # Each wants its reason before a release (baseline.py).
    record = baseline.read(path)
    last = None if env.get("partial") else baseline.before(baseline.records(out), record)
    found = baseline.slower(last, record) if last else []
    if found:
        with open(path, "a", encoding="utf-8", newline="\n") as record:
            record.write("\n".join([baseline.HEAD, "", *baseline.lines(found), ""]))
    print(path)
    return path


if __name__ == "__main__":
    main()
