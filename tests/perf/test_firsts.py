#!/usr/bin/env python3
# A trimmed log; Office never holds Ely's record.
import datetime
import os
import sys
import tempfile
import unittest
import unittest.mock

import drive
import firsts
import run


def stamp(at):
    return f"[{at.strftime('%Y-%m-%dT%H:%M:%S.%f')[:-3]}Z INFO  ely] "


class Opened(unittest.TestCase):
    def test_a_log_trimmed_at_start_still_finds_this_open(self):
        folder = tempfile.mkdtemp()
        log = os.path.join(folder, "ely.log")
        old = stamp(datetime.datetime(2026, 1, 1, tzinfo=datetime.timezone.utc))
        with open(log, "w") as file:
            file.write("x" * 9000 + "\n")

        def start(*_, **__):
            now = stamp(datetime.datetime.now(datetime.timezone.utc))
            # Trimmed: shorter, an old open kept, this open after it.
            with open(log, "w") as file:
                file.write(f"{old}ElyDoc 0.1.15 started\n{old}opened doc.docx\n{old}lok: first tile drawn\n")
                file.write(f"{now}ElyDoc 0.1.16 started\n{now}opened doc.docx\n{now}lok: first tile drawn\n")
            with open(os.path.join(folder, "instance.door"), "w") as file:
                file.write("50123 4242 00ff\n")

        run.HOST = unittest.mock.Mock()
        with unittest.mock.patch.object(firsts, "log_of", lambda slug: log), \
                unittest.mock.patch.object(firsts, "launcher", lambda slug, path: []), \
                unittest.mock.patch.object(firsts.subprocess, "Popen", start), \
                unittest.mock.patch.object(firsts, "TIMEOUT_S", 5):
            took, version, pid = firsts.opened("ely-doc", "/tmp/doc.docx")
        self.assertEqual(version, "0.1.16")
        self.assertEqual(pid, 4242)
        self.assertGreaterEqual(took, 0)


class Closed(unittest.TestCase):
    """By name would end the successor engine too."""

    def closing(self, platform):
        ran, killed = [], []
        with unittest.mock.patch.object(sys, "platform", platform), \
                unittest.mock.patch.object(firsts.subprocess, "run", lambda args, **_: ran.append(args)), \
                unittest.mock.patch.object(firsts.os, "kill", lambda pid, sig: killed.append((pid, sig))), \
                unittest.mock.patch.object(firsts, "alive", lambda pid: False), \
                unittest.mock.patch.object(firsts.time, "sleep", lambda s: None):
            firsts.closed("ely-doc", 4242)
        return ran, killed

    def test_windows_ends_the_app_by_its_pid_never_its_name(self):
        ran, killed = self.closing("win32")
        self.assertEqual(ran, [["taskkill", "/pid", "4242", "/f"]])
        self.assertEqual(killed, [])

    def test_unix_signals_the_app_s_pid_and_runs_no_pkill(self):
        ran, killed = self.closing("linux")
        self.assertEqual(ran, [])
        self.assertEqual(killed, [(4242, firsts.signal.SIGTERM)])

    def test_a_door_from_before_the_open_is_no_pid(self):
        folder = tempfile.mkdtemp()
        door = os.path.join(folder, "instance.door")
        with open(door, "w") as file:
            file.write("50123 4242 00ff\n")
        os.utime(door, (1, 1))
        with unittest.mock.patch.object(firsts, "log_of", lambda slug: os.path.join(folder, "ely.log")):
            with self.assertRaisesRegex(RuntimeError, "no door written since this open"):
                firsts.app_pid("ely-doc", 10_000_000)


class Office(unittest.TestCase):
    def test_a_failed_office_open_keeps_elys_record(self):
        out = tempfile.mkdtemp()
        record = os.path.join(out, "record.md")

        def write(*_):
            with open(record, "w") as file:
                file.write("ely rows\n")
            return record

        def broken(program, path):
            raise RuntimeError(f"{program}: no window titled x")

        with unittest.mock.patch.object(firsts, "opened", lambda slug, path: (100, "0.1.16", 4242)), \
                unittest.mock.patch.object(firsts, "closed", lambda slug, pid: None), \
                unittest.mock.patch.object(firsts, "CASES", [("ely-doc", "doc.docx", lambda path: None)]), \
                unittest.mock.patch.object(firsts.report, "write", write), \
                unittest.mock.patch.object(firsts.bench, "machine", lambda: {}), \
                unittest.mock.patch.object(firsts.drive, "host", lambda work: None), \
                unittest.mock.patch.object(firsts, "office", broken), \
                unittest.mock.patch.object(sys, "platform", "win32"), \
                unittest.mock.patch.object(sys, "argv", ["firsts.py", "install", "--out", out]):
            firsts.main()
        with open(record) as file:
            text = file.read()
        self.assertIn("ely rows", text)
        self.assertIn("WINWORD.EXE doc.docx: not measured: WINWORD.EXE: no window titled x", text)

    def test_a_window_open_before_the_open_is_no_time(self):
        def launched(*_, **__):
            raise AssertionError("Office started over a window already open")

        with unittest.mock.patch.object(firsts, "titled", lambda base: ["doc.docx - ElyDoc"]), \
                unittest.mock.patch.object(firsts.subprocess, "Popen", launched):
            with self.assertRaisesRegex(RuntimeError, "'doc.docx - ElyDoc' was open before the open"):
                firsts.office("WINWORD.EXE", "/tmp/doc.docx")

    def test_a_window_that_appears_is_timed(self):
        seen = iter([[], [], ["doc - Word"]])
        with unittest.mock.patch.object(firsts, "titled", lambda base: next(seen)), \
                unittest.mock.patch.object(firsts.subprocess, "Popen", unittest.mock.Mock()), \
                unittest.mock.patch.object(firsts.subprocess, "run", unittest.mock.Mock()), \
                unittest.mock.patch.object(firsts.time, "sleep", lambda s: None):
            took = firsts.office("WINWORD.EXE", "/tmp/doc.docx")
        self.assertGreaterEqual(took, 0)


class Watcher(unittest.TestCase):
    def test_on_the_mac_firsts_sets_up_no_input(self):
        ran = []
        same = "com.apple.keylayout.US com.apple.keylayout.US"
        with unittest.mock.patch.object(drive.platform, "system", lambda: "Darwin"), \
                unittest.mock.patch.object(firsts.sys, "platform", "darwin"), \
                unittest.mock.patch.object(drive, "sh", lambda *args, **_: ran.append(" ".join(args)) or same), \
                tempfile.TemporaryDirectory() as work:
            self.assertIsInstance(firsts.watcher(work), drive.Mac)
            self.assertEqual(ran, [])
            # The bench's driver compiles its tools and picks a layout.
            drive.host(work)
        self.assertTrue(any("keyboard.swift" in each for each in ran), ran)


if __name__ == "__main__":
    unittest.main()
