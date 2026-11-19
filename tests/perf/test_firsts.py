#!/usr/bin/env python3
# A trimmed log; Office never holds Ely's record.
import datetime
import os
import sys
import tempfile
import unittest
import unittest.mock

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

        run.HOST = unittest.mock.Mock()
        with unittest.mock.patch.object(firsts, "log_of", lambda slug: log), \
                unittest.mock.patch.object(firsts, "launcher", lambda slug, path: []), \
                unittest.mock.patch.object(firsts.subprocess, "Popen", start), \
                unittest.mock.patch.object(firsts, "TIMEOUT_S", 5):
            took, version = firsts.opened("ely-doc", "/tmp/doc.docx")
        self.assertEqual(version, "0.1.16")
        self.assertGreaterEqual(took, 0)


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

        with unittest.mock.patch.object(firsts, "opened", lambda slug, path: (100, "0.1.16")), \
                unittest.mock.patch.object(firsts, "closed", lambda exe: None), \
                unittest.mock.patch.object(firsts, "CASES", [("ely-doc", "doc.docx", lambda path: None)]), \
                unittest.mock.patch.object(firsts.bench, "write", write), \
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


if __name__ == "__main__":
    unittest.main()
