"""A benchmark record: the table, PERF-08 counts, slower cases."""

import json
import os

import baseline
from run import APPS, percentile

TARGETS = {"PERF-01": 3000, "PERF-02": 100, "PERF-03": 5000, "PERF-04": 5000,
           "PERF-05": 3000, "PERF-06": 1000, "PERF-07": 2000}
# Total memory budgets, MB, at the opening mark (H-02).
BUDGETS = {"PERF-07": 400}


def write(out, env, rows, held, runs):
    os.makedirs(out, exist_ok=True)
    kind = f"-{env['kind']}" if env.get("kind") else ""
    path = os.path.join(out, f"{env['date']}-{env['machine']}{kind}{'-partial' if env.get('partial') else ''}.md")
    lines = [f"# Benchmark {env['date']}", "", "Format: docs/perf/README.md.", "", "```json",
             json.dumps(env, indent=2), "```", "",
             "| PERF | Case | Runs | P50 ms | P95 ms | Target ms | Resident MB | Budget MB |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for perf, case, times, memory in rows:
        target = TARGETS.get(perf)
        p95 = percentile(times, 0.95)
        # Counts, not times: stalls and starved timers.
        counted = "busy" in case or "starved" in case
        verdict = "" if target is None or counted else (" ✓" if p95 <= target else " ✗")
        budget = BUDGETS.get(perf) if memory is not None else None
        within = "" if budget is None else (" ✓" if memory <= budget else " ✗")
        mem = "" if memory is None else f"{memory:.0f}{within}"
        lines.append(f"| {perf} | {case} | {len(times)} | {percentile(times, 0.5):.0f} | {p95:.0f}{verdict} | "
                     f"{target or ''} | {mem} | {budget or ''} |")
    for slug, (memory, handles, live), seen in held:
        lines += ["", f"PERF-08, {APPS[slug]} opening and closing a document {len(seen)} times "
                  "(private memory MB, swapped included / handles / live documents):", "",
                  f"before: {memory:.0f}/{handles}/{','.join(map(str, live))}", "",
                  " ".join(f"{memory:.0f}/{handles}/{','.join(map(str, live))}" for memory, handles, live in seen)]
    lines += ["", "PERF-09 (Agent sessions) belongs to the second release.", ""]
    with open(path, "w", encoding="utf-8", newline="\n") as record:
        record.write("\n".join(lines))
    # Each wants its reason before a release (baseline.py).
    record = baseline.read(path)
    last = None if env.get("partial") else baseline.before(baseline.records(out), record)
    found = baseline.slower(last, record) if last else []
    if found:
        with open(path, "a", encoding="utf-8", newline="\n") as record:
            record.write("\n".join([baseline.HEAD, "", *baseline.lines(found), ""]))
    print(path)
    return path
