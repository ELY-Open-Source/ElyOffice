#!/usr/bin/env python3
"""PERF-10's extreme cases on real windows, each one checked."""

import argparse
import contextlib
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
from run import APPS, App

# Name, steps up from the 25 % floor, wheel notches.
ZOOMS = [("25", 0, 0), ("67", 2, 0), ("100", 4, 0), ("125", 5, 0), ("400", 9, 0), ("wheel+2", 0, 2), ("wheel-3", 5, -3)]
CASES = {}
VERB = {**dict.fromkeys(RECIPES, "text"), "ely-table": "cell"}
WORK = ""


def case(fn):
    CASES[fn.__name__.replace("_", "-")] = fn
    return fn


def place(name, slug):
    os.makedirs(folder := os.path.join(WORK, name, slug), exist_ok=True)
    return folder


def start(name, slug, files, front=None):
    """`slug` with `files` open; `front` handed over last, active."""
    app = App(slug, load.data_home(WORK, f"{name[:3]}{slug[4:7]}"), files, frames=True)
    try:
        load.first_place(app)
        load.wait_opened(app, len(files), 60 + 15 * len(files))
        if front:
            since = len(app.lines())
            drive.sh(run.HOST.exe(slug, True), front, env=app.env, timeout=120)
            load.wait_opened(app, len(files) + 1, 120, since)
    except BaseException:
        app.stop()
        raise
    return app


@contextlib.contextmanager
def running(name, slug, files, front=None):
    """One case's app; void if someone used the desktop."""
    app, clicked = start(name, slug, files, front), run.HOST.clicks
    try:
        yield app
        # The log flushes every two seconds.
        time.sleep(2.5)
        downs = [line for line in app.lines() if "ui: mouse_down to frame" in line]
        if len(downs) > run.HOST.clicks - clicked:
            raise load.Interfered(f"{app.slug}: {len(downs)} clicks, the driver made {run.HOST.clicks - clicked}")
        if why := load.interfered(app, len(app.lines()), moves=True):
            raise load.Interfered(why)
    finally:
        app.stop()


def typing_into(app, into=None):
    """The app in front, the caret where keys land."""
    if app.slug != "ely-mark":
        app.wait(r"lok: first tile drawn", timeout=120)
        time.sleep(2)
    app.front()
    (into or RECIPES[app.slug].into)(app.process.pid)
    time.sleep(2.5)


def typed_now(app, path, keys):
    """What the open document holds of `keys` typed, read live."""
    if app.slug == "ely-table":
        value = json.loads(app.said("cell", "get", path, "Typing!A1"))["text"]
        return len(value) if set(value) <= {"1"} else -1
    found = re.findall(r"x+", app.said("text", "get", path))
    return max((len(run_) for run_ in found), default=0)


def lines_matching(app, pattern, since=0):
    return [line for line in app.lines()[since:] if re.search(pattern, line)]


@case
def missing_sources():
    """Linked files whose sources are gone open, type and save."""
    rows = []
    for slug in ("ely-doc", "ely-table"):
        folder = place("missing", slug)
        path = os.path.join(folder, f"linked.{loadfiles.EXT[slug]}")
        loadfiles.linked_docx(path, folder) if slug == "ely-doc" else loadfiles.workbook(path, 2000, linked=True, typing=True)
        began = time.monotonic()
        with running("missing", slug, [path]) as app:
            took = (time.monotonic() - began) * 1000
            typing_into(app)
            ticks, _ = load.type_keys(app, RECIPES[slug], 5)
            RECIPES[slug].finish(app.process.pid)
            app.said(VERB[slug], "save", path)
            warned = lines_matching(app, r"(WARN|ERROR).*(link|source|not found|missing)")
        kept = loadfiles.word_links(path) if slug == "ely-doc" else loadfiles.book_links(path)
        lost = [what for what, ok in kept.items() if not ok and "shows" not in what]
        rows.append((f"{APPS[slug]}: opens a file whose link sources are missing", not lost,
                     f"opened in {took:.0f} ms, 5 keys max {max(ticks)} ms, saved; lost on save: {lost or 'nothing'}; "
                     f"warnings: {[line[-80:] for line in warned[:2]] or 'none'}"))
    return rows


@case
def changed_sources():
    """Saved values on opening; Update links reads the sources (LOK-11)."""
    folder = place("changed", "ely-table")
    loadfiles.sources(folder)
    book, source = os.path.join(folder, "book.xlsx"), os.path.join(folder, "source.xlsx")
    loadfiles.workbook(book, 200, linked=True)
    with running("changed", "ely-table", [book]) as app:
        a1 = lambda: float(json.loads(app.said("cell", "get", book, "Links!A1"))["text"])
        opened = a1()
        app.update_links()
        updated = a1()
        drive.sh(run.HOST.exe("ely-table", True), source, env=app.env, timeout=120)
        load.wait_opened(app, 2, 60)
        app.said("cell", "set", source, "Rates!B1", "9.99")
        held, why = load.refused_for(app, ["cell", "save", source])
        time.sleep(3)
        waited = a1()
        drive.sh(run.HOST.exe("ely-table", True), book, env=app.env, timeout=120)
        app.update_links()
        again = a1()
    shown = {"an external reference shows its saved value on opening": (opened, loadfiles.RATES_CACHED[0]),
             "Update links reads the source": (updated, loadfiles.RATES_NOW[0]),
             "a later save of the source waits for the user": (waited, loadfiles.RATES_NOW[0]),
             "Update links again reads the saved source": (again, 9.99)}
    return [("ElyTable: the source saves right after Update links read it", not why, f"refused for {held:.0f} s: {why}" if why else "saved at once")] + [
        (f"ElyTable: {what}", got == want, f"Links!A1 shows {got}, wants {want}") for what, (got, want) in shown.items()]


@case
def changed_word_sources():
    """INCLUDETEXT, linked picture and OLE preview (LOK-11)."""
    folder = place("changed", "ely-doc")
    loadfiles.sources(folder)
    path = os.path.join(folder, "linked.docx")
    loadfiles.linked_docx(path, folder)
    with running("changed", "ely-doc", [path]) as app:
        shoot = lambda name: time.sleep(3) or run.HOST.shot(app.process.pid, os.path.join(folder, f"links-{name}.png"))
        typing_into(app, run.HOST.park)
        shoot("opened")
        opened = app.said("text", "get", path)
        app.update_links()
        shoot("updated")
        updated = app.said("text", "get", path)
        loadfiles.word(os.path.join(folder, "include.docx"), [loadfiles.para("Included changed")])
        loadfiles.sources(folder, rates=[9 + m / 100 for m in range(1, 13)], seed=5)
        shoot("changed")
        waited = app.said("text", "get", path)
        app.update_links()
        shoot("renewed")
        later = app.said("text", "get", path)
        app.said("text", "save", path)
    seen = {"INCLUDETEXT shows its saved text on opening": loadfiles.INCLUDED_CACHED in opened,
            "Update links reads INCLUDETEXT's source": loadfiles.INCLUDED_NOW in updated,
            "a later save of the source waits for the user": "Included changed" not in waited,
            "Update links again reads the saved source": "Included changed" in later}
    rows = [(f"ElyDoc: {what}", ok, "read with text get") for what, ok in seen.items()]
    rows += [(f"ElyDoc: {what}", ok, f"read in the saved file after the sources changed; screens {folder}\\links-*.png")
             for what, ok in loadfiles.word_updates(path, folder).items()]
    return rows


def edit_others(app, files, rounds, problems):
    for r in range(1, rounds + 1):
        problems += load.at_once(app, [words for path in files for words in RECIPES[app.slug].edits(path, r)])[1]


@case
def three_switching():
    """Three open, tabs switched mid-typing while all are edited."""
    rows = []
    for slug in ("ely-doc", "ely-show"):
        folder = place("switching", slug)
        front = loadfiles.front_file(folder, slug)
        files = [shutil.copy(front, os.path.join(folder, f"{name}.{loadfiles.EXT[slug]}")) for name in "abc"]
        base = {path: loadfiles.text_of(path).lower().count("x") for path in files}  # Autocorrect capitalizes some x.
        with running("switching", slug, files) as app:
            typed, problems, slow, focused = dict.fromkeys(files, 0), [], [], []
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
                app.said("text", "save", path)
            alive = app.process.poll() is None
        lost = {os.path.basename(path): (loadfiles.text_of(path).lower().count("x") - base[path], keys) for path, keys in typed.items()
                if loadfiles.text_of(path).lower().count("x") - base[path] != keys}
        tokens = {os.path.basename(path): loadfiles.tokens(path) for path in files}
        ok = alive and not lost and not problems and not slow and all(focused) and all(found == [3] for found in tokens.values())
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
        with running("saving", slug, [front]) as app:
            saves, failures = [], []

            def keep_saving(stop):
                while not stop.is_set():
                    try:
                        saves.append(load.ask(app, [VERB[slug], "save", front]))
                    except (RuntimeError, OSError, subprocess.TimeoutExpired) as error:
                        failures.append(str(error))
                    time.sleep(0.5)
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
            app.said(VERB[slug], "save", front)
        slow = [tick for tick in ticks if tick > load.TARGET]
        ok = richfiles.typed(slug, front, 40) and not failures and not slow
        rows.append((f"{APPS[slug]}: saved {len(saves)} times while 40 keys went in", ok,
                     f"keys P95 {load.percentile(ticks, 0.95)} max {max(ticks)} ms, {len(slow)} over {load.TARGET}; every key saved: {richfiles.typed(slug, front, 40)}; "
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
        with running("closing", slug, [other], front) as app:
            typing_into(app)
            load.type_keys(app, RECIPES[slug], 5)
            RECIPES[slug].finish(app.process.pid)
            time.sleep(1)
            run.HOST.chord(app.process.pid, "w")
            asked = any(run.HOST.shows(app.process.pid, "Don't Save") or time.sleep(0.25) for _ in range(40))
            if not asked and lines_matching(app, r"closed front\."):
                # ElyMark saves as it goes; nothing is left to ask.
                written = richfiles.typed(slug, front, 5)
                rows.append((f"{APPS[slug]}: closing an autosaved document", written, f"closed at once; the keys on disk {written}"))
                continue
            run.HOST.key(app.process.pid, "escape")
            time.sleep(1.5)
            kept = not lines_matching(app, r"office: closed front\.") and typed_now(app, front, 5) == 5
            run.HOST.chord(app.process.pid, "w")
            any(run.HOST.shows(app.process.pid, "Save") or time.sleep(0.25) for _ in range(40))
            run.HOST.press(app.process.pid, "Save")
            closed = any(lines_matching(app, r"office: closed front\.") or time.sleep(0.25) for _ in range(80))
        written = richfiles.typed(slug, front, 5)
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
        with running("killed", slug, others, front) as app:
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
            noticed = any(lines_matching(app, r"the engine ended", since) or time.sleep(0.25) for _ in range(60))
            reopened = False
            if noticed and any(run.HOST.shows(app.process.pid, "Reopen documents") or time.sleep(0.25) for _ in range(40)):
                since = len(app.lines())
                run.HOST.press(app.process.pid, "Reopen documents")
                reopened = any(len(lines_matching(app, r"opened .+ as tab", since)) >= 3 or time.sleep(0.25) for _ in range(240))
            names = [line.split("opened ")[1].split(" as tab")[0] for line in lines_matching(app, r"opened .+ as tab", since)]
            # Edited tabs reopen as unsaved copies (decisions 2026-10-08).
            points = {name: richfiles.held(slug, path) for name, path in richfiles.newest_points(app, WORK).items()}
            alive = app.process.poll() is None
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
        missed = []
        with running("zoom", slug, [path]) as app:
            app.front()
            time.sleep(3)
            for name, ups, wheel in ZOOMS:
                before = len(app.lines())
                run.HOST.chord(app.process.pid, "0")
                for _ in range(9):
                    run.HOST.chord(app.process.pid, "minus")
                # The log reaches the file within two seconds.
                time.sleep(2.5)
                floor, since = load.zooms(app.lines()[before:])[-1:], len(app.lines())
                for _ in range(ups):
                    run.HOST.chord(app.process.pid, "plus")
                    time.sleep(0.3)
                if wheel:
                    run.HOST.wheel(app.process.pid, wheel)
                if slug == "ely-table":
                    # A1's selection border, against its gridlines.
                    run.HOST.chord(app.process.pid, "home")
                time.sleep(3)
                run.HOST.shot(app.process.pid, os.path.join(shots, f"{slug[4:]}-{name}.png"))
                seen = load.zooms(app.lines()[since:])
                if floor != [25] or seen != load.walk(ups, wheel):
                    missed.append(f"{name}: floor {floor}, then {seen}")
            failed = lines_matching(app, run.PAINT_FAILED)
        rows.append((f"{APPS[slug]}: zoom 25 % to 400 % and by wheel", not failed and not missed,
                     f"tiles failed: {len(failed)}; zoom not as asked: {missed or 'none'}; screenshots in {shots}"))
    return rows


@case
def rapid_open_close():
    """Twenty handed over at once, closed by key, thrice."""
    rows = []
    for slug in RECIPES:
        folder = place("rapid", slug)
        front = loadfiles.front_file(folder, slug)
        files = [shutil.copy(front, os.path.join(folder, f"r{n:02d}.{loadfiles.EXT[slug]}")) for n in range(20)]
        with running("rapid", slug, [front]) as app:
            times, memory = [], [app.memory_mb()]
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
        rows.append((f"{APPS[slug]}: 20 handed over at once and closed by key, 3 times", alive,
                     f"open / close ms {[(round(a), round(b)) for a, b in times]}; memory MB {[round(m) for m in memory]}"))
    return rows


@case
def very_large():
    """One huge file per app, typed, edited, saved, read."""
    rows = []
    for slug in RECIPES:
        try:
            rows.append(huge(slug))
        except load.Interfered:
            raise
        except (RuntimeError, subprocess.TimeoutExpired) as error:
            rows.append((f"{APPS[slug]}: a huge file", False, f"stopped: {str(error)[-200:]}"))
    return rows


def huge(slug):
    path = os.path.join(place("large", slug), f"huge.{loadfiles.EXT[slug]}")
    richfiles.huge_sample(slug, path)
    size, began = os.path.getsize(path) / 2**20, time.monotonic()
    with running("large", slug, [path]) as app:
        opened = (time.monotonic() - began) * 1000
        # ElyMark shows files past LARGEST_READ as source.
        typing_into(app, run.HOST.middle if slug == "ely-mark" else None)
        ticks, typed_lines = load.type_keys(app, RECIPES[slug], 20)
        RECIPES[slug].finish(app.process.pid)
        edit = [load.ask(app, words) for words in RECIPES[slug].edits(path, 1)]
        saved = load.ask(app, [VERB[slug], "save", path])
        memory = app.memory_mb()
    problems = [] if richfiles.typed(slug, path, 20) else ["the 20 keys are not all saved"]
    problems += richfiles.saved(slug, path, 1)
    seen = load.frames(typed_lines)
    problems += [why] if (why := load.frames_failed(seen)) else []
    drawn = "no frame line" if seen is None else f"frames over 8.3 ms {seen[2]}/{seen[0]}, dirty to shown max {seen[4]:.1f} ms"
    slow = [tick for tick in ticks if tick > load.TARGET]
    return (f"{APPS[slug]}: a {size:.0f} MB file", not problems and not slow,
            f"open {opened:.0f} ms; keys P50 {load.percentile(ticks, 0.5)} P95 {load.percentile(ticks, 0.95)} "
            f"max {max(ticks)} ms; {drawn}; edit {max(edit):.0f} ms; save {saved:.0f} ms; {memory:.0f} MB; problems {problems or 'none'}")


def main():
    global WORK
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", nargs="*", default=list(CASES))
    parser.add_argument("--out", default=os.path.join(run.ROOT, "docs", "perf"))
    args = parser.parse_args()
    if sys.platform != "win32":
        sys.exit("SKIP: the extreme cases drive Windows windows")
    if unknown := set(args.cases) - set(CASES):
        parser.error(f"unknown cases {sorted(unknown)}; known {list(CASES)}")
    os.makedirs(WORK := os.path.join(run.ROOT, ".work", f"E{datetime.datetime.now():%H%M%S}"))
    run.HOST = drive.host(WORK)
    load.watch_memory()
    load.build(list(RECIPES))
    rows = []
    for name in args.cases:
        for attempt in range(load.RETRIES + 1):
            try:
                rows += CASES[name]()
                load.free_gb()
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
    print(path, f"the work and logs: {WORK}", sep="\n")
    if not all(ok for _, ok, _ in rows):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
