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
# Each record a release needs, and what writes it.
KINDS = {None: "tests/perf/bench.py", "install": "tests/perf/firsts.py install",
         "reboot": "tests/perf/firsts.py reboot"}
# After reboot: Windows only (the user, 2026-11-20).
ONLY_ON = {"reboot": {"windows"}}
# The user, 2026-11-20: these keep 0.1.16's bench record.
EARLIER_BENCH = {("macos", "0.1.17"): "0.1.16", ("linux", "0.1.17"): "0.1.16"}
# The user, 2026-11-20: Linux 0.1.17 ships with no install record.
NO_INSTALL = {("linux", "0.1.17")}
# Stalls and starved timers: their count and times.
COUNTED = re.compile(r"^(.+): (\d+) times$")


def read(path):
    """A record: header, each case's P50 and P95, counts."""
    with open(path, encoding="utf-8") as record:
        text = record.read()
    if not text.startswith("# Benchmark "):
        return None
    env = json.loads(re.search(r"```json\n(.*?)\n```", text, re.S).group(1))
    rows, counts = {}, {}
    for line in text.split("\n"):
        cells = [cell.strip() for cell in line.split("|")]
        if len(cells) > 6 and cells[1].startswith("PERF-") and cells[4].isdigit():
            counted = COUNTED.match(cells[2])
            # The count names the row; the key leaves it out.
            case = cells[1] + " " + (counted.group(1) if counted else cells[2])
            rows[case] = (int(cells[4]), int(cells[5].split()[0]))
            if counted:
                counts[case] = int(counted.group(2))
    told = text.split(HEAD, 1)[1] if HEAD in text else ""
    reasons = {}
    for case, why in re.findall(r"^- (.+?): (?:P50 .*? ms|\S+ → \S+ times) — *(.*)$", told, re.M):
        # Each of a case's lines wants its reason.
        reasons[case] = why.strip() if reasons.get(case, why).strip() else ""
    return {"path": os.path.abspath(path), "env": env, "rows": rows, "counts": counts, "reasons": reasons}


def platform(machine):
    return "windows" if machine.endswith("-Windows") else "linux" if machine.endswith("-Linux") else "macos"


def records(folder):
    """Whole benchmark records, oldest first."""
    found = [read(path) for path in sorted(glob.glob(os.path.join(folder, "*.md")))]
    return [record for record in found if record and not record["env"].get("partial")]


def slower(old, new):
    """Cases `new` lacks or ran slower; counts that rose."""
    found = []
    for case, was in old["rows"].items():
        now = new["rows"].get(case)
        # A counted row gone is told by its count.
        if now is None and case in old["counts"]:
            continue
        if now is None or any(ms > then * SLOWER and ms - then >= FLOOR_MS for then, ms in zip(was, now)):
            found.append((case, was, now or ("none", "none")))
    for case, was in old["counts"].items():
        now = new["counts"].get(case)
        if now is None or now > was:
            found.append((case, was, "none" if now is None else now))
    return found


def before(all_records, record):
    """The same machine's record before `record`, if any."""
    same = [other for other in all_records
            if (other["env"]["machine"], other["env"].get("kind")) == (record["env"]["machine"], record["env"].get("kind"))]
    at = [other["path"] for other in same].index(record["path"])
    return same[at - 1] if at else None


def lines(found):
    return [f"- {case}: {was} → {now} times — " if not isinstance(was, tuple)
            else f"- {case}: P50 {was[0]} → {now[0]}, P95 {was[1]} → {now[1]} ms — " for case, was, now in found]


def unexplained(all_records, record):
    old = before(all_records, record)
    found = dict.fromkeys(case for case, _, _ in (slower(old, record) if old else []))
    return [case for case in found if not record["reasons"].get(case, "").strip()]


def earlier(all_records, system, version):
    """This system's full bench records of exactly `version`."""
    return [record for record in all_records if platform(record["env"]["machine"]) == system
            and record["env"].get("kind") is None and record["env"].get("version") == version]


def check(folder, version, system):
    """Why this version may not ship from `system`, if so."""
    all_records = records(folder)
    for kind, tool in KINDS.items():
        if system not in ONLY_ON.get(kind, {system}):
            continue
        mine = [record for record in all_records if platform(record["env"]["machine"]) == system
                and record["env"].get("version") == version and record["env"].get("kind") == kind]
        if not mine and kind is None and (system, version) in EARLIER_BENCH:
            mine = earlier(all_records, system, EARLIER_BENCH[(system, version)])
            if mine:
                print(f"baseline: {system} {version} has no bench record of its own and uses "
                      f"{os.path.basename(mine[-1]['path'])} ({mine[-1]['env']['version']}), as the user decided on 2026-11-20")
        if not mine and kind == "install" and (system, version) in NO_INSTALL:
            print(f"baseline: {system} {version} ships with no install record, as the user decided on 2026-11-20")
            continue
        if not mine:
            return f"no {kind or 'benchmark'} record of {version} on {system} in {folder}: run {tool} there and commit it"
        if mine[-1]["env"].get("void"):
            return f"{mine[-1]['path']}: timed while the machine paged, redo with bench.py --redo: {mine[-1]['env']['void']}"
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
