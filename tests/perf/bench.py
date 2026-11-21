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

import counters
import cycles
import drive
import run
import samples
import search
import sheets
import feedback
import judge
from report import write

from run import APPS, App

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
        try:
            self.until(lambda taken: taken)
        except BaseException as error:
            # Started but silent: stopped here; its trace stays.
            counters.stopped(error, self.stop)
            raise

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
        # Before any build: a bad name wastes a run.
        known(kept, args.redo)
    cases = Cases(kept["cases"] if kept else {}, args.redo)
    want = lambda perf: not args.only or perf in args.only
    partial = bool(args.only) or set(args.apps) != set(APPS)
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
    sampler, trace = (Paging, "vmstat.log") if linux else (counters.Counted, "paging.log")
    # What the run reached; a failure picks up from it.
    held = {"judged": None, "keep": None, "traced": False, "raw": False, "stranded": False,
            "trace": (os.path.join(work, trace), sampler.suffix)}
    paging, env, up = None, None, True
    try:
        # A sampler that fails to start stops itself first.
        paging = sampler(held["trace"][0])
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
        # To the last write: a failure keeps the evidence.
        stopped_engines(work)
        up = False
        # Kept, but left out by --only or --apps.
        if set(args.redo) - cases.ran:
            raise RuntimeError(f"no such case to redo here: {sorted(set(args.redo) - cases.ran)}")
        judged = held["judged"] = paging.judged(cases.windows)
        env = environment(found, partial)
        if kept:
            env = redone(kept["env"], env, args.redo)
        for key, counts, note in judged:
            print(f"bench: paging {'ok' if counts else 'VOID'} {key}: {note}")
        env["void"] = void_after(kept["env"].get("void", []) if kept else [], cases.ran, judged)
        write(args.out, env, *table(cases), args.runs)
        # A void case's evidence stays, --keep or not.
        if args.keep or env["void"]:
            keep = held["keep"] = kept_at(args)
            kept_trace(keep, held["trace"])
            held["traced"] = True
            written(keep, {"env": env, "cases": cases.done})
    except BaseException as failure:
        engines = (lambda: stopped_engines(work)) if up else (lambda: None)
        failed(args, kept, cases, paging, held, lambda: environment(found, partial), failure, engines, work)
        raise failure
    finally:
        # Nothing else holds the evidence then.
        if not held["stranded"]:
            shutil.rmtree(work, ignore_errors=True)
    if env["void"]:
        sys.exit(f"bench: timed while the machine paged, redo: {env['void']}; samples and raw results: {keep}")


def table(cases):
    """The record's rows and PERF-08 counts, in case order."""
    rows = [row for case in cases.done.values() for row in case.get("rows", [])]
    held = [one for case in cases.done.values() for one in case.get("held", [])]
    return rows, held


def known(kept, redo):
    """Refuses names the kept run has no case for."""
    unknown = sorted(set(redo) - set(kept["cases"]))
    if unknown:
        raise RuntimeError(f"no such case to redo: {unknown}; --keep has {sorted(kept['cases'])}")


def kept_at(args):
    """Raw results' path: --keep, else a folder beside --out."""
    if args.keep:
        return args.keep
    folder = os.path.join(os.path.dirname(os.path.abspath(args.out)), "bench-kept")
    os.makedirs(folder, exist_ok=True)
    return os.path.join(folder, f"{utc():%Y%m%dT%H%M%S}.json")


def stopped_engines(work):
    left = run.stop_engines(work)
    if left:
        raise RuntimeError(f"engines outlived the run: {left}")


def failed(args, kept, cases, paging, held, now, failure, engines, work):
    """From sampling's start, a failure keeps samples and raw results."""

    def choose():
        held["keep"] = held["keep"] or kept_at(args)

    def judge():
        if held["judged"] is None and paging:
            held["judged"] = paging.judged(cases.windows)

    def stop():
        if held["judged"] is None and paging:
            paging.stop()

    def logs():
        copied = tempfile.mkdtemp(prefix="ely-bench-failed-")
        for log in glob.glob(os.path.join(work, "**", "*.log"), recursive=True):
            to = os.path.join(copied, os.path.relpath(log, work))
            os.makedirs(os.path.dirname(to), exist_ok=True)
            shutil.copy(log, to)
        print(f"the failed run's logs: {copied}")

    def trace():
        if held["keep"] and not held["traced"]:
            kept_trace(held["keep"], held["trace"])
            held["traced"] = True

    def raw():
        # A redo always keeps the kept record's void.
        if kept:
            failed_redo(args, kept, cases, held, now)
        elif held["keep"]:
            kept_failed(held["keep"], cases, now)
            held["raw"] = True

    steps = [("no folder could hold the evidence", choose), ("the engines did not stop", engines),
             ("the samples were not judged", judge), ("the sampler did not stop", stop),
             ("the logs were not copied", logs), ("the samples were not kept", trace),
             ("the raw results were not kept", raw)]
    for what, step in steps:
        # Shown, and noted on the run's own error.
        try:
            step()
        except (Exception, SystemExit) as error:
            said = f"bench: {what}: {type(error).__name__}: {error}"
            print(said)
            failure.add_note(said)
    if held["traced"] and held["raw"]:
        print(f"bench: the failed run's samples and raw results: {held['keep']}")
        return
    held["stranded"] = True
    lost = [what for what, done in (("samples", held["traced"]), ("raw results", held["raw"])) if not done]
    where = "nothing could be kept" if held["keep"] is None else f"{' and '.join(lost)} not kept at {held['keep']}"
    said = f"bench: {where}; the work folder stays: {work}"
    print(said)
    failure.add_note(said)


def kept_failed(keep, cases, now):
    """A failed fresh run's finished cases, none of them judged."""
    try:
        env = now()
    except (Exception, SystemExit) as error:
        # An early failure: no app has logged its GPU.
        env = {"unread": f"the machine was not read: {type(error).__name__}: {error}"}
    written(keep, {"env": {**env, "void": sorted(cases.ran)}, "cases": cases.done})


def written(path, raw):
    """Raw results, whole or not at all."""
    folder = os.path.dirname(os.path.abspath(path))
    handle, part = tempfile.mkstemp(dir=folder, prefix=os.path.basename(path) + ".", suffix=".part")
    with os.fdopen(handle, "w", encoding="utf-8") as out:
        json.dump(raw, out)
    os.replace(part, path)


def kept_trace(keep, trace):
    """The paging samples beside --keep, as the record rules ask."""
    path, suffix = trace
    shutil.copy(path, f"{keep}.{utc():%Y%m%dT%H%M%S}.{suffix}")


def failed_redo(args, kept, cases, held, now):
    """Merges a failed redo's judged cases; else --keep stays."""
    if held["judged"] is None or not cases.ran:
        held["raw"] = True
        print(f"bench: the failed redo left {args.keep} as it was; void: {kept['env'].get('void', [])}")
        return
    env = {**redone(kept["env"], now(), sorted(cases.ran)),
           "void": void_after(kept["env"].get("void", []), cases.ran, held["judged"])}
    written(args.keep, {"env": env, "cases": cases.done})
    held["raw"] = True
    write(args.out, env, *table(cases), args.runs)
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


if __name__ == "__main__":
    main()
