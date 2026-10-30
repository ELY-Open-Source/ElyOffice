"""PERF-06: first results of a common query, as shown."""

import functools
import os
import pathlib
import re

import drive
import run

MAILS = 100_000


def spec(folder):
    """A package by folder; names may repeat in the graph."""
    return "path+" + pathlib.Path(run.ROOT, folder).as_uri()


def tree(work, repo, runs):
    """History's first screen, then a query's results, from each input."""
    home = os.path.join(work, "home-search-tree")
    first, found = [], []
    for _ in range(runs):
        app = run.App("ely-tree", home, [repo])
        pid = app.process.pid
        try:
            app.wait("tree: opened")
            app.front()
            run.HOST.settle(pid, "History")
            since = len(app.lines())
            first.append(app.answer(r"history shows \d+ commits, searched false", run.HOST.press(pid, "History"), since))
            run.HOST.click_field(pid)
            run.HOST.type(pid, "part 42")
            since = len(app.lines())
            found.append(app.answer(r"searched true", run.HOST.enter(pid), since))
        finally:
            app.stop()
    return first, found


@functools.cache
def seeded(work):
    """A mail folder of MAILS messages, indexed by the engine."""
    home = os.path.join(work, "home-search-mail")
    env = dict(os.environ, ELY_SEED=home, ELY_MAILS=str(MAILS))
    drive.sh("cargo", "test", "--release", "-q", "-p", spec("crates/ely-mail-engine"), "--test", "search_perf",
             "--", "--ignored", "--exact", "seeds_a_data_root_for_frame_timing", cwd=run.ROOT, env=env)
    return home


def mail(home, runs):
    """From the last key typed to the first results shown."""
    found = []
    for _ in range(runs):
        app = run.App("ely-mail", home)
        pid = app.process.pid
        try:
            app.wait(r"mail: list read \d+ shows \d+ messages, searched false")
            app.front()
            run.HOST.click_field(pid)
            since = len(app.lines())
            sent = run.HOST.type(pid, "report")
            found.append(app.answer(r"shows [1-9]\d* messages, searched true", sent, since))
        finally:
            app.stop()
    return found
