"""PERF-02: each key's feedback, and main-thread stalls meanwhile."""

import os
import re
import shutil
import tempfile
import time

import run
import search

# A typed key's time, or why it never drew.
TILE = r"lok: edit \d+ (?:text|key) to tile (\d+) ms"
NEVER = r"lok: edit \d+ (?:text|key) never drawn: .+"
FRAME = r"ui: key_down to frame (\d+) ms"
STALL = r"main thread was busy (\d+) ms"


class Keys:
    """Where one app takes keys, and what answers them."""

    def __init__(self, slug, args, ready, answer, into, key="x", home=None):
        self.slug, self.args, self.ready, self.answer, self.into, self.key = slug, args, ready, answer, into, key
        # Made on demand: seeding mail takes a while.
        self.home = home

    def send(self, pid):
        if self.key == "pagedown":
            run.HOST.key(pid, "pagedown")
        else:
            run.HOST.type(pid, self.key)


def into_document(pid):
    """Nothing to do: the blank page holds the caret."""


def into_box(pid):
    """Impress: Tab picks the first box, Enter edits it."""
    run.HOST.key(pid, "tab")
    run.HOST.enter(pid)


def into_field(pid):
    run.HOST.click_field(pid)


def into_search(pid):
    """ElyTree: the history's search box."""
    run.HOST.settle(pid, "History")
    run.HOST.press(pid, "History")
    run.HOST.click_field(pid)


def into_paragraph(pid):
    run.HOST.settle(pid, "A paragraph.")
    run.HOST.press(pid, "A paragraph.")


def keys(work, found):
    note = os.path.join(work, "typing.md")
    with open(note, "w", encoding="utf-8", newline="\n") as file:
        file.write("# Typing\n\nA paragraph.\n")
    return [
        Keys("ely-doc", [], "opened Untitled 1 ", TILE, into_document),
        Keys("ely-table", [], "opened Untitled 1 ", TILE, into_document, key="1"),
        Keys("ely-show", [], "opened Untitled 1 ", TILE, into_box),
        Keys("ely-pdf", [found["pages.pdf"][0]], r"pdf: first tile drawn, page 0$", FRAME, into_document, key="pagedown"),
        Keys("ely-mark", [note], r'mark: shown as "visual"', FRAME, into_paragraph),
        Keys("ely-tree", [found["repo"][0]], "tree: opened", FRAME, into_search),
        Keys("ely-mail", [], r"mail: list read \d+ shows \d+ messages, searched false", FRAME, into_field,
             home=lambda: search.seeded(work)),
    ]


def answers(lines, answer):
    """Keys answered so far; a key that never drew raises."""
    never = next((line for line in lines if re.search(NEVER, line)), None)
    if never:
        raise RuntimeError(f"a key never drew: {never[-90:]}")
    return sum(1 for line in lines if re.search(answer, line))


def timed(lines):
    """One timing per typed input; else it raises."""
    answers(lines, TILE)
    times = [m.groups() for line in lines if (m := re.search(r"lok: edit (\d+) text to tile (\d+) ms", line))]
    typed = [m.group(1) for line in lines if (m := re.search(r"lok: input (\d+) text waited", line))]
    seqs = [seq for seq, _ in times]
    if not typed or sorted(seqs) != sorted(typed):
        raise RuntimeError(f"typed inputs {typed} gave timings {seqs}")
    return [int(ms) for _, ms in times]


def judged(recipe, lines, since, runs):
    """Key times and stalls; a failed draw voids them."""
    failed = [line for line in lines if re.search(run.PAINT_FAILED, line)]
    if failed:
        raise RuntimeError(f"{recipe.slug}: a page failed to draw while typing: {failed[0]}")
    ticks = [int(m.group(1)) for line in lines[since:] if (m := re.search(recipe.answer, line))]
    if len(ticks) != runs:
        raise RuntimeError(f"{recipe.slug}: {runs} keys gave {len(ticks)} timings")
    stalls = [int(m.group(1)) for line in lines[since:] if (m := re.search(STALL, line))]
    return ticks, stalls


def typing(work, recipe, runs):
    """Ms from each key to feedback; stalls over 100 ms."""
    home = recipe.home() if recipe.home else os.path.join(work, "home-typing-" + recipe.slug)
    app = run.App(recipe.slug, home, recipe.args, frames=True)
    try:
        app.wait(recipe.ready)
        app.front()
        recipe.into(app.process.pid)
        # Getting in takes keys too; they reach the next flush.
        time.sleep(2.5)
        since = len(app.lines())
        for n in range(runs):
            recipe.send(app.process.pid)
            # Each key must show; a lost one fails the run.
            for _ in range(800):
                if answers(app.lines()[since:], recipe.answer) > n:
                    break
                time.sleep(0.01)
            else:
                raise RuntimeError(f"{recipe.slug}: key {n + 1} of {runs} never showed")
            time.sleep(0.15)
        lines = app.lines()
    finally:
        app.stop()
    ticks, stalls = judged(recipe, lines, since, runs)
    if stalls:
        # What the main thread did is in the log.
        kept = tempfile.mkdtemp(prefix=f"ely-bench-stall-{recipe.slug}-")
        shutil.copy(app.log, kept)
        print(f"{recipe.slug}: main thread busy {stalls} ms; its log: {kept}")
    return ticks, stalls


if __name__ == "__main__":
    # lag.sh: a log and its first measured line.
    import sys
    with open(sys.argv[1], encoding="utf-8", errors="replace") as log:
        measured = log.read().splitlines()[int(sys.argv[2]) - 1:]
    try:
        timed(measured)
    except RuntimeError as error:
        sys.exit(str(error))
