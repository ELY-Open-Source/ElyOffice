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
REF = re.compile(r"(?<![\w/.-])`?((?:\.work|\.gitea|\.github|scripts|apps|crates|docs|packaging"
                 r"|tests/(?:scripts|perf|engines|fixtures))/[A-Za-z0-9_.{}*/-]*[A-Za-z0-9_])`?")


def faults(text, tracked):
    found = []
    for m in REF.finditer(text):
        ref, after = m.group(1), text[m.end():m.end() + len(LOCAL)]
        if ref.startswith(".work/"):
            if not after.startswith(LOCAL):
                found.append(f"{ref} is local evidence, unmarked")
        elif any(c in ref for c in "{}*") or after.startswith(GONE):
            continue
        # A name cut at a comma prefixes a real file.
        elif not any(path.startswith(ref) for path in tracked):
            found.append(f"{ref} is not in the repository")
    return found


def main():
    tracked = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.splitlines()
    # Each must be caught: an unmarked gap, unmarked local evidence.
    for bad in ["see `scripts/no-such.sh`", "log in .work/R1/run.log"]:
        if not faults(bad, tracked):
            sys.exit(f"FAIL: records.py passes {bad!r}")
    if faults("`scripts/check.sh`, `scripts/gone.sh`（已移除）, .work/a.log（本机，未入库）", tracked):
        sys.exit("FAIL: records.py rejects present or marked references")
    failed = []
    for record in RECORDS:
        with open(os.path.join(ROOT, record), encoding="utf-8") as f:
            failed += [f"{record}: {fault}" for fault in faults(f.read(), tracked)]
    if failed:
        sys.exit("FAIL:\n" + "\n".join(failed))
    print(f"PASS: {len(RECORDS)} records cite tracked files, or mark them removed or local")


main()
