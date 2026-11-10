#!/usr/bin/env python3
"""A release's benchmark record, and what it slowed."""

import argparse
import glob
import json
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
# A third slower, and by ten milliseconds, wants a reason.
SLOWER, FLOOR_MS = 1.30, 10
HEAD = "Slower than the last record by over 30%:"


def read(path):
    """A benchmark record: its header and each case's P50, P95."""
    with open(path, encoding="utf-8") as record:
        text = record.read()
    if not text.startswith("# Benchmark "):
        return None
    env = json.loads(re.search(r"```json\n(.*?)\n```", text, re.S).group(1))
    rows = {}
    for line in text.split("\n"):
        cells = [cell.strip() for cell in line.split("|")]
        if len(cells) > 6 and cells[1].startswith("PERF-") and cells[4].isdigit():
            rows[cells[1] + " " + cells[2]] = (int(cells[4]), int(cells[5].split()[0]))
    told = text.split(HEAD, 1)[1] if HEAD in text else ""
    reasons = dict(re.findall(r"^- (.+?): P50 .*? ms — *(.*)$", told, re.M))
    return {"path": os.path.abspath(path), "env": env, "rows": rows, "reasons": reasons}


def platform(machine):
    return "windows" if machine.endswith("-Windows") else "linux" if machine.endswith("-Linux") else "macos"


def records(folder):
    """Whole benchmark records, oldest first."""
    found = [read(path) for path in sorted(glob.glob(os.path.join(folder, "*.md")))]
    return [record for record in found if record and not record["env"].get("partial")]


def slower(old, new):
    """Cases `new` lacks or ran a third slower."""
    found = []
    for case, was in old["rows"].items():
        now = new["rows"].get(case)
        if now is None or any(ms > then * SLOWER and ms - then >= FLOOR_MS for then, ms in zip(was, now)):
            found.append((case, was, now or ("none", "none")))
    return found


def before(all_records, record):
    """The same machine's record before `record`, if any."""
    same = [other for other in all_records if other["env"]["machine"] == record["env"]["machine"]]
    at = [other["path"] for other in same].index(record["path"])
    return same[at - 1] if at else None


def lines(found):
    return [f"- {case}: P50 {was[0]} → {now[0]}, P95 {was[1]} → {now[1]} ms — " for case, was, now in found]


def unexplained(all_records, record):
    old = before(all_records, record)
    return [case for case, _, _ in (slower(old, record) if old else []) if not record["reasons"].get(case, "").strip()]


def check(folder, version, system):
    """Why this version may not ship from `system`, if so."""
    all_records = records(folder)
    mine = [record for record in all_records
            if platform(record["env"]["machine"]) == system and record["env"].get("version") == version]
    if not mine:
        return f"no benchmark record of {version} on {system} in {folder}: run tests/perf/bench.py there and commit it"
    cases = unexplained(all_records, mine[-1])
    if cases:
        return f"{mine[-1]['path']}: slower by over 30% with no reason written: {'; '.join(cases)}"
    return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", nargs=2, metavar=("VERSION", "SYSTEM"))
    parser.add_argument("--folder", default=os.path.join(ROOT, "docs", "perf"))
    parser.add_argument("record", nargs="?")
    args = parser.parse_args()
    if args.check:
        why = check(args.folder, *args.check)
        sys.exit(f"baseline: {why}" if why else 0)
    record = read(args.record)
    old = before(records(os.path.dirname(os.path.abspath(args.record))), record)
    print("\n".join([HEAD, ""] + lines(slower(old, record))) if old and slower(old, record) else "nothing over 30% slower")


if __name__ == "__main__":
    main()
