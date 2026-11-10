#!/usr/bin/env python3
"""PERF-10 for ElyPDF: many large PDFs open, each paged through."""

import argparse
import datetime
import os
import re
import shutil
import sys
import time

import drive
import feedback
import load
import run
import samples
import search

WORKER = re.compile(r"pdf-worker: opened document \d+, locked \w+, (\d+) documents open")


def build():
    """The frames build of ElyPDF and its worker."""
    cargo = ("cargo", "build", "--release", "-q", "--bins", "--target-dir", drive.target_dir(frames=True))
    drive.sh(*cargo, "-p", search.spec("crates/ely-pdf-worker"), cwd=run.ROOT)
    drive.sh(*cargo, "--features", "ely-ui/frames", "-p", search.spec("apps/ely-pdf"), cwd=run.ROOT)


def documents(place, count):
    """`count` names for PERF-05's 1000-page sample."""
    sample = os.path.join(run.ROOT, ".work", "load-samples", "pages.pdf")
    if not os.path.exists(sample):
        os.makedirs(os.path.dirname(sample), exist_ok=True)
        samples.pdf(sample + ".part")
        os.replace(sample + ".part", sample)
    files = [os.path.join(place, f"pdf-{i:03d}.pdf") for i in range(count)]
    for path in files:
        # A hard link is the same file: one tab.
        shutil.copyfile(sample, path)
    return files


def one_run(work, args):
    """Opens every PDF, pages the front one, then the rest."""
    out, place = {"problems": []}, os.path.join(work, "ely-pdf")
    os.makedirs(place)
    files = documents(place, args.docs)
    recipe = load.Recipe("ely-pdf", feedback.FRAME, feedback.into_document, load.finish_nothing, lambda path, r: [], key="pagedown")
    app = run.App("ely-pdf", load.data_home(work, "ely-pdf"), files, frames=True)
    try:
        load.first_place(app)
        out["open all ms"] = load.wait_opened(app, len(files), 60 + 15 * len(files), last=files[-1])
        app.front()
        time.sleep(2.5)
        out["front"] = load.type_keys(app, recipe, args.keys)
        out["each"] = []
        # A tab behind loads when first shown (R15.57).
        for path in files[:-1]:
            run.HOST.press(app.process.pid, os.path.basename(path))
            # The click reaches the log at the next flush.
            time.sleep(2.5)
            took = load.landed(app, recipe)
            out["each"] += took
            if not took:
                out["problems"].append(f"{os.path.basename(path)}: a key showed nothing in {load.KEY_WAIT} s")
        out["memory MB"] = app.memory_mb()
        time.sleep(2.5)
        lines = app.lines()
    finally:
        app.stop()
    held = [int(m.group(1)) for line in lines if (m := WORKER.search(line))]
    if not held or max(held) != len(files):
        out["problems"].append(f"the worker held {max(held, default=0)} documents, not {len(files)}")
    out["stalls"], out["free GB"] = load.stalls(lines), load.free_gb()
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--docs", type=int, default=20)
    parser.add_argument("--keys", type=int, default=30)
    parser.add_argument("--out", default=os.path.join(run.ROOT, "docs", "perf"))
    args = parser.parse_args()
    if sys.platform not in ("win32", "linux"):
        sys.exit("SKIP: PERF-10 reads free memory on Windows and Linux only")
    work = os.path.join(run.ROOT, ".work", f"P{datetime.datetime.now():%H%M%S}")
    os.makedirs(work)
    run.HOST = drive.host(work)
    load.watch_memory()
    build()
    out = one_run(work, args)
    for name in ("front", "each"):
        ticks = out[name][0] if name == "front" else out[name]
        slow = [tick for tick in ticks if tick > load.TARGET]
        out["problems"] += [f"PERF-02: {len(slow)} of {len(ticks)} keys over {load.TARGET} ms ({name}), max {max(slow)} ms"] if slow else []
    if out["stalls"]:
        out["problems"].append(f"PERF-02: main thread busy over 100 ms {len(out['stalls'])} times, max {max(out['stalls'])} ms")
    front, each, env = out["front"][0], out["each"], run.HOST.environment()
    cells = [f"{out['open all ms']:.0f}", f"{out['memory MB']:.0f}", f"{out['free GB']:.0f}",
             f"{len(front)}: {run.percentile(front, 0.5)} / {run.percentile(front, 0.95)} / {max(front)}",
             f"{len(each)}: {run.percentile(each, 0.5)} / {run.percentile(each, 0.95)} / {max(each)}" if each else "none",
             f"{len(out['stalls'])}, max {max(out['stalls'], default=0)}"]
    lines = [f"# PERF-10 ElyPDF load {datetime.date.today().isoformat()}, {args.docs} PDFs of 1000 pages", "",
             f"{env['machine']}, {env['system']}.", "",
             "| Open all ms | Memory MB | Free GB, lowest | Keys in front: P50 / P95 / max ms | First key in each other tab | Stalls |",
             "| --- | --- | --- | --- | --- | --- |", "| " + " | ".join(cells) + " |", "", "Problems:", ""]
    lines += [f"- {problem}" for problem in out["problems"]] or ["- none"]
    os.makedirs(args.out, exist_ok=True)
    path = os.path.join(args.out, f"{datetime.date.today().isoformat()}-{load.machine()}-pdf-load-{args.docs}.md")
    with open(path, "w", encoding="utf-8", newline="\n") as record:
        record.write("\n".join(lines) + "\n")
    print(path)
    sys.exit(1 if out["problems"] else 0)


if __name__ == "__main__":
    main()
