#!/usr/bin/env python3
"""PERF-10's extreme cases on real windows, each one checked."""

import argparse
import datetime
import json
import os
import re
import shutil
import subprocess
import sys
import threading
import time

import drive
import load
import loadfiles
import richfiles
import run

from load import RECIPES
from loadfiles import TOKEN
from run import APPS, App

ENDED = r"the engine ended"
ZOOMS = [("100", "0", 0), ("25", "minus", 4), ("67", "minus", 2), ("125", "plus", 1), ("400", "plus", 5), ("wheel+2", "0", 2), ("wheel-3", "0", -3)]
CASES = {}
WORK = ""


def case(fn):
    CASES[fn.__name__.replace("_", "-")] = fn
    return fn


def place(name, slug):
    folder = os.path.join(WORK, name, slug)
    os.makedirs(folder, exist_ok=True)
    return folder


def start(name, slug, files, front=None):
    """`slug` with `files` open; `front` handed over last, active."""
    app = App(slug, load.data_home(WORK, f"{name[:3]}{slug[4:7]}"), files, frames=True)
    try:
        load.wait_opened(app, len(files), 60 + 15 * len(files))
        if front:
            since = len(app.lines())
            drive.sh(run.HOST.exe(slug, True), front, env=app.env, timeout=120)
            load.wait_opened(app, len(files) + 1, 120, since)
    except BaseException:
        app.stop()
        raise
    return app


def said(app, *words, timeout=300):
    out = subprocess.run([run.HOST.exe(app.slug, True), *words], env=app.env, capture_output=True, text=True,
                         timeout=timeout, creationflags=drive.QUIET)
    if out.returncode:
        raise RuntimeError(f"{' '.join(words[:2])}: exit {out.returncode}: {out.stderr.strip()[:200]}")
    return out.stdout


def verb(slug):
    return "cell" if slug == "ely-table" else "text"


def typing_into(app):
    """The app in front, the caret where keys land."""
    if app.slug != "ely-mark":
        app.wait(r"lok: first tile drawn", timeout=120)
        time.sleep(2)
    app.front()
    RECIPES[app.slug].into(app.process.pid)
    time.sleep(2.5)


def typed_now(app, path, keys):
    """What the open document holds of `keys` typed, read live."""
    if app.slug == "ely-table":
        value = json.loads(said(app, "cell", "get", path, "Typing!A1"))["text"]
        return len(value) if set(value) <= {"1"} else -1
    found = re.findall(r"x+", said(app, "text", "get", path))
    return max((len(run_) for run_ in found), default=0)


def lines_matching(app, pattern, since=0):
    return [line for line in app.lines()[since:] if re.search(pattern, line)]


@case
def missing_sources():
    """Linked files whose sources are gone open, type and save."""
    rows = []
    for slug in ("ely-doc", "ely-table"):
        folder = place("missing", slug)
        if slug == "ely-doc":
            path = os.path.join(folder, "doc-linked.docx")
            loadfiles.linked_docx(path, folder)
        else:
            path = os.path.join(folder, "table-big.xlsx")
            loadfiles.workbook(path, 2000, linked=True, typing=True)
        began = time.monotonic()
        app = start("missing", slug, [path])
        took = (time.monotonic() - began) * 1000
        try:
            typing_into(app)
            ticks, _ = load.type_keys(app, RECIPES[slug], 5)
            RECIPES[slug].finish(app.process.pid)
            said(app, verb(slug), "save", path)
            warned = lines_matching(app, r"(WARN|ERROR).*(link|source|not found|missing)")
        finally:
            app.stop()
        kept = loadfiles.word_links(path) if slug == "ely-doc" else loadfiles.book_links(path)
        lost = [what for what, ok in kept.items() if not ok and "shows" not in what]
        rows.append((f"{APPS[slug]}: opens a file whose link sources are missing", not lost,
                     f"opened in {took:.0f} ms, 5 keys max {max(ticks)} ms, saved; lost on save: {lost or 'nothing'}; "
                     f"warnings: {[line[-80:] for line in warned[:2]] or 'none'}"))
    return rows


def value_of(app, path, address):
    return float(json.loads(said(app, "cell", "get", path, address))["text"])


@case
def changed_sources():
    """Sources newer than the links' cache, closed and then open."""
    folder = place("changed", "ely-table")
    loadfiles.sources(folder)
    book, source = os.path.join(folder, "book.xlsx"), os.path.join(folder, "source.xlsx")
    loadfiles.workbook(book, 200, linked=True)
    app = start("changed", "ely-table", [book])
    try:
        closed = value_of(app, book, "Links!A1")
        drive.sh(run.HOST.exe("ely-table", True), source, env=app.env, timeout=120)
        load.wait_opened(app, 2, 60)
        said(app, "cell", "set", source, "Rates!B1", "9.99")
        said(app, "cell", "save", source)
        time.sleep(3)
        live = value_of(app, book, "Links!A1")
    finally:
        app.stop()
    rows = [("ElyTable: an external reference to a closed source shows the source's value", closed == loadfiles.RATES_NOW[0],
             f"Links!A1 shows {closed}; the cache holds {loadfiles.RATES_CACHED[0]}, the source {loadfiles.RATES_NOW[0]}"),
            ("ElyTable: the reference follows its source edited and saved in the same app", live == 9.99,
             f"Links!A1 shows {live} after Rates!B1 became 9.99 and was saved")]
    folder = place("changed", "ely-doc")
    loadfiles.sources(folder)
    path = os.path.join(folder, "linked.docx")
    loadfiles.linked_docx(path, folder)
    app = start("changed", "ely-doc", [path])
    try:
        opened = said(app, "text", "get", path)
        loadfiles.word(os.path.join(folder, "include.docx"), [loadfiles.para("Included changed")])
        time.sleep(3)
        later = said(app, "text", "get", path)
    finally:
        app.stop()
    rows.append(("ElyDoc: INCLUDETEXT shows its source's text on opening", loadfiles.INCLUDED_NOW in opened,
                 f"shows {'the source' if loadfiles.INCLUDED_NOW in opened else 'the cached'} text"))
    rows.append(("ElyDoc: INCLUDETEXT follows its source changed while open", "Included changed" in later,
                 "followed" if "Included changed" in later else "kept the old text"))
    return rows


def edit_others(app, files, rounds, problems):
    for r in range(1, rounds + 1):
        _, failures = load.at_once(app, [words for path in files for words in RECIPES[app.slug].edits(path, r)])
        problems += failures


@case
def three_switching():
    """Three open, tabs switched mid-typing while all are edited."""
    rows = []
    for slug in ("ely-doc", "ely-show"):
        folder = place("switching", slug)
        front = loadfiles.front_file(folder, slug)
        files = [shutil.copy(front, os.path.join(folder, f"{name}.{loadfiles.EXT[slug]}")) for name in "abc"]
        base = {path: loadfiles.text_of(path).count("x") for path in files}
        app = start("switching", slug, files)
        typed, problems, slow, focused = dict.fromkeys(files, 0), [], [], []
        try:
            app.front()
            busy = threading.Thread(target=edit_others, args=(app, files, 3, problems), daemon=True)
            busy.start()
            for _ in range(3):
                for path in files:
                    run.HOST.press(app.process.pid, os.path.basename(path))
                    time.sleep(0.3)
                    RECIPES[slug].into(app.process.pid)
                    time.sleep(2.5)
                    ticks = load.landed(app, RECIPES[slug])
                    focused.append(bool(ticks))
                    if not ticks:
                        # Focus left the document: the user clicks back in.
                        run.HOST.middle(app.process.pid)
                        run.HOST.key(app.process.pid, "escape")
                        RECIPES[slug].into(app.process.pid)
                        time.sleep(2.5)
                    ticks += load.type_keys(app, RECIPES[slug], 2)[0]
                    typed[path] += len(ticks)
                    slow += [tick for tick in ticks if tick > load.TARGET]
            busy.join()
            for path in files:
                said(app, "text", "save", path)
            alive = app.process.poll() is None
        finally:
            app.stop()
        lost = {os.path.basename(path): (loadfiles.text_of(path).count("x") - base[path], keys) for path, keys in typed.items()
                if loadfiles.text_of(path).count("x") - base[path] != keys}
        tokens = {os.path.basename(path): loadfiles.tokens(path) for path in files}
        ok = alive and not lost and not problems and all(focused) and all(found == [3] for found in tokens.values())
        rows.append((f"{APPS[slug]}: three open, tabs switched while typing and while all are edited", ok,
                     f"keys stay in the document after {focused.count(True)} of {len(focused)} tab switches; "
                     f"{sum(typed.values())} keys, {len(slow)} over {load.TARGET} ms; keys lost (saved, typed): {lost or 'none'}; "
                     f"edits saved {tokens}; command failures {problems[:2] or 'none'}"))
    return rows


@case
def save_while_typing():
    """Saves while keys go in; every key reaches the file."""
    rows = []
    for slug in RECIPES:
        folder = place("saving", slug)
        front = loadfiles.front_file(folder, slug)
        app = start("saving", slug, [front])
        saves, failures = [], []

        def keep_saving(stop):
            while not stop.is_set():
                try:
                    saves.append(load.ask(app, [verb(slug), "save", front]))
                except RuntimeError as error:
                    failures.append(str(error))
                time.sleep(0.5)

        try:
            typing_into(app)
            stop = threading.Event()
            saver = threading.Thread(target=keep_saving, args=(stop,), daemon=True)
            saver.start()
            try:
                ticks, _ = load.type_keys(app, RECIPES[slug], 40)
            finally:
                stop.set()
                saver.join()
            RECIPES[slug].finish(app.process.pid)
            time.sleep(1)
            said(app, verb(slug), "save", front)
        finally:
            app.stop()
        ok = load.typed(slug, front, 40) and not failures
        rows.append((f"{APPS[slug]}: saved {len(saves)} times while 40 keys went in", ok,
                     f"keys P95 {load.percentile(ticks, 0.95)} max {max(ticks)} ms; every key saved: {load.typed(slug, front, 40)}; "
                     f"save failures: {failures[:2] or 'none'}"))
    return rows


@case
def close_unsaved():
    """Closing asks; Escape keeps the document, Save writes it."""
    rows = []
    for slug in RECIPES:
        folder = place("closing", slug)
        front = loadfiles.front_file(folder, slug)
        other = os.path.join(folder, f"other.{loadfiles.EXT[slug]}")
        shutil.copy(front, other)
        app = start("closing", slug, [other], front)
        try:
            typing_into(app)
            load.type_keys(app, RECIPES[slug], 5)
            RECIPES[slug].finish(app.process.pid)
            time.sleep(1)
            run.HOST.chord(app.process.pid, "w")
            asked = any(run.HOST.shows(app.process.pid, "Don't Save") or time.sleep(0.25) for _ in range(40))
            if not asked and lines_matching(app, r"closed front\."):
                # ElyMark saves as it goes; nothing is left to ask.
                written = load.typed(slug, front, 5)
                rows.append((f"{APPS[slug]}: closing an autosaved document", written, f"closed at once; the keys on disk {written}"))
                continue
            run.HOST.key(app.process.pid, "escape")
            time.sleep(1.5)
            kept = not lines_matching(app, r"office: closed front\.") and typed_now(app, front, 5) == 5
            run.HOST.chord(app.process.pid, "w")
            for _ in range(40):
                if run.HOST.shows(app.process.pid, "Save"):
                    break
                time.sleep(0.25)
            run.HOST.press(app.process.pid, "Save")
            closed = any(lines_matching(app, r"office: closed front\.") or time.sleep(0.25) for _ in range(80))
        finally:
            app.stop()
        written = load.typed(slug, front, 5)
        rows.append((f"{APPS[slug]}: closing an edited document asks, cancels and saves", asked and kept and closed and written,
                     f"asked {asked}; Escape kept it with its keys {kept}; Save closed it {closed}; the keys on disk {written}"))
    return rows


@case
def engine_killed():
    """The engine dies mid-edit; edited tabs come back from points."""
    rows = []
    for slug in ("ely-doc", "ely-table", "ely-show"):
        folder = place("killed", slug)
        front = loadfiles.front_file(folder, slug)
        others = [shutil.copy(front, os.path.join(folder, f"other{n}.{loadfiles.EXT[slug]}")) for n in range(2)]
        app = start("killed", slug, others, front)
        try:
            typing_into(app)
            load.type_keys(app, RECIPES[slug], 10)
            RECIPES[slug].finish(app.process.pid)
            load.at_once(app, [words for path in others for words in RECIPES[slug].edits(path, 1)])
            # Recovery points come every ten seconds.
            time.sleep(12)
            typing_into(app)
            load.type_keys(app, RECIPES[slug], 5)
            workers = run.HOST.pids(app.process.pid) - {app.process.pid}
            for pid in workers:
                os.kill(pid, 9)
            since = len(app.lines())
            RECIPES[slug].send(app.process.pid)
            noticed = any(lines_matching(app, ENDED, since) or time.sleep(0.25) for _ in range(60))
            reopened = False
            if noticed and any(run.HOST.shows(app.process.pid, "Reopen documents") or time.sleep(0.25) for _ in range(40)):
                since = len(app.lines())
                run.HOST.press(app.process.pid, "Reopen documents")
                reopened = any(len(lines_matching(app, r"opened .+ as tab", since)) >= 3 or time.sleep(0.25) for _ in range(240))
            names = [line.split("opened ")[1].split(" as tab")[0] for line in lines_matching(app, r"opened .+ as tab", since)]
            # Edited tabs reopen as unsaved copies (decisions 2026-10-08).
            points = {name: richfiles.held(slug, path) for name, path in richfiles.newest_points(app, WORK).items()}
            alive = app.process.poll() is None
        finally:
            app.stop()
        front_keys = points.get("front." + loadfiles.EXT[slug], (0, None))[0]
        edited = all(points.get(os.path.basename(path), (0, None))[1] in ([1], "1") for path in others)
        ok = alive and noticed and reopened and front_keys >= 10 and edited
        rows.append((f"{APPS[slug]}: the engine killed mid-edit", ok,
                     f"workers killed {len(workers)}; app alive {alive}; noticed {noticed}; reopened as {names}; "
                     f"the points hold (keys, edit) {points}: front keeps {front_keys} of 15 keys, 10 typed before the "
                     f"last point; the 5 after it are lost by the 10 s point interval"))
    return rows


@case
def zoom():
    """Big documents at extreme zooms, shot for review."""
    rows, folder = [], loadfiles.made(os.path.join(run.ROOT, ".work", "load-samples"))
    for slug in ("ely-doc", "ely-table", "ely-show"):
        shots = place("zoom", slug)
        path = os.path.join(shots, f"big.{loadfiles.EXT[slug]}")
        shutil.copy(os.path.join(folder, f"big.{loadfiles.EXT[slug]}"), path)
        app = start("zoom", slug, [path])
        try:
            app.front()
            time.sleep(3)
            if slug == "ely-table":
                run.HOST.middle(app.process.pid)
            for name, key, times in ZOOMS:
                run.HOST.chord(app.process.pid, "0")
                time.sleep(0.5)
                for _ in range(abs(times) if not name.startswith("wheel") else 0):
                    run.HOST.chord(app.process.pid, key)
                    time.sleep(0.3)
                if name.startswith("wheel"):
                    run.HOST.wheel(app.process.pid, times)
                time.sleep(3)
                run.HOST.shot(app.process.pid, os.path.join(shots, f"{slug[4:]}-{name}.png"))
            failed = lines_matching(app, run.PAINT_FAILED)
        finally:
            app.stop()
        rows.append((f"{APPS[slug]}: zoom 25 % to 400 % and by wheel", not failed,
                     f"tiles failed: {len(failed)}; screenshots for review in {shots}"))
    return rows


@case
def rapid_open_close():
    """Twenty handed over at once, closed by key, thrice."""
    rows = []
    for slug in RECIPES:
        folder = place("rapid", slug)
        front = loadfiles.front_file(folder, slug)
        files = [shutil.copy(front, os.path.join(folder, f"r{n:02d}.{loadfiles.EXT[slug]}")) for n in range(20)]
        app = start("rapid", slug, [front])
        times, memory = [], [app.memory_mb()]
        try:
            app.front()
            for _ in range(3):
                since, began = len(app.lines()), time.monotonic()
                handed = [subprocess.Popen([run.HOST.exe(slug, True), path], env=app.env, creationflags=drive.QUIET) for path in files]
                if any(process.wait(timeout=120) for process in handed):
                    raise RuntimeError(f"{slug}: a hand-over failed")
                load.wait_opened(app, 21, 300, since)
                opened = time.monotonic() - began
                app.front()
                for _ in range(20):
                    run.HOST.chord(app.process.pid, "w")
                    time.sleep(0.05)
                if not any(lines_matching(app, r"closed .+, 1 tabs open", since) or time.sleep(0.25) for _ in range(240)):
                    raise RuntimeError(f"{slug}: 20 Ctrl+W did not leave one tab open")
                times.append((opened * 1000, (time.monotonic() - began - opened) * 1000))
                memory.append(app.memory_mb())
            alive = app.process.poll() is None
        finally:
            app.stop()
        rows.append((f"{APPS[slug]}: 20 handed over at once and closed by key, 3 times", alive,
                     f"open / close ms {[(round(a), round(b)) for a, b in times]}; memory MB {[round(m) for m in memory]}"))
    return rows


@case
def very_large():
    """One huge file per app, typed, edited, saved, read."""
    rows = []
    for slug in RECIPES:
        folder = place("large", slug)
        path = os.path.join(folder, f"huge.{loadfiles.EXT[slug]}")
        if slug == "ely-doc":
            richfiles.pages_docx(path, 500)
        elif slug == "ely-table":
            loadfiles.workbook(path, 60000, typing=True)
            richfiles.xlsx(path, 60000)
        elif slug == "ely-show":
            loadfiles.presentation(path, 20)
        else:
            with open(path, "w", encoding="utf-8", newline="\n") as note:
                note.write(f"# Typing\n\nA paragraph.\n\n{TOKEN}0\n\n" + loadfiles.small_md(0).replace(f"{TOKEN}0", "") * 6000)
        size = os.path.getsize(path) / 2**20
        began = time.monotonic()
        app = start("large", slug, [path])
        opened = (time.monotonic() - began) * 1000
        try:
            typing_into(app)
            ticks, _ = load.type_keys(app, RECIPES[slug], 20)
            RECIPES[slug].finish(app.process.pid)
            edit = [load.ask(app, words) for words in RECIPES[slug].edits(path, 1)]
            saved = load.ask(app, [verb(slug), "save", path])
            memory = app.memory_mb()
        finally:
            app.stop()
        problems = [] if load.typed(slug, path, 20) else ["the 20 keys are not all saved"]
        problems += load.saved(slug, path, 1)
        slow = [tick for tick in ticks if tick > load.TARGET]
        rows.append((f"{APPS[slug]}: a {size:.0f} MB file", not problems and not slow,
                     f"open {opened:.0f} ms; keys P50 {load.percentile(ticks, 0.5)} P95 {load.percentile(ticks, 0.95)} "
                     f"max {max(ticks)} ms; edit {max(edit):.0f} ms; save {saved:.0f} ms; {memory:.0f} MB; problems {problems or 'none'}"))
    return rows


def main():
    global WORK
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", nargs="*", default=list(CASES))
    parser.add_argument("--out", default=os.path.join(run.ROOT, "docs", "perf"))
    args = parser.parse_args()
    if sys.platform != "win32":
        sys.exit("SKIP: the extreme cases drive Windows windows")
    unknown = set(args.cases) - set(CASES)
    if unknown:
        parser.error(f"unknown cases {sorted(unknown)}; known {list(CASES)}")
    WORK = os.path.join(run.ROOT, ".work", f"E{datetime.datetime.now():%H%M%S}")
    os.makedirs(WORK)
    run.HOST = drive.host(WORK)
    load.build(list(RECIPES))
    rows = []
    for name in args.cases:
        for attempt in range(load.RETRIES + 1):
            try:
                rows += CASES[name]()
            except load.Interfered as error:
                if attempt < load.RETRIES:
                    print(f"{error}; again in 30 s", flush=True)
                    time.sleep(30)
                    continue
                rows.append((name, False, f"voided {load.RETRIES + 1} times: {error}"))
            except (RuntimeError, OSError, subprocess.TimeoutExpired, KeyError, ValueError) as error:
                rows.append((name, False, f"stopped: {str(error)[-300:]}"))
            break
        print(f"{name}: {sum(not ok for _, ok, _ in rows)} failing so far", flush=True)
    path = os.path.join(args.out, f"{datetime.date.today().isoformat()}-{load.machine()}-extreme.md")
    os.makedirs(args.out, exist_ok=True)
    lines = [f"# PERF-10 extreme cases {datetime.date.today().isoformat()}", "", "| Case | Result | Measured |", "| --- | --- | --- |"]
    lines += [f"| {name} | {'pass' if ok else 'FAIL'} | {detail.replace('|', '/')} |" for name, ok, detail in rows]
    with open(path, "w", encoding="utf-8", newline="\n") as record:
        record.write("\n".join(lines) + "\n")
    print(path)
    print(f"the work and logs: {WORK}")
    if not all(ok for _, ok, _ in rows):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
