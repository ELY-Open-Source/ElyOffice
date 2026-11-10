"""PERF-08: 150 opens and closes; documents, handles, memory settle."""

import re
import shutil
import subprocess
import os
import time

import drive
import run
import search

# ElyDoc's memory settles by about cycle 100.
CYCLES = 150
# Opened once on first use, plus the thread pool's churn.
ONCE = 8
# A page per cycle leaks; less is warm-up.
PAGE_MB = 4 / 1024


def counted(lines, pattern):
    """The last count `pattern` logged; none fails."""
    counts = [int(m.group(1)) for line in lines if (m := re.search(pattern, line))]
    if not counts:
        raise RuntimeError(f"no count matching {pattern!r} was logged")
    return counts[-1]


# Anonymous kinds, judged by trend: a sync may hold one.
UNNAMED = ("socket", "pipe")


def handles_kept(who, before, now, cycle, system):
    """Named handles past baseline: only `system` ones."""
    # Read-only system resources load on first use.
    left = sorted(handle for handle in now - before if not handle[1].startswith(system) and handle[1] not in UNNAMED)
    if left:
        raise RuntimeError(f"{who}: cycle {cycle} leaves handles: {left}")


def steady(who, what, counts, slack=0):
    """Not climbing over `slack` a cycle in the last quarter."""
    quarter = max(1, len(counts) // 4)
    late, prior = counts[-quarter:], counts[-2 * quarter:-quarter]
    # A step that settled before it is not a climb.
    rate = (late[-1] - prior[-1]) / quarter
    if min(late) > max(prior) and rate > slack:
        raise RuntimeError(f"{who}: {len(counts)} cycles still took the {what} from {prior} to {late}")


def grown(root, families):
    """Each pid's first and last KB; `root` is the app."""
    pids = sorted(set(families[0]) | set(families[-1]))
    return {("app" if pid == root else pid): (families[0].get(pid), families[-1].get(pid)) for pid in pids}


def handles_steady(who, before, counts):
    """Settled, and none kept past ONCE."""
    steady(who, "handles", counts)
    late = min(counts[-max(1, len(counts) // 4):])
    if late > before + ONCE:
        raise RuntimeError(f"{who}: {len(counts)} cycles keep {late - before:g} handles past {before:g}")


class Cycle:
    """How one app opens, closes and counts a document."""

    def __init__(self, slug, args, ready, counts, home=None):
        self.slug, self.args, self.ready, self.counts, self.home = slug, args, ready, counts, home

    def live(self, app, since):
        """Each count since line `since`, as logged."""
        return tuple(counted(app.lines()[since:], pattern) for pattern in self.counts)

    def first(self, app):
        return self.live(app, 0)


class Office(Cycle):
    """Keyboard New; Doc and Show ask for a template."""

    def __init__(self, slug, template):
        super().__init__(slug, [], "opened Untitled 1 ", [r"(\d+) tabs open", r"lok-worker: (\d+) documents open"])
        self.template = template

    def open(self, app, n):
        run.HOST.chord(app.process.pid, "n")
        if self.template:
            for _ in range(40):
                if run.HOST.shows(app.process.pid, "Use template"):
                    break
                time.sleep(0.25)
            run.HOST.settle(app.process.pid, "Use template")
            run.HOST.press(app.process.pid, "Use template")
        return f"opened Untitled {n + 2} "

    def close(self, app, n):
        run.HOST.chord(app.process.pid, "w")
        return f"office: closed Untitled {n + 2},"


class Handed(Cycle):
    """A second app copy hands it `other`; keys close it."""

    def __init__(self, slug, first, other, opened, closed, counts):
        super().__init__(slug, [first], opened(first), counts)
        self.other, self.opened, self.closed = other, opened, closed

    def open(self, app, n):
        handed = subprocess.run([run.HOST.exe(self.slug), self.other], env=app.env, capture_output=True, timeout=60,
                                creationflags=drive.QUIET)
        if handed.returncode:
            raise RuntimeError(f"{self.slug}: handing over {self.other} exited {handed.returncode}")
        return self.opened(self.other)

    def close(self, app, n):
        app.front()
        run.HOST.chord(app.process.pid, "w")
        return self.closed(self.other)


class Compose(Cycle):
    """ElyMail: a new message, discarded blank; writing needs an account."""

    def __init__(self, work):
        super().__init__("ely-mail", [], r"mail: list read \d+ shows", [r"a window closed, (\d+) open"],
                         home=lambda: search.seeded(work))

    def open(self, app, n):
        run.HOST.chord(app.process.pid, "n")
        return "mail: a compose window opened"

    def first(self, app):
        # The main window alone; none has closed yet.
        return (1,)

    def close(self, app, n):
        run.HOST.settle(app.process.pid, "Discard")
        run.HOST.press(app.process.pid, "Discard")
        return r"a window closed, \d+ open"


def repo(path, git):
    """A one-commit repository."""
    os.makedirs(path)
    for args in (["init", "-q"], ["-c", "user.name=Ely", "-c", "user.email=ely@example.com",
                                  "commit", "-q", "--allow-empty", "-m", "first"]):
        subprocess.run([git, *args], cwd=path, check=True, capture_output=True, creationflags=drive.QUIET)
    return path


def recipes(work):
    """Each app's cycle, on small files of its own."""
    files = {}
    for name, source in [("first.pdf", "office/t.pdf"), ("other.pdf", "office/t.pdf")]:
        files[name] = os.path.join(work, name)
        shutil.copy(os.path.join(run.ROOT, "tests", "fixtures", source), files[name])
    for name in ["first.md", "other.md"]:
        files[name] = os.path.join(work, name)
        with open(files[name], "w", encoding="utf-8", newline="\n") as note:
            note.write(f"# {name}\n\nA paragraph.\n")
    office = lambda name: f"opened {re.escape(os.path.basename(name))} as tab"
    closed = lambda name: f"office: closed {re.escape(os.path.basename(name))},"
    tree = lambda path: re.escape(f"tree: opened {os.path.realpath(path)}")
    return [
        Office("ely-doc", True), Office("ely-table", False), Office("ely-show", True),
        Handed("ely-pdf", files["first.pdf"], files["other.pdf"], office, closed,
               [r"(\d+) tabs open", r"pdf-worker: \w+ document \d+.*, (\d+) documents open"]),
        Handed("ely-mark", files["first.md"], files["other.md"], office, closed, [r"(\d+) tabs open"]),
        Handed("ely-tree", repo(os.path.join(work, "repo-a"), run.HOST.git()),
               repo(os.path.join(work, "repo-b"), run.HOST.git()), tree,
               lambda path: re.escape(f"tree: closed {os.path.realpath(path)},"), [r"(\d+) repositories open"]),
        Compose(work),
    ]


def one(app, recipe, n, first):
    """Open and close once; the counts must return to `first`."""
    since = len(app.lines())
    app.wait(recipe.open(app, n), since=since, timeout=30)
    since = len(app.lines())
    time.sleep(1)
    app.wait(recipe.close(app, n), since=since, timeout=30)
    # The engine lets go later; a fresh report must come.
    for _ in range(100):
        try:
            if recipe.live(app, since) == first:
                break
        except RuntimeError:
            pass
        time.sleep(0.05)
    else:
        raise RuntimeError(f"{recipe.slug}: cycle {n} closed, yet the counts never returned to {first}")
    time.sleep(0.5)
    return since


def cycles(work, recipe):
    """Baseline, then each cycle's memory MB, handles and live counts."""
    home = recipe.home() if recipe.home else os.path.join(work, "home-cycles-" + recipe.slug)
    app = run.App(recipe.slug, home, recipe.args)
    try:
        app.wait(recipe.ready)
        app.front()
        time.sleep(2)
        first = recipe.first(app)
        # A first window loads the system's plumbing once.
        one(app, recipe, 0, first)
        before = app.handles()
        baseline = (app.memory_mb(), sum(before.values()), first)
        seen, families = [], []
        for n in range(1, CYCLES + 1):
            since = one(app, recipe, n, first)
            now = app.handles()
            if run.HOST.system is not None:
                handles_kept(recipe.slug, before, now, n, run.HOST.system)
            families.append(app.family())
            seen.append((sum(families[-1].values()) / 1024, sum(now.values()), recipe.live(app, since)))
        counts = [count for _, count, _ in seen]
        if run.HOST.system is None:
            handles_steady(recipe.slug, baseline[1], counts)
        else:
            # Named system handles may settle late.
            steady(recipe.slug, "handles", counts)
        # Thread arenas keep freed memory: settling is the test.
        try:
            steady(recipe.slug, "MB", [memory for memory, _, _ in seen], PAGE_MB)
        except RuntimeError as error:
            raise RuntimeError(f"{error}; KB per process, first cycle to last: {grown(app.process.pid, families)}") from error
    finally:
        app.stop()
    return baseline, seen
