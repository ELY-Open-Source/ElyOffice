#!/usr/bin/env python3
# Swap judged per case window; a void case blocks release.
import datetime
import os
import sys
import tempfile
import unittest
import unittest.mock

import baseline
import bench
import report
import judge


HEADER = ["procs -----------memory---------- ---swap-- -----io---- -system-- -------cpu------- -----timestamp-----",
          " r  b   swpd   free   buff  cache   si   so    bi    bo   in   cs us sy id wa st gu                 UTC"]


def trace(rows):
    """vmstat rows at 01:00:SS UTC; the first counts since boot."""
    return HEADER + [f" 0  0 1 1 1 1 {si} {so} 0 0 0 0 0 0 100 0 0 0 2026-11-20 01:00:{at:02d}" for at, si, so in rows]


def window(key, start, end):
    at = lambda second: datetime.datetime(2026, 11, 20, 1, 0, tzinfo=datetime.timezone.utc) + datetime.timedelta(seconds=second)
    return (key, at(start), at(end))


class Paged(unittest.TestCase):
    def test_the_since_boot_row_is_dropped_and_a_local_trace_refused(self):
        taken = judge.samples(trace([(0, 900, 900), (1, 0, 0)]))
        self.assertEqual([(stamp.second, si, so) for stamp, si, so in taken], [(1, 0, 0)])
        with self.assertRaises(ValueError):
            judge.samples([HEADER[0], HEADER[1].replace("UTC", "CET")] + trace([(0, 0, 0)])[2:])

    def test_a_window_takes_every_second_a_truncated_stamp_may_cover(self):
        def counts(at, si, so):
            rows = [(0, 0, 0)] + [(s, si if s == at else 0, so if s == at else 0) for s in range(1, 9)]
            return judge.verdicts([window("A", 2.5, 4.2)], judge.samples(trace(rows)))[0][1]
        # Stamp 2 may print after 2.5, stamp 5 before 5.2.
        self.assertTrue(counts(1, 0, 9))
        self.assertFalse(counts(2, 0, 9))
        self.assertFalse(counts(5, 0, 9))
        self.assertTrue(counts(6, 0, 9))
        # Four samples: 2, 3, 4, 5.
        self.assertFalse(counts(3, 400, 0))
        self.assertTrue(counts(3, 399, 0))

    def test_a_trace_that_misses_a_window_is_refused(self):
        taken = judge.samples(trace([(s, 0, 0) for s in range(2, 6)]))
        for start, end in [(2.5, 5.0), (3.0, 5.5)]:
            with self.assertRaisesRegex(ValueError, "does not cover"):
                judge.verdicts([window("A", start, end)], taken)
        self.assertTrue(judge.verdicts([window("A", 3.0, 5.0)], taken)[0][1])

    def test_a_later_run_s_verdict_wins_and_lands_in_the_record(self):
        line = lambda key, start, end: f"bench: case {key} from 2026-11-20T01:00:{start:02d}.000+00:00 to 2026-11-20T01:00:{end:02d}.000+00:00"
        clean = [(s, 0, 0) for s in range(10)]
        with tempfile.TemporaryDirectory() as out:
            files = {"first.log": [line("A", 2, 4), line("B", 5, 7)], "first.vmstat": trace(clean[:3] + [(3, 0, 9)] + clean[4:6] + [(6, 0, 9)] + clean[7:]),
                     "redo.log": [line("A", 2, 4)], "redo.vmstat": trace(clean)}
            for name, lines in files.items():
                with open(os.path.join(out, name), "w", encoding="utf-8") as file:
                    file.write("\n".join(lines) + "\n")
            env = {"date": "2000-01-01", "machine": "Test-Linux", "version": "9.9"}
            with unittest.mock.patch("builtins.print"):
                record = report.write(out, env, [("PERF-01", "ElyDoc first run", [100], None)], [], 1)
            logs = [os.path.join(out, name) for name in files]
            with unittest.mock.patch("builtins.print"), unittest.mock.patch.object(sys, "argv", ["judge.py", *logs, "--record", record]):
                judge.main()
            self.assertEqual(baseline.read(record)["env"]["void"], ["B"])
            self.assertEqual(baseline.read(record)["rows"], {"PERF-01 ElyDoc first run": (100, 100)})

    def test_a_gap_or_an_empty_window_is_refused_either_way(self):
        def taken(seconds, si):
            at = window("", 0, 0)[1]
            return [(at + datetime.timedelta(seconds=s), si, 0) for s in seconds]

        for si, swap_in in [(0, True), (None, False)]:
            # 03 to 05 unseen: stamps 00, 02, 10.
            with self.assertRaisesRegex(ValueError, "misses 8 s"):
                judge.verdicts([window("A", 2, 5)], taken([0, 2, 10], si), swap_in=swap_in)
            with self.assertRaisesRegex(ValueError, "no sample inside"):
                judge.verdicts([window("A", 4, 5)], taken([0, 10], si), swap_in=swap_in)
            # One period and a truncated second passes; more does not.
            judged = judge.verdicts([window("A", 2, 5)], taken([0, 1, 3, 4, 6], si), swap_in=swap_in)
            self.assertTrue(judged[0][1], judged)
            over = (judge.INTERVAL + judge.SECOND).total_seconds() + 0.001
            with self.assertRaisesRegex(ValueError, "misses"):
                judge.verdicts([window("A", 2, 5)], taken([0, 1, 1 + over, 4, 6], si), swap_in=swap_in)

    def test_a_redo_clears_only_what_it_redid(self):
        judged = [("A", True, ""), ("C", False, "")]
        self.assertEqual(bench.void_after(["A", "B"], {"A"}, judged), ["B", "C"])

    def test_the_release_check_refuses_a_record_with_void_cases(self):
        rows = [("PERF-01", "ElyDoc first run", [100], None)]
        for void, said in [(["PERF-01 ely-doc"], "paged"), ([], "no install record")]:
            with tempfile.TemporaryDirectory() as out:
                env = {"date": "2000-01-01", "machine": "Test-Linux", "version": "9.9", "void": void}
                with unittest.mock.patch("builtins.print"):
                    report.write(out, env, rows, [], 1)
                self.assertIn(said, baseline.check(out, "9.9", "linux"))

    @unittest.skipUnless(sys.platform == "linux", "vmstat is Linux only")
    def test_a_run_samples_and_judges_its_own_cases(self):
        with tempfile.TemporaryDirectory() as work:
            paging, cases = bench.Paging(os.path.join(work, "vmstat.log")), bench.Cases({}, [])
            self.addCleanup(paging.process.kill)
            with unittest.mock.patch("builtins.print"):
                cases.run("A", lambda _: [])
            judged = paging.judged(cases.windows)
        self.assertEqual([key for key, _, _ in judged], ["A"])
        self.assertIsNotNone(paging.process.poll(), "vmstat outlived its run")


if __name__ == "__main__":
    unittest.main()
