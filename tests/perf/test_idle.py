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
                unittest.mock.patch.dict(idle.__dict__, {"linux": lambda root: next(reads), "windows": lambda root: next(reads)}), \
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
    WORKER = engine("ely-doc", 0)[1].encode()

    def test_a_worker_line_reads_strictly(self):
        self.assertEqual(idle.ours(self.WORKER + " Größe".encode(), ROOT, "/proc/1/cmdline"), self.WORKER.decode() + " Größe")

    def test_a_bad_byte_in_a_worker_line_fails_naming_where(self):
        with self.assertRaises(RuntimeError) as said:
            idle.ours(self.WORKER + b" \x93", ROOT, "/proc/7/cmdline")
        self.assertIn("/proc/7/cmdline: an idle worker's command line is not UTF-8", str(said.exception))

    def test_another_process_is_never_decoded(self):
        self.assertIsNone(idle.ours(b"someone else \x93\xff", ROOT, "/proc/8/cmdline"))

    def test_the_marker_alone_is_not_ours(self):
        """R20.92 review 1's probe: no profile under the root."""
        self.assertIsNone(idle.ours(b"unrelated --idle idle-ely-doc-0123456789abcdef \xff", ROOT, "probe"))

    def test_another_root_is_not_ours(self):
        other = engine("ely-doc", 0)[1].replace(ROOT, "/elsewhere").encode()
        self.assertIsNone(idle.ours(other + b" \xff", ROOT, "probe"))

    def test_a_marker_naming_no_app_of_ours_is_not_ours(self):
        line = self.WORKER.replace(b"ely-doc", b"ely-pdf")
        self.assertIsNone(idle.ours(line + b" \xff", ROOT, "probe"))

    def test_windows_keeps_only_our_workers(self):
        rows = [{"ProcessId": 10, "CommandLine": self.WORKER.decode(), "WorkingSetSize": 2**20},
                {"ProcessId": 11, "CommandLine": "unrelated --idle idle-ely-doc-0123456789abcdef", "WorkingSetSize": 2**20}]
        said = unittest.mock.Mock(stdout=json.dumps(rows))
        k32 = unittest.mock.Mock(**{"OpenProcess.return_value": 1, "QueryProcessCycleTime.return_value": 1})
        with unittest.mock.patch.object(idle.child, "run", return_value=said), \
                unittest.mock.patch.object(idle.ctypes, "WinDLL", return_value=k32, create=True):
            self.assertEqual([row[0] for row in idle.windows(ROOT)], [10])


if __name__ == "__main__":
    unittest.main()
