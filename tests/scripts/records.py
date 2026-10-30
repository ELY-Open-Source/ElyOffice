#!/usr/bin/env python3
"""Records cite files that exist, or say they do not."""
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
        # Globs, braces and r1..r3 ranges stand for several files.
        elif any(c in ref for c in "{}*") or ".." in ref or after.startswith(GONE):
            continue
        elif ref not in tracked and not any(path.startswith(ref.rstrip("/") + "/") for path in tracked):
            found.append(f"{ref} is not in the repository")
    return found


def main():
    tracked = set(subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.splitlines())
    # Each must be caught: gaps, truncated names, unmarked local evidence.
    bad = ["see `scripts/no-such.sh`", "`scripts/check.s`", "`tasks/reviews/no-such-r1.md`",
           "log in .work/R1/run.log", "`.work/review/`"]
    for text in bad:
        if not faults(text, tracked):
            sys.exit(f"FAIL: records.py passes {text!r}")
    if not faults("`tasks/R4-acceptance.md`", tracked - {"tasks/R4-acceptance.md"}):
        sys.exit("FAIL: records.py passes a removed task file")
    good = "`scripts/check.sh`, `scripts/`, `scripts/gone.sh`（已移除）, .work/a.log（本机，未入库）, `.work/review/`（本机，未入库）"
    if faults(good, tracked):
        sys.exit(f"FAIL: records.py rejects present or marked references: {faults(good, tracked)}")
    failed = []
    for record in RECORDS:
        with open(os.path.join(ROOT, record), encoding="utf-8") as f:
            failed += [f"{record}: {fault}" for fault in faults(f.read(), tracked)]
    if failed:
        sys.exit("FAIL:\n" + "\n".join(failed))
    print(f"PASS: {len(RECORDS)} records cite tracked files, or mark them removed or local")


main()
