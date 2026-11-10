#!/usr/bin/env python3
"""Records cite real files; a tick has a passed review."""
import os
import re
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
RECORDS = ["tasks/TASKS.md", "tasks/BUGS.md", "docs/decisions.md", "docs/BugList.md"]
GONE, LOCAL = "（已移除）", "（本机，未入库）"
# Repo-rooted paths; tests/ only where the root has it.
REF = re.compile(r"(?<![\w/.-])`?((?:\.work|\.gitea|\.github|scripts|apps|crates|docs|packaging|tasks"
                 r"|tests/(?:scripts|perf|engines|fixtures))/[A-Za-z0-9_.,{}*/-]*[A-Za-z0-9_/*])`?")


def faults(text, tracked):
    found = []
    for m in REF.finditer(text):
        ref, after = m.group(1), text[m.end():m.end() + len(LOCAL)]
        if ref.startswith(".work/"):
            if not after.startswith(LOCAL):
                found.append(f"{ref} is local evidence, unmarked")
        # Globs and braces stand for several files.
        elif any(c in ref for c in "{}*") or after.startswith(GONE):
            continue
        else:
            found += [f"{one} is not in the repository" for one in spread(ref) if not present(one, tracked)]
    return found


def spread(ref):
    """A review range like -r1..r3 names each round."""
    m = re.search(r"-r(\d+)\.\.r(\d+)\.md$", ref)
    # A reversed range names no round: check it as written.
    if not m or int(m.group(1)) > int(m.group(2)):
        return [ref]
    return [f"{ref[:m.start()]}-r{n}.md" for n in range(int(m.group(1)), int(m.group(2)) + 1)]


def present(ref, tracked):
    return ref in tracked or any(path.startswith(ref.rstrip("/") + "/") for path in tracked)


def last_rounds(tracked):
    """Each reviewed task's last round: number and verdict."""
    last = {}
    for path in tracked:
        m = re.fullmatch(r"tasks/reviews/(.+)-r(\d+)\.md", path)
        if m and int(m.group(2)) > last.get(m.group(1), (0, ""))[0]:
            with open(os.path.join(ROOT, path), encoding="utf-8") as f:
                last[m.group(1)] = (int(m.group(2)), f.readline().strip())
    return last


def unpassed(text, last):
    """Ticked tasks whose last round failed, re-reviewed nowhere."""
    found = []
    for task, rest in re.findall(r"^\s*- \[x\] (\S+) (.*)$", text, re.M):
        if last.get(task, (0, ""))[1] != "FAIL":
            continue
        again = re.findall(r"([A-Z][\w.-]*) 复[审核]通过", rest)
        if not any(last.get(other, (0, ""))[1] == "PASS" for other in again if other != task):
            found.append(f"{task} is ticked, its last review failed, and no passed re-review is named")
    return found


def main():
    tracked = set(subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.splitlines())
    # Each must be caught: gaps, truncated names, unmarked local evidence.
    bad = ["see `scripts/no-such.sh`", "`scripts/check.s`", "`scripts/check..sh`", "`tasks/reviews/no-such-r1.md`",
           "`tasks/reviews/R4.4-r1..r9.md`", "`tasks/reviews/R4.4-r9..r1.md`", "log in .work/R1/run.log", "`.work/review/`"]
    for text in bad:
        if not faults(text, tracked):
            sys.exit(f"FAIL: records.py passes {text!r}")
    if not faults("`tasks/R4-acceptance.md`", tracked - {"tasks/R4-acceptance.md"}):
        sys.exit("FAIL: records.py passes a removed task file")
    good = "`scripts/check.sh`, `scripts/`, `tasks/reviews/R4.4-r1..r3.md`, `scripts/gone.sh`（已移除）, .work/a.log（本机，未入库）, `.work/review/`（本机，未入库）"
    if faults(good, tracked):
        sys.exit(f"FAIL: records.py rejects present or marked references: {faults(good, tracked)}")
    rounds = {"A": (3, "FAIL"), "B": (1, "PASS"), "C": (2, "FAIL")}
    for text in ["- [x] A done", "- [x] A 经 C 复审通过", "- [x] A 经 A 复审通过"]:
        if not unpassed(text, rounds):
            sys.exit(f"FAIL: records.py passes {text!r}")
    if unpassed("- [x] A 经 B 复审通过\n- [~] C fixed\n- [x] B ok", rounds):
        sys.exit("FAIL: records.py rejects a ticked task with a passed re-review")
    failed, last = [], last_rounds(tracked)
    for record in RECORDS:
        with open(os.path.join(ROOT, record), encoding="utf-8") as f:
            text = f.read()
        failed += [f"{record}: {fault}" for fault in faults(text, tracked) + unpassed(text, last)]
    if failed:
        sys.exit("FAIL:\n" + "\n".join(failed))
    print(f"PASS: {len(RECORDS)} records cite tracked files, or mark them removed or local")


main()
