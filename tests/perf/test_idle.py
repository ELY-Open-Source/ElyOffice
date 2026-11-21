#!/usr/bin/env python3
# The idle gate's verdicts on made-up engines.
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
import unittest.mock

import child
import idle

ROOT = "/data"
WORKER = "/opt/ely/ely-lok-worker" + (".exe" if sys.platform == "win32" else "")


def argv(app, root=ROOT):
    return [WORKER, "x", idle.profile(root, app), "lok-1", "--idle", f"idle-{app}-0123456789abcdef"]


def engine(app, used, tasks=frozenset({"1"}), pid=None, mb=100):
    return app, (pid or idle.APPS.index(app) + 10, used, mb, tasks)


class Gate(unittest.TestCase):
    def run_gate(self, before, after, platform="linux"):
        """Exit message, or None when the gate passes."""
        record = tempfile.NamedTemporaryFile("w", suffix=".json", delete=False)
        json.dump({platform: {app: 100 for app in idle.APPS}, "win32-cycles": {app: 10 for app in idle.APPS}}, record)
        record.close()
        reads = iter([dict(before), dict(before), dict(after)])
        argv = ["idle.py", ROOT, "--seconds", "0", "--settle", "0"]
        with unittest.mock.patch.object(idle, "RECORD", record.name), \
                unittest.mock.patch.object(idle, "engines", lambda root: next(reads)), \
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



# Holds the profile's lock as the worker does.
HOLD = "import fcntl, os, sys; f = open(os.path.join(sys.argv[2], 'idle.lock'), 'w'); " \
       "fcntl.lockf(f, fcntl.LOCK_EX | fcntl.LOCK_NB); print('held', flush=True); sys.stdin.read()"
WAIT = b"import sys; print('up', flush=True); sys.stdin.read()"
# Reviews 1 and 2's probes: our marks, a bad byte.
PROBES = [[b"unrelated", b"--idle", b"idle-ely-doc-0123456789abcdef", b"\xff"],
          [b"unrelated", b"{root}/ely-doc/lok-profile", b"--idle", b"idle-ely-doc-0123456789abcdef", b"\xff"]]


@unittest.skipIf(sys.platform == "win32", "POSIX locks; Windows is read through Restart Manager")
class Held(unittest.TestCase):
    """Real lock holders and real foreign processes on this host."""

    @classmethod
    def setUpClass(cls):
        # The stand-in is Python; macOS renames it on exec.
        process = subprocess.Popen([sys.executable, "-c", WAIT], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                   encoding="utf-8", errors="strict")
        child.line(process)
        first = idle.linux(process.pid)[0][0] if sys.platform == "linux" else idle.mac_argv(process.pid)[0]
        process.stdin.close()
        process.wait()
        cls.named = unittest.mock.patch.object(idle, "WORKER", os.path.basename(first))

    def setUp(self):
        self.named.start()
        self.addCleanup(self.named.stop)
        self.root = tempfile.mkdtemp(dir=os.environ.get("TMPDIR"))
        self.started = []
        for app in idle.APPS:
            os.makedirs(idle.profile(self.root, app))

    def tearDown(self):
        for process in self.started:
            process.stdin.close() if process.stdin else process.kill()
            process.wait()
        shutil.rmtree(self.root)

    def hold(self, app, extra=(), args=None):
        args = argv(app, self.root)[1:] if args is None else args
        process = subprocess.Popen([sys.executable, "-c", HOLD, *args, *extra], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, encoding="utf-8", errors="strict")
        self.started.append(process)
        self.assertEqual(child.line(process), "held\n")
        return process.pid

    def probes(self):
        for probe in PROBES:
            args = [part.replace(b"{root}", os.fsencode(self.root)) for part in probe]
            # Its argv carries the probe until stdin closes.
            process = subprocess.Popen([os.fsencode(sys.executable), b"-c", WAIT, *args], stdin=subprocess.PIPE,
                                       stdout=subprocess.PIPE, encoding="utf-8", errors="strict")
            self.started.append(process)
            self.assertEqual(child.line(process), "up\n")
            if sys.platform == "linux":
                with open(f"/proc/{process.pid}/cmdline", "rb") as line:
                    self.assertIn(b"\0".join(args), line.read())

    def read_only(self, allowed):
        real = idle.linux if sys.platform == "linux" else idle.mac
        name = "linux" if sys.platform == "linux" else "mac"

        def read(pid):
            if pid not in allowed:
                raise AssertionError(f"pid {pid} holds no lock and was read")
            return real(pid)
        return unittest.mock.patch.object(idle, name, read)

    def test_each_lock_holder_is_found_and_read_strictly(self):
        pids = {app: self.hold(app) for app in idle.APPS}
        self.probes()
        with self.read_only(set(pids.values())):
            found = idle.engines(self.root)
        self.assertEqual({app: found[app][0] for app in idle.APPS}, pids)

    def test_our_marks_without_the_lock_are_never_read(self):
        self.probes()
        with self.read_only(set()), self.assertRaises(RuntimeError) as said:
            idle.engines(self.root)
        self.assertEqual(f"no idle engine for ely-doc, ely-table, ely-show under {self.root}", str(said.exception))

    def test_no_holder_and_no_file_are_both_none(self):
        lock = os.path.join(idle.profile(self.root, "ely-doc"), idle.LOCK)
        self.assertEqual(idle.posix_holders(lock), [])
        open(lock, "w").close()
        self.assertEqual(idle.posix_holders(lock), [])

    def test_a_failing_reader_raises_never_none(self):
        lock = os.path.join(idle.profile(self.root, "ely-doc"), idle.LOCK)
        open(lock, "w").close()
        with unittest.mock.patch("fcntl.fcntl", side_effect=OSError(5, "EIO")), self.assertRaises(OSError):
            idle.engines(self.root)

    def test_a_holder_whose_argv_is_not_utf8_fails_naming_it(self):
        pid = self.hold("ely-doc", [b"\xff"])
        with self.assertRaisesRegex(RuntimeError, f"pid {pid}'s command line is not UTF-8"):
            idle.engines(self.root)


    def test_a_lock_holder_that_is_not_our_worker_fails_naming_it(self):
        pid = self.hold("ely-doc", args=["x", idle.profile(self.root, "ely-doc")])
        with self.assertRaisesRegex(RuntimeError, f"pid {pid} holds ely-doc's idle lock but is not its idle worker"):
            idle.engines(self.root)


class Check(unittest.TestCase):
    def test_our_argv_passes(self):
        idle.check(argv("ely-doc"), ROOT, "ely-doc", 10)

    def test_a_wrong_holder_fails_naming_its_pid(self):
        wrong = {
            "another root": argv("ely-doc", "/elsewhere"),
            "the profile inside a longer argument": [a + "-old" if a.endswith("lok-profile") else a for a in argv("ely-doc")],
            "no --idle": [a for a in argv("ely-doc") if a != "--idle"],
            "another app's idle name": [a.replace("idle-ely-doc", "idle-ely-pdf") for a in argv("ely-doc")],
            "a name with more after it": [a + "0" if a.startswith("idle-") else a for a in argv("ely-doc")],
            "another program": ["/usr/bin/python3", *argv("ely-doc")[1:]],
            "a worker-like name": [WORKER + "-old", *argv("ely-doc")[1:]],
        }
        for why, args in wrong.items():
            with self.subTest(why), self.assertRaisesRegex(RuntimeError, "pid 42 holds ely-doc's idle lock but is not"):
                idle.check(args, ROOT, "ely-doc", 42)


    def test_a_windows_worker_path_passes(self):
        args = ["C:\\Program Files\\Ely\\ely-lok-worker.exe", *argv("ely-doc")[1:]]
        with unittest.mock.patch.object(sys, "platform", "win32"):
            idle.check(args, ROOT, "ely-doc", 10)


class Windows(unittest.TestCase):
    def rm(self, *lists):
        """A Restart Manager whose RmGetList answers `lists` in turn."""
        answers = iter(lists)

        def get(session, needed, count, found, why):
            said, pids = next(answers)
            needed._obj.value = len(pids)
            if found is not None:
                for n, pid in enumerate(pids):
                    found[n].pid = pid
                count._obj.value = len(pids)
            return said
        return unittest.mock.Mock(**{"RmStartSession.return_value": 0, "RmRegisterResources.return_value": 0,
                                     "RmGetList.side_effect": get})

    def holders(self, rm):
        with tempfile.NamedTemporaryFile() as lock, \
                unittest.mock.patch.object(idle.ctypes, "WinDLL", return_value=rm, create=True):
            return idle.windows_holders(lock.name)

    def test_the_holder_is_listed(self):
        rm = self.rm((234, [4242]), (0, [4242]))
        self.assertEqual(self.holders(rm), [4242])
        rm.RmEndSession.assert_called_once()

    def test_the_path_goes_only_to_restart_manager(self):
        rm = self.rm((0, []))
        with unittest.mock.patch.object(idle.ctypes, "WinDLL", return_value=rm, create=True), \
                unittest.mock.patch("builtins.open", side_effect=AssertionError("idle.py opened the lock")), \
                unittest.mock.patch.object(idle.os.path, "exists", side_effect=AssertionError("idle.py stat'd the lock")):
            self.assertEqual(idle.windows_holders("C:\\nowhere\\idle.lock"), [])
        self.assertEqual(rm.RmRegisterResources.call_args[0][2][0], "C:\\nowhere\\idle.lock")

    def test_none_open_is_none(self):
        self.assertEqual(self.holders(self.rm((0, []))), [])

    def test_a_failing_call_raises_never_none(self):
        rm = self.rm((5, []))
        with self.assertRaisesRegex(OSError, "RmGetList"):
            self.holders(rm)
        rm.RmEndSession.assert_called_once()


if __name__ == "__main__":
    unittest.main()
