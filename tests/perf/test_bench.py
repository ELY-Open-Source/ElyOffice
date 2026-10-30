#!/usr/bin/env python3
# The bench's verdicts on made-up logs and handles.
import collections
import os
import re
import tempfile
import unittest

import bench
import cycles
import drive
import run

MAC = drive.Mac.system


class Verdicts(unittest.TestCase):
    def test_a_failed_draw_voids_the_run(self):
        for line in ["lok: tile paint failed: atlas full", "lok: tile (0, 1) failed: kernel",
                     "pdf: drawing a page failed (Other)"]:
            self.assertTrue(re.search(run.PAINT_FAILED, line), line)
        self.assertFalse(re.search(run.PAINT_FAILED, "lok: first tile drawn"))

    def test_a_leak_on_the_first_close_fails(self):
        before = collections.Counter({("REG", "/tmp/a"): 1})
        now = before + collections.Counter({("CHR", "/dev/null"): 1})
        with self.assertRaisesRegex(RuntimeError, "leaves handles"):
            cycles.handles_kept("app", before, now, None, 1, MAC)

    def test_system_resources_once_then_steady(self):
        before = collections.Counter({("REG", "/tmp/a"): 1})
        font = collections.Counter({("REG", "/System/Library/Fonts/SFNSItalic.ttf"): 1})
        extra = cycles.handles_kept("app", before, before + font, None, 1, MAC)
        self.assertEqual(cycles.handles_kept("app", before, before + font, extra, 2, MAC), font)
        more = before + font + collections.Counter({("REG", "/System/Library/Fonts/More.ttf"): 1})
        with self.assertRaisesRegex(RuntimeError, "changed the system handles"):
            cycles.handles_kept("app", before, more, extra, 3, MAC)

    def test_unnamed_counts_may_jitter_but_not_climb(self):
        # Windows, 40 cycles after the liblangtag fix.
        measured = [1428, 1428, 1428, 1430, 1430, 1430, 1431, 1431, 1431, 1432] + [1423] * 20 + [1415, 1407, 1407] + [1399] * 7
        cycles.handles_steady("app", 1423, measured)
        with self.assertRaisesRegex(RuntimeError, "took the handles"):
            cycles.handles_steady("app", 1365, [1365, 1371, 1377, 1383, 1389])
        # One handle every second cycle, 19 in all.
        with self.assertRaisesRegex(RuntimeError, "took the handles"):
            cycles.handles_steady("app", 1400, [1400 + n // 2 for n in range(40)])
        # The first cycle keeps 50 and never gives them back.
        with self.assertRaisesRegex(RuntimeError, "keep 50 handles"):
            cycles.handles_steady("app", 1400, [1450] * 40)

    def test_memory_may_wobble_but_not_climb(self):
        cycles.steady("app", "MB", [261, 261, 254, 255, 254, 261, 260, 261, 255, 256] * 2, cycles.PAGE_MB)
        # ElyPDF on Linux: thread arenas keep 55 MB, then flat.
        arenas = [
            234.6, 236.7, 240.6, 244.0, 245.4, 246.8, 248.7, 248.7, 250.6, 252.0,
            253.9, 255.9, 255.9, 257.9, 261.8, 262.5, 262.6, 266.5, 266.6, 268.6,
            268.6, 270.5, 270.5, 271.2, 271.9, 271.9, 272.6, 274.6, 274.6, 274.6,
            275.6, 275.6, 276.3,
        ] + [276.3] * 66 + [277.0]
        cycles.steady("app", "MB", arenas, cycles.PAGE_MB)
        # ElyDoc on Linux, 100 cycles: warm-up, then flat.
        settling = [
            387.0, 387.6, 388.0, 388.9, 389.1, 389.3, 389.4, 389.5, 390.0, 390.2,
            390.3, 390.4, 390.6, 390.2, 390.4, 390.6, 391.3, 391.3, 391.5, 391.5,
            391.5, 391.7, 391.7, 391.8, 391.9, 392.0, 392.0, 392.0, 392.7, 392.8,
            392.8, 392.3, 392.5, 392.5, 392.7, 392.7, 392.8, 392.8, 392.8, 392.8,
            392.8, 392.9, 392.9, 394.3, 394.6, 394.6, 395.1, 395.1, 394.9, 394.9,
            395.0, 394.7, 395.1, 394.6, 394.7, 394.7, 394.3, 394.3, 394.3, 394.3,
            394.4, 393.9, 394.1, 394.1, 394.2, 394.8, 395.0, 395.0, 395.0, 395.5,
            395.6, 395.9, 395.9, 396.1, 396.2, 396.1, 396.1, 396.1, 396.2, 396.3,
            396.3, 396.3, 396.3, 396.3, 396.3, 396.4, 396.4, 396.4, 396.3, 396.3,
            396.1, 396.1, 396.1, 395.9, 395.6, 395.6, 396.1, 395.6, 395.6, 395.1,
        ]
        cycles.steady("app", "MB", settling, cycles.PAGE_MB)
        # ElyMark on Linux, 150 cycles: under a page a cycle.
        runs = [(120, 7), (125, 4), (127, 3), (128, 3), (129, 3), (130, 1), (131, 2), (132, 2), (134, 1), (135, 7), (136, 1),
                (139, 3), (140, 3), (141, 2), (142, 2), (143, 6), (144, 4), (145, 3), (147, 2), (149, 6), (150, 3), (151, 6)]
        tail = [173 + k / 256 for k, n in runs for _ in range(n)]
        cycles.steady("app", "MB", [173.4] * 76 + tail, cycles.PAGE_MB)
        with self.assertRaisesRegex(RuntimeError, "took the MB"):
            cycles.steady("app", "MB", [373 + n for n in range(20)], cycles.PAGE_MB)
        # A tenth of a MB every cycle.
        with self.assertRaisesRegex(RuntimeError, "took the MB"):
            cycles.steady("app", "MB", [387 + n / 10 for n in range(100)], cycles.PAGE_MB)

    def test_counts_come_from_the_log(self):
        self.assertEqual(cycles.counted(["x", "lok-worker: 2 documents open", "lok-worker: 1 documents open"],
                                        r"lok-worker: (\d+) documents open"), 1)
        with self.assertRaisesRegex(RuntimeError, "no count"):
            cycles.counted(["office: closed Untitled 2, 1 tabs open"], r"lok-worker: (\d+) documents open")

    def test_an_answer_is_timed_by_its_stamp_not_its_reading(self):
        log = ["[2026-10-29T01:11:42.100Z INFO  ely_tree] tree: opened /r",
               "[2026-10-29T01:11:42.730Z DEBUG ely_tree] history shows 50 commits, searched false"]
        sent = run.logged_at(log[0])
        # However late the driver returns, the stamps decide.
        self.assertEqual(run.answered(log, "history shows", sent), 630)
        self.assertIsNone(run.answered(log, "searched true", sent))
        with self.assertRaisesRegex(RuntimeError, "before its input"):
            run.answered(log, "history shows", sent + 1000)
        with self.assertRaisesRegex(RuntimeError, "millisecond stamp"):
            run.logged_at("[2026-10-29T01:11:42Z INFO  x] no milliseconds")

    def test_a_helper_must_say_when_it_sent(self):
        self.assertEqual(drive.sent("drive: took the front back\nsent 1761700302730\n"), 1761700302730)
        with self.assertRaisesRegex(RuntimeError, "did not say"):
            drive.sent("pressed\n")

    def test_ElyMark_memory_is_held_to_its_budget(self):
        env = {"date": "2000-01-01", "machine": "Test"}
        rows = [("PERF-07", "ElyMark at the budget", [100], 400.0),
                ("PERF-07", "ElyMark past it", [100], 401.0)]
        held = [("ely-mark", (120.0, 50, (1,)), [(121.0, 50, (1,))])]
        with tempfile.TemporaryDirectory() as out:
            bench.write(out, env, rows, held, 1)
            with open(os.path.join(out, "2000-01-01-Test.md"), encoding="utf-8") as record:
                text = record.read()
        self.assertIn("| ElyMark at the budget | 1 | 100 | 100 ✓ | 2000 | 400 ✓ | 400 |", text)
        self.assertIn("| ElyMark past it | 1 | 100 | 100 ✓ | 2000 | 401 ✗ | 400 |", text)
        self.assertIn("before: 120/50/1", text)


class Children(drive.Host):
    """App 1 with child 2; `alive` says if 2 runs."""

    def __init__(self, alive):
        self.alive = alive

    def pids(self, root):
        return {1, 2} if self.alive else {1}

    def held(self, pid):
        if pid == 1:
            return collections.Counter({("fd", "x"): 3})
        raise RuntimeError("process 2: no such process")


class Ended(unittest.TestCase):
    def test_a_child_that_exited_is_left_out(self):
        self.assertEqual(Children(False).handles(1, [1, 2]), collections.Counter({("fd", "x"): 3}))

    def test_a_live_child_that_cannot_be_read_voids_the_run(self):
        with self.assertRaises(RuntimeError):
            Children(True).handles(1, [1, 2])

    def test_the_app_itself_is_never_left_out(self):
        host = Children(False)
        host.held = lambda pid: (_ for _ in ()).throw(RuntimeError("gone"))
        with self.assertRaises(RuntimeError):
            host.handles(1, [1])


if __name__ == "__main__":
    unittest.main()
