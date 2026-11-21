#!/usr/bin/env python3
# Counted rows keyed without their count.
import os
import tempfile
import unittest

import baseline

HEAD = "| PERF | Case | Runs | P50 ms | P95 ms | Target ms | Resident MB | Budget MB |\n| --- | --- | --- | --- | --- | --- | --- | --- |\n"


def record(folder, day, rows, told=""):
    path = os.path.join(folder, f"2000-01-0{day}-Box-Windows.md")
    table = "".join(f"| {perf} | {case} | {runs} | {p50} | {p95} | 100 |  |  |\n" for perf, case, runs, p50, p95 in rows)
    with open(path, "w", encoding="utf-8") as out:
        out.write(f"# Benchmark 2000-01-0{day}\n\n```json\n{{\"machine\": \"Box-Windows\", \"version\": \"9.{day}\"}}\n```\n\n"
                  + HEAD + table + told)
    return baseline.read(path)


def busy(count, p50, p95):
    return ("PERF-02", f"ElyMail main thread busy >100 ms: {count} times", max(count, 1), p50, p95)


KEY = "PERF-02 ElyMail main thread busy >100 ms"


class Counted(unittest.TestCase):
    def test_a_count_row_is_keyed_without_its_count(self):
        with tempfile.TemporaryDirectory() as folder:
            got = record(folder, 1, [busy(6, 398, 511), ("PERF-02", "ElyMail key to frame", 30, 20, 28)])
        self.assertEqual(got["counts"], {KEY: 6})
        self.assertEqual(got["rows"], {"PERF-02 ElyMail key to frame": (20, 28)})

    def test_a_falling_or_equal_count_passes_whatever_the_stall_times(self):
        with tempfile.TemporaryDirectory() as folder:
            old = record(folder, 1, [busy(6, 398, 511)])
            for count in (2, 6, 0):
                # Longer stalls, fewer of them: the count is judged.
                self.assertEqual(baseline.slower(old, record(folder, 2, [busy(count, 597, 900)])), [], count)

    def test_a_rising_or_missing_count_wants_a_reason(self):
        with tempfile.TemporaryDirectory() as folder:
            old = record(folder, 1, [busy(2, 300, 400)])
            self.assertEqual(baseline.slower(old, record(folder, 2, [busy(3, 300, 400)])), [(KEY, 2, 3)])
            self.assertEqual(baseline.slower(old, record(folder, 2, [])), [(KEY, 2, "none")])
            self.assertEqual(baseline.lines([(KEY, 2, 3)]), [f"- {KEY}: 2 → 3 times — "])

    def test_a_rise_with_its_reason_is_explained(self):
        with tempfile.TemporaryDirectory() as folder:
            record(folder, 1, [busy(2, 300, 400)])
            told = f"\n{baseline.HEAD}\n\n- {KEY}: 2 → 3 times — a sync ran during typing.\n"
            new = record(folder, 2, [busy(3, 300, 400)], told)
            self.assertEqual(baseline.unexplained(baseline.records(folder), new), [])


if __name__ == "__main__":
    unittest.main()
