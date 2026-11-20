#!/usr/bin/env python3
"""Bench cases timed while the machine paged, judged per window."""
import argparse
import datetime
import json
import re
import sys

WINDOW = re.compile(r"^bench: case (.+) from (\S+) to (\S+)$")
# Mean KiB/s of swap-in a case may see; swap-out, none.
SWAP_IN = 100
SECOND = datetime.timedelta(seconds=1)


def windows(lines):
    """(case, start, end) from bench's output."""
    found = []
    for line in lines:
        hit = WINDOW.match(line.strip())
        if hit:
            key, start, end = hit.groups()
            found.append((key, datetime.datetime.fromisoformat(start), datetime.datetime.fromisoformat(end)))
    return found


def samples(lines):
    """(stamp, si, so) per second; the since-boot first row dropped."""
    head = next((line.split() for line in lines if line.split()[:2] == ["r", "b"]), None)
    if head is None or head[-1] != "UTC":
        raise ValueError("not a trace of TZ=UTC vmstat -n -t 1")
    si, so = head.index("si"), head.index("so")
    rows = [line.split() for line in lines if line.split() and line.split()[0].isdigit()]
    return [(datetime.datetime.fromisoformat(f"{row[-2]}T{row[-1]}+00:00"), int(row[si]), int(row[so]))
            for row in rows[1:]]


def verdicts(cases, taken, swap_in=True):
    """(case, counts, note) per window; `swap_in` false: Windows, unmeasured."""
    if not taken:
        raise ValueError("the trace has no samples")
    first, last = taken[0][0], taken[-1][0]
    judged = []
    for key, start, end in cases:
        if start < first or end > last:
            raise ValueError(f"the trace ({first:%H:%M:%S} to {last:%H:%M:%S}) does not cover {key}")
        # Stamps are truncated: T is printed somewhere in [T, T+1).
        inside = [(si, so) for stamp, si, so in taken if start - SECOND < stamp < end + SECOND]
        out = [so for _, so in inside if so > 0]
        if swap_in and any(si is None for si, _ in inside):
            raise ValueError(f"swap-in is unmeasured in {key} on a system that measures it")
        mean = sum(si for si, _ in inside) / len(inside) if swap_in else None
        counts = not out and (mean is None or mean < SWAP_IN)
        said = "si not measured" if mean is None else f"mean si {mean:.1f}"
        judged.append((key, counts, f"{len(inside)} samples, so>0 in {len(out)} (max {max(out, default=0)}), "
                                    f"{said}; {start:%H:%M:%S} to {end:%H:%M:%S}"))
    return judged


def recorded(path, void):
    """Writes `void` into a bench record's JSON."""
    with open(path, encoding="utf-8") as record:
        text = record.read()
    block = re.search(r"```json\n(.*?)\n```", text, re.S)
    if not block:
        raise ValueError(f"{path} has no JSON block")
    env = {**json.loads(block.group(1)), "void": void}
    with open(path, "w", encoding="utf-8", newline="\n") as record:
        record.write(text[:block.start(1)] + json.dumps(env, indent=2) + text[block.end(1):])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("runs", nargs="+", metavar="BENCH VMSTAT", help="pairs, oldest first; a later case wins")
    parser.add_argument("--record", help="the bench record to write `void` into")
    args = parser.parse_args()
    if len(args.runs) % 2:
        parser.error("logs come in pairs: bench output, then its vmstat trace")
    last = {}
    try:
        for bench, trace in zip(args.runs[::2], args.runs[1::2]):
            with open(bench, encoding="utf-8") as out, open(trace, encoding="utf-8") as stat:
                cases, taken = windows(out.read().splitlines()), samples(stat.read().splitlines())
            if not cases:
                sys.exit(f"judge: no case windows in {bench}")
            for key, counts, note in verdicts(cases, taken):
                last[key] = (counts, note)
                print(f"{'ok  ' if counts else 'VOID'} {key}: {note} ({bench})")
        void = [key for key, (counts, _) in last.items() if not counts]
        if args.record:
            recorded(args.record, void)
    except ValueError as error:
        sys.exit(f"judge: {error}")
    for key in void:
        print(f"void: {key}")


if __name__ == "__main__":
    main()
