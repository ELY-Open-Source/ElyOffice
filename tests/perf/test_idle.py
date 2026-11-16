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
    def run_gate(self, before, after):
        """Exit message, or None when the gate passes."""
        record = tempfile.NamedTemporaryFile("w", suffix=".json", delete=False)
        json.dump({"linux": {app: 100 for app in idle.APPS}}, record)
        record.close()
        reads = iter([before, before, after])
        argv = ["idle.py", ROOT, "--seconds", "0", "--settle", "0"]
        with unittest.mock.patch.object(idle, "RECORD", record.name), \
                unittest.mock.patch.dict(idle.__dict__, {"linux": lambda: next(reads)}), \
                unittest.mock.patch.object(sys, "platform", "linux"), \
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


if __name__ == "__main__":
    unittest.main()
