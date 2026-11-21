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
        self.assertEqual(got["rows"], {KEY: (398, 511), "PERF-02 ElyMail key to frame": (20, 28)})

    def test_a_falling_or_equal_count_of_no_longer_stalls_passes(self):
        with tempfile.TemporaryDirectory() as folder:
            old = record(folder, 1, [busy(6, 398, 511)])
            for count, p50, p95 in ((2, 398, 511), (6, 500, 600), (0, 0, 0)):
                self.assertEqual(baseline.slower(old, record(folder, 2, [busy(count, p50, p95)])), [], count)

    def test_longer_stalls_at_a_lower_count_want_a_reason(self):
        with tempfile.TemporaryDirectory() as folder:
            old = record(folder, 1, [busy(6, 398, 511)])
            found = baseline.slower(old, record(folder, 2, [busy(2, 597, 597)]))
        self.assertEqual(found, [(KEY, (398, 511), (597, 597))])
        self.assertEqual(baseline.lines(found), [f"- {KEY}: P50 398 → 597, P95 511 → 597 ms — "])

    def test_a_rising_or_missing_count_wants_a_reason(self):
        with tempfile.TemporaryDirectory() as folder:
            old = record(folder, 1, [busy(2, 300, 400)])
            self.assertEqual(baseline.slower(old, record(folder, 2, [busy(3, 300, 400)])), [(KEY, 2, 3)])
            gone = baseline.slower(old, record(folder, 2, []))
            self.assertEqual(gone, [(KEY, 2, "none")])
            self.assertEqual(baseline.lines(gone), [f"- {KEY}: 2 → none times — "])
            self.assertEqual(baseline.lines([(KEY, 2, 3)]), [f"- {KEY}: 2 → 3 times — "])

    def test_a_case_is_explained_once_each_of_its_lines_has_a_reason(self):
        with tempfile.TemporaryDirectory() as folder:
            record(folder, 1, [busy(2, 300, 400)])
            for first, second, left in (("a sync ran.", "", [KEY]), ("", "a sync ran.", [KEY]), ("a sync ran.", "it ran long.", [])):
                told = (f"\n{baseline.HEAD}\n\n- {KEY}: P50 300 → 600, P95 400 → 800 ms — {first}\n"
                        f"- {KEY}: 2 → 3 times — {second}\n")
                new = record(folder, 2, [busy(3, 600, 800)], told)
                self.assertEqual(baseline.unexplained(baseline.records(folder), new), left, (first, second))

    def test_a_gone_count_with_its_reason_is_explained(self):
        with tempfile.TemporaryDirectory() as folder:
            record(folder, 1, [busy(2, 300, 400)])
            for why, left in (("", [KEY]), ("ElyMail was not timed.", [])):
                told = f"\n{baseline.HEAD}\n\n- {KEY}: 2 → none times — {why}\n"
                new = record(folder, 2, [], told)
                self.assertEqual(baseline.unexplained(baseline.records(folder), new), left, why)

    def test_a_rise_with_its_reason_is_explained(self):
        with tempfile.TemporaryDirectory() as folder:
            record(folder, 1, [busy(2, 300, 400)])
            told = f"\n{baseline.HEAD}\n\n- {KEY}: 2 → 3 times — a sync ran during typing.\n"
            new = record(folder, 2, [busy(3, 300, 400)], told)
            self.assertEqual(baseline.unexplained(baseline.records(folder), new), [])


if __name__ == "__main__":
    unittest.main()
