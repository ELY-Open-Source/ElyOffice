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
import tempfile
import time

import cycles
import drive
import run
import samples
import search
import feedback

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
        rows.append(("PERF-01", f"{slug} first run", [first], None))
        rows.append(("PERF-01", f"{slug} daily start", daily, None))
    # The window opens first; the blank document follows the engine.
    for slug in ["ely-doc", "ely-table", "ely-show"]:
        blank = []
        for _ in range(runs):
            app = App(slug, os.path.join(work, "home-" + slug))
            try:
                blank.append(app.wait("opened Untitled"))
            finally:
                app.stop()
        rows.append(("PERF-01", f"{slug} blank document ready", blank, None))
    return rows


def environment(found):
    builds = {slug: hashlib.sha256(open(run.HOST.exe(slug), "rb").read()).hexdigest()[:16] for slug in APPS}
    env = {"date": datetime.date.today().isoformat(), **run.HOST.environment()}
    # One PC runs both systems; each keeps its own record.
    if run.HOST.name != "macOS":
        env["machine"] = re.sub(r"[^A-Za-z0-9]+", "-", f"{env['machine']} {run.HOST.name}").strip("-")
    return {**env, "builds": builds, "samples": {name: value[1] for name, value in found.items()},
            "mails": search.MAILS}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=30)
    parser.add_argument("--only", nargs="*")
    parser.add_argument("--apps", nargs="*", default=list(APPS))
    parser.add_argument("--out", default=os.path.join(run.ROOT, "docs", "perf"))
    args = parser.parse_args()
    want = lambda perf: not args.only or perf in args.only
    work = tempfile.mkdtemp()
    run.HOST = drive.host(work)
    packages = ["-p", search.spec("crates/ely-lok"), "-p", search.spec("crates/ely-pdf-worker"),
                *[arg for slug in APPS for arg in ("-p", search.spec("apps/" + slug))]]
    drive.sh("cargo", "build", "--release", "-q", "--bins", *packages, cwd=run.ROOT)
    # gpui's journal times each key to its frame (PERF-02).
    drive.sh("cargo", "build", "--release", "-q", "--bins", "--features", "ely-ui/frames",
             "--target-dir", drive.target_dir(frames=True), *packages, cwd=run.ROOT)
    found = samples.made(os.path.join(run.ROOT, "engines", "bench"), run.HOST.git())
    rows, held = [], []
    try:
        if want("PERF-01"):
            rows += perf01(work, args.runs)
        if want("PERF-02"):
            for recipe in feedback.keys(work, found):
                if recipe.slug in args.apps:
                    ticks, stalls = feedback.typing(work, recipe, args.runs)
                    what = "tile" if recipe.answer == feedback.TILE else "frame"
                    rows.append(("PERF-02", f"{APPS[recipe.slug]} key to {what}", ticks, None))
                    rows.append(("PERF-02", f"{APPS[recipe.slug]} main thread busy >100 ms: {len(stalls)} times",
                                 stalls or [0], None))
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
            history, searched = search.tree(work, found["repo"][0], args.runs)
            rows.append(("PERF-06", "ElyTree history, press to first screen", history, None))
            rows.append(("PERF-06", "ElyTree query 'part 42', Enter to first results", searched, None))
            mailed = search.mail(search.seeded(work), args.runs)
            rows.append(("PERF-06", f"ElyMail query 'report' in {search.MAILS} messages, last key to first results",
                         mailed, None))
        if want("PERF-08"):
            for recipe in cycles.recipes(work):
                if recipe.slug in args.apps:
                    held.append((recipe.slug, *cycles.cycles(work, recipe)))
    except BaseException:
        kept = tempfile.mkdtemp(prefix="ely-bench-failed-")
        for log in glob.glob(os.path.join(work, "**", "*.log"), recursive=True):
            to = os.path.join(kept, os.path.relpath(log, work))
            os.makedirs(os.path.dirname(to), exist_ok=True)
            shutil.copy(log, to)
        print(f"the failed run's logs: {kept}")
        raise
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
    for slug, (memory, handles, live), seen in held:
        lines += ["", f"PERF-08, {APPS[slug]} opening and closing a document {len(seen)} times "
                  "(total memory MB / handles / live documents):", "",
                  f"before: {memory:.0f}/{handles}/{','.join(map(str, live))}", "",
                  " ".join(f"{memory:.0f}/{handles}/{','.join(map(str, live))}" for memory, handles, live in seen)]
    lines += ["", "PERF-09 (Agent sessions) belongs to the second release.", ""]
    with open(path, "w", encoding="utf-8", newline="\n") as record:
        record.write("\n".join(lines))
    print(path)


if __name__ == "__main__":
    main()
