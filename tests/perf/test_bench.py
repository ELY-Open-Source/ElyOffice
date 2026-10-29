#!/usr/bin/env python3
# The bench's verdicts on made-up logs and handles.
import collections
import os
import re
import tempfile
import unittest

import bench
import drive

MAC = drive.Mac.system


class Verdicts(unittest.TestCase):
    def test_a_failed_draw_voids_the_run(self):
        for line in ["lok: tile paint failed: atlas full", "lok: tile (0, 1) failed: kernel",
                     "pdf: drawing a page failed (Other)"]:
            self.assertTrue(re.search(bench.PAINT_FAILED, line), line)
        self.assertFalse(re.search(bench.PAINT_FAILED, "lok: first tile drawn"))

    def test_a_leak_on_the_first_close_fails(self):
        before = collections.Counter({("REG", "/tmp/a"): 1})
        now = before + collections.Counter({("CHR", "/dev/null"): 1})
        with self.assertRaisesRegex(RuntimeError, "leaves handles"):
            bench.handles_kept(before, now, None, 1, MAC)

    def test_system_resources_once_then_steady(self):
        before = collections.Counter({("REG", "/tmp/a"): 1})
        font = collections.Counter({("REG", "/System/Library/Fonts/SFNSItalic.ttf"): 1})
        extra = bench.handles_kept(before, before + font, None, 1, MAC)
        self.assertEqual(bench.handles_kept(before, before + font, extra, 2, MAC), font)
        more = before + font + collections.Counter({("REG", "/System/Library/Fonts/More.ttf"): 1})
        with self.assertRaisesRegex(RuntimeError, "changed the system handles"):
            bench.handles_kept(before, more, extra, 3, MAC)

    def test_unnamed_counts_may_jitter_but_not_climb(self):
        bench.handles_steady([1365, 1367, 1366, 1365, 1366])
        with self.assertRaisesRegex(RuntimeError, "took the handles"):
            bench.handles_steady([1365, 1371, 1377, 1383, 1389])
        with self.assertRaisesRegex(RuntimeError, "took the handles"):
            bench.handles_steady([1365, 1366, 1367, 1368, 1369])

    def test_the_document_count_must_be_logged(self):
        self.assertEqual(bench.documents_open(["x", "lok-worker: 2 documents open", "lok-worker: 1 documents open"]), 1)
        with self.assertRaisesRegex(RuntimeError, "no document count"):
            bench.documents_open(["office: closed Untitled 2"])

    def test_a_cycle_needs_its_own_count(self):
        log = ["lok-worker: 1 documents open", "ely-doc: opened Untitled 2 as tab 1", "office: closed Untitled 2"]
        cursor = 1
        with self.assertRaisesRegex(RuntimeError, "no document count"):
            bench.documents_open(log[cursor:])

    def test_ElyMark_memory_is_held_to_its_budget(self):
        env = {"date": "2000-01-01", "machine": "Test"}
        rows = [("PERF-07", "ElyMark at the budget", [100], 400.0),
                ("PERF-07", "ElyMark past it", [100], 401.0)]
        with tempfile.TemporaryDirectory() as out:
            bench.write(out, env, rows, None, 1)
            with open(os.path.join(out, "2000-01-01-Test.md"), encoding="utf-8") as record:
                text = record.read()
        self.assertIn("| ElyMark at the budget | 1 | 100 | 100 ✓ | 2000 | 400 ✓ | 400 |", text)
        self.assertIn("| ElyMark past it | 1 | 100 | 100 ✓ | 2000 | 401 ✗ | 400 |", text)


if __name__ == "__main__":
    unittest.main()
