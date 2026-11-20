#!/usr/bin/env python3
# Windows process trees, and stop() never ending the bench.
import ctypes
import os
import subprocess
import sys
import types
import unittest
import unittest.mock

import lineage
import run


def tree(born):
    return lambda pid: born.get(pid)


class Descendants(unittest.TestCase):
    def test_a_parent_id_reused_by_the_app_takes_in_nothing_older(self):
        # wininit's dead parent pid 912, reused by the app.
        pairs = [(388, 912), (1072, 388), (1952, 1072), (11900, 912), (4536, 912), (12508, 4536)]
        born = tree({912: 500, 388: 1, 1072: 2, 1952: 3, 11900: 501, 4536: 502, 12508: 503})
        self.assertEqual(lineage.descendants(912, pairs, born), {912, 11900, 4536, 12508})

    def test_a_child_gone_or_unreadable_is_left_out(self):
        pairs = [(20, 10), (30, 20)]
        self.assertEqual(lineage.descendants(10, pairs, tree({10: 5, 30: 9})), {10})

    def test_a_child_born_in_the_parents_tick_is_ours(self):
        self.assertEqual(lineage.descendants(10, [(20, 10)], tree({10: 5, 20: 5})), {10, 20})


@unittest.skipUnless(os.name == "nt", "process start times are read on Windows")
class Created(unittest.TestCase):
    def test_a_child_starts_after_the_bench_and_no_process_reads_none(self):
        k32 = run.drive.Windows.k32
        child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(5)"])
        try:
            self.assertGreaterEqual(lineage.created(k32, child.pid), lineage.created(k32, os.getpid()))
        finally:
            child.kill()
            child.wait()
        # Windows pids are multiples of four.
        self.assertIsNone(lineage.created(k32, 3))


class Stopping(unittest.TestCase):
    def app(self, family, gone=()):
        app = object.__new__(run.App)
        app.slug, app.lines = "ely-doc", lambda: []
        app.process = types.SimpleNamespace(pid=912, wait=lambda: 0)
        names = {912: "ely-doc.exe", 11900: "ely-lok-worker.exe", 388: "wininit.exe", os.getpid(): "python.exe"}
        host = types.SimpleNamespace(stopped=lambda lines: None, pids=lambda pid: family, image=names.get,
                                     gone=lambda pid: pid in gone)
        return app, host

    def test_a_tree_holding_the_bench_kills_nothing_and_says_so(self):
        for bench in [os.getpid(), os.getppid()]:
            app, host = self.app({912, 11900, bench})
            with unittest.mock.patch.object(run, "HOST", host), unittest.mock.patch.object(run.os, "kill") as kill:
                with self.assertRaisesRegex(RuntimeError, rf"holds the bench or its parent: {bench} "):
                    app.stop()
            kill.assert_not_called()

    def test_the_apps_own_tree_is_killed(self):
        app, host = self.app({912, 11900})
        with unittest.mock.patch.object(run, "HOST", host), unittest.mock.patch.object(run.os, "kill") as kill:
            app.stop()
        self.assertEqual(sorted(call.args for call in kill.call_args_list), [(912, 9), (11900, 9)])

    def test_a_refused_kill_names_the_process_after_ending_the_rest(self):
        app, host = self.app({912, 11900, 388})
        refuse = lambda pid, signal: (_ for _ in ()).throw(PermissionError(13, "denied")) if pid == 388 else None
        with unittest.mock.patch.object(run, "HOST", host), unittest.mock.patch.object(run.os, "kill", side_effect=refuse) as kill:
            with self.assertRaisesRegex(RuntimeError, r"refused for 388 \(wininit.exe\)$"):
                app.stop()
        self.assertEqual(sorted(call.args[0] for call in kill.call_args_list), [388, 912, 11900])

    def test_a_refusal_from_a_process_mid_exit_passes_a_live_one_raises(self):
        # ElyMark's git, ending as the app closes (R20.95).
        refuse = lambda pid, signal: (_ for _ in ()).throw(PermissionError(13, "denied")) if pid == 11900 else None
        for gone, fails in [({11900}, False), (set(), True)]:
            app, host = self.app({912, 11900}, gone)
            with unittest.mock.patch.object(run, "HOST", host), unittest.mock.patch.object(run.os, "kill", side_effect=refuse):
                if fails:
                    with self.assertRaisesRegex(RuntimeError, r"refused for 11900 \(ely-lok-worker.exe\)$"):
                        app.stop()
                else:
                    app.stop()

    def test_a_process_that_already_ended_passes_and_other_errors_raise(self):
        ended = OSError(22, "gone")
        ended.winerror = 87
        other = OSError(22, "odd")
        other.winerror = 6
        for error, fails in [(ProcessLookupError(3, "gone"), False), (ended, False), (other, True)]:
            app, host = self.app({912, 11900})
            kill = lambda pid, signal, error=error: (_ for _ in ()).throw(error) if pid == 11900 else None
            with unittest.mock.patch.object(run, "HOST", host), unittest.mock.patch.object(run.os, "kill", side_effect=kill):
                if fails:
                    with self.assertRaises(OSError):
                        app.stop()
                else:
                    app.stop()


class Process:
    """kernel32 for one process: open error, wait result, exit code."""

    def __init__(self, refused=None, waited=258, code=259, read=True):
        self.waited, self.code, self.read = waited, code, read
        self.CloseHandle = lambda handle: 1

        # A function, so `restype` can be set on it.
        def opened(access, inherit, pid):
            if refused is None:
                return 7
            ctypes.set_last_error(refused)
            return None

        self.OpenProcess = opened

    def WaitForSingleObject(self, handle, ms):
        return self.waited

    def GetExitCodeProcess(self, handle, code):
        code._obj.value = self.code
        if not self.read:
            ctypes.set_last_error(6)
        return self.read


@unittest.skipUnless(os.name == "nt", "the last error is Windows only")
class Gone(unittest.TestCase):
    def test_an_ended_or_ending_process_is_gone_a_running_one_is_not(self):
        self.assertTrue(lineage.gone(Process(87), 1))
        self.assertTrue(lineage.gone(Process(None, waited=0), 1))
        self.assertTrue(lineage.gone(Process(None, code=0), 1), "exit code set, not yet signalled")
        self.assertFalse(lineage.gone(Process(None), 1))

    def test_an_unexpected_failure_raises(self):
        for kernel in [Process(5), Process(None, read=False)]:
            with self.assertRaises(RuntimeError):
                lineage.gone(kernel, 1)


if __name__ == "__main__":
    unittest.main()
