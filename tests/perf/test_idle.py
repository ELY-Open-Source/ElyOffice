#!/usr/bin/env python3
# The idle gate's verdicts on made-up engines.
import json
import os
import sys
import tempfile
import unittest
import unittest.mock

import idle

ROOT = "/data"


def engine(app, used, tasks=frozenset({"1"}), pid=None, mb=100):
    line = f"worker x {os.path.join(ROOT, app, 'lok-profile')} lok-1 --idle idle-{app}-0123456789abcdef"
    return (pid or idle.APPS.index(app) + 10, line, used, mb, tasks)


class Gate(unittest.TestCase):
    def run_gate(self, before, after, platform="linux"):
        """Exit message, or None when the gate passes."""
        record = tempfile.NamedTemporaryFile("w", suffix=".json", delete=False)
        json.dump({platform: {app: 100 for app in idle.APPS}, "win32-cycles": {app: 10 for app in idle.APPS}}, record)
        record.close()
        reads = iter([before, before, after])
        argv = ["idle.py", ROOT, "--seconds", "0", "--settle", "0"]
        with unittest.mock.patch.object(idle, "RECORD", record.name), \
                unittest.mock.patch.dict(idle.__dict__, {"linux": lambda: next(reads), "windows": lambda: next(reads)}), \
                unittest.mock.patch.object(sys, "platform", platform), \
                unittest.mock.patch.object(sys, "argv", argv):
            try:
                idle.main()
            except SystemExit as stop:
                return str(stop)
            finally:
                os.unlink(record.name)
        return None

    def quiet(self, used=5):
        return [engine(app, used) for app in idle.APPS]

    def test_still_engines_pass(self):
        self.assertIsNone(self.run_gate(self.quiet(), self.quiet()))

    def test_spent_cpu_fails(self):
        after = self.quiet()
        after[1] = engine("ely-table", 9)
        self.assertIn("ely-table used 4 ns CPU idle", self.run_gate(self.quiet(), after))

    def test_windows_cycles_within_twice_the_record_pass(self):
        after = [engine(app, 25, tasks=None) for app in idle.APPS]
        self.assertIsNone(self.run_gate([engine(app, 5, tasks=None) for app in idle.APPS], after, "win32"))

    def test_windows_cycles_past_twice_the_record_fail(self):
        before = [engine(app, 5, tasks=None) for app in idle.APPS]
        after = list(before)
        after[1] = engine("ely-table", 26, tasks=None)
        said = self.run_gate(before, after, "win32")
        self.assertIn("ely-table used 21 cycles CPU idle; 20 allowed", said)
        self.assertNotIn("ely-doc", said)

    def test_an_exited_thread_fails_though_the_sum_fell(self):
        before = self.quiet()
        before[2] = engine("ely-show", 900, frozenset({"1", "2"}))
        after = self.quiet()
        after[2] = engine("ely-show", 5)
        self.assertIn("ely-show's threads changed", self.run_gate(before, after))

    def test_a_counter_going_back_fails(self):
        after = self.quiet(used=4)
        self.assertIn("CPU counter went back", self.run_gate(self.quiet(), after))

    def test_more_memory_than_the_record_allows_fails(self):
        after = self.quiet()
        after[0] = engine("ely-doc", 5, mb=111)
        self.assertIn("ely-doc holds 111 MB", self.run_gate(self.quiet(), after))



class Ours(unittest.TestCase):
    WORKER = b"worker --idle idle-ely-doc-0123456789abcdef"

    def test_a_worker_line_reads_strictly(self):
        self.assertEqual(idle.ours(self.WORKER + " Größe".encode(), "/proc/1/cmdline"), self.WORKER.decode() + " Größe")

    def test_a_bad_byte_in_a_worker_line_fails_naming_where(self):
        with self.assertRaises(RuntimeError) as said:
            idle.ours(self.WORKER + b" \x93", "/proc/7/cmdline")
        self.assertIn("/proc/7/cmdline: an idle worker's command line is not UTF-8", str(said.exception))

    def test_another_process_is_never_decoded(self):
        self.assertIsNone(idle.ours(b"someone else \x93\xff", "/proc/8/cmdline"))


if __name__ == "__main__":
    unittest.main()
