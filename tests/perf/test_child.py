"""Strict UTF-8 child output, named on failure (R20.84)."""

import ast
import glob
import os
import subprocess
import sys
import tempfile
import unittest
import unittest.mock

import child
import drive
import run

HERE = os.path.dirname(os.path.abspath(__file__))
# Curly quotes to cp1252; UTF-8 refuses both bytes.
CP1252 = b"\x93quoted\x94\n"
TEXT = "Größe — 文档 ✓\n"
CALLS = {"run", "Popen", "call", "check_call", "check_output"}


def printing(raw):
    return [sys.executable, "-c", f"import sys; sys.stdout.buffer.write({raw!r})"]


def piped(raw):
    return subprocess.Popen(printing(raw), stdout=subprocess.PIPE, encoding="utf-8", errors="strict")


def loose(path):
    """Calls that decode other than strict UTF-8."""
    with open(path, encoding="utf-8") as file:
        tree = ast.parse(file.read())
    found = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Call) and replacing(node):
            found.append(f"{os.path.basename(path)}:{node.lineno}")
            continue
        named = isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
        if not (named and isinstance(node.func.value, ast.Name) and node.func.value.id == "subprocess" and node.func.attr in CALLS):
            continue
        said = {k.arg: k.value.value if isinstance(k.value, ast.Constant) else k.value for k in node.keywords}
        if {"text", "universal_newlines"} & said.keys():
            found.append(f"{os.path.basename(path)}:{node.lineno}")
        elif {"encoding", "errors"} & said.keys() and (said.get("encoding"), said.get("errors")) != ("utf-8", "strict"):
            found.append(f"{os.path.basename(path)}:{node.lineno}")
    return found


def replacing(call):
    """An open or decode that hides bad bytes."""
    errors = [k.value for k in call.keywords if k.arg == "errors"]
    if isinstance(call.func, ast.Attribute) and call.func.attr == "decode" and len(call.args) > 1:
        errors.append(call.args[1])
    return any(not (isinstance(e, ast.Constant) and e.value == "strict") for e in errors)


def loose_source(source):
    with tempfile.TemporaryDirectory() as folder:
        path = os.path.join(folder, "probe.py")
        with open(path, "w", encoding="utf-8") as file:
            file.write(source)
        return loose(path)


class Child(unittest.TestCase):
    def test_utf8_text_reads_back_exactly(self):
        self.assertEqual(child.run(printing(TEXT.encode()), capture_output=True, check=True).stdout, TEXT)

    def test_a_byte_cp1252_reads_but_utf8_refuses_fails_naming_the_command(self):
        self.assertEqual(CP1252.decode("cp1252"), "“quoted”\n")
        with self.assertRaises(RuntimeError) as said:
            child.run(printing(CP1252), capture_output=True, check=True)
        self.assertIn(f"{child.named(printing(CP1252))}: output is not UTF-8", str(said.exception))

    def test_run_hands_subprocess_bytes_and_decodes_them_itself(self):
        """Windows's reader threads would swallow a decode error."""
        said = subprocess.CompletedProcess(["x"], 0, CP1252, b"")
        with unittest.mock.patch.object(child.subprocess, "run", return_value=said) as ran, \
                self.assertRaisesRegex(RuntimeError, "x: output is not UTF-8"):
            child.run(["x"], capture_output=True)
        self.assertFalse({"encoding", "errors", "text", "universal_newlines"} & ran.call_args.kwargs.keys())
        said = subprocess.CompletedProcess(["x"], 0, b"ok", TEXT.encode())
        with unittest.mock.patch.object(child.subprocess, "run", return_value=said):
            out = child.run(["x"], capture_output=True)
        self.assertEqual((out.stdout, out.stderr), ("ok", TEXT))

    def test_crlf_and_lone_cr_read_back_as_text_true_reads_them(self):
        raw = b"sent 1795224692841\r\nnext\rlast\n"
        said = child.run(printing(raw), capture_output=True, check=True).stdout
        self.assertEqual(said, subprocess.run(printing(raw), capture_output=True, text=True).stdout)
        self.assertEqual(said, "sent 1795224692841\nnext\nlast\n")

    def test_drive_reads_when_a_crlf_helper_sent(self):
        """Windows helpers end their lines in CRLF."""
        said = child.run(printing(b"pressed\r\nsent 1795224692841\r\n"), capture_output=True, check=True).stdout
        self.assertEqual(drive.sent(said), 1795224692841.0)

    def test_a_piped_line_reads_back_or_fails_naming_the_command(self):
        good = piped(TEXT.encode())
        self.assertEqual(child.line(good), TEXT)
        good.wait()
        bad = piped(CP1252)
        with self.assertRaises(RuntimeError) as said:
            child.line(bad)
        bad.wait()
        self.assertIn(f"{child.named(printing(CP1252))}: output is not UTF-8", str(said.exception))

    def test_every_call_that_decodes_says_utf8_strictly(self):
        tools = [path for path in glob.glob(os.path.join(HERE, "*.py")) if not os.path.basename(path).startswith("test_")]
        self.assertIn(os.path.join(HERE, "child.py"), tools)
        self.assertEqual([where for path in sorted(tools) for where in loose(path)], [])

    def test_the_scan_finds_a_loose_call(self):
        self.assertEqual(loose_source('subprocess.run(["x"], capture_output=True, text=True)'), ["probe.py:1"])
        self.assertEqual(loose_source('subprocess.run(["x"], errors="strict")'), ["probe.py:1"])
        self.assertEqual(loose_source('subprocess.run(["x"], encoding="utf-8", errors="replace")'), ["probe.py:1"])
        self.assertEqual(loose_source('subprocess.run(["x"], encoding="utf-8", errors="strict")'), [])
        self.assertEqual(loose_source('open("log", encoding="utf-8", errors="replace")'), ["probe.py:1"])
        self.assertEqual(loose_source('raw.decode("utf-8", "ignore")'), ["probe.py:1"])
        self.assertEqual(loose_source('raw.decode(errors="replace")'), ["probe.py:1"])
        self.assertEqual(loose_source('open("log", encoding="utf-8")'), [])

    def test_a_character_split_across_two_reads_reads_back_exactly(self):
        with tempfile.TemporaryDirectory() as folder:
            log = os.path.join(folder, "app.log")
            whole = "文档\n".encode()
            with open(log, "wb") as file:
                file.write(b"first\n" + whole[:2])
            self.assertEqual(child.logged(log), ["first"])
            with open(log, "ab") as file:
                file.write(whole[2:])
            self.assertEqual(child.logged(log), ["first", "文档"])
            self.assertEqual(child.logged(log, len("first\n")), ["文档"])

    def test_a_bad_byte_in_a_complete_line_fails_naming_the_file(self):
        with tempfile.TemporaryDirectory() as folder:
            log = os.path.join(folder, "app.log")
            with open(log, "wb") as file:
                file.write(b"first\n" + CP1252 + b"last\n")
            with self.assertRaises(RuntimeError) as said:
                child.logged(log)
            self.assertIn(f"{log}: a logged line is not UTF-8", str(said.exception))

    def test_a_clean_file_reads_whole(self):
        with tempfile.TemporaryDirectory() as folder:
            log = os.path.join(folder, "app.log")
            with open(log, "w", encoding="utf-8", newline="\n") as file:
                file.write(TEXT + "done\n")
            self.assertEqual(child.logged(log), [TEXT.rstrip("\n"), "done"])



class Ready(unittest.TestCase):
    """App.ready reads the smoke run's bytes (R20.84 review 1)."""

    def ready(self, raw):
        # App's own Popen, with Python standing in for the app.
        host = unittest.mock.Mock(exe=lambda slug, frames: sys.executable, lingering=lambda home: False)
        with tempfile.TemporaryDirectory() as home, unittest.mock.patch.object(run, "HOST", host):
            return run.App("ely-doc", home, printing(raw)[1:]).ready()

    def test_a_ready_line_in_bytes_is_read(self):
        self.assertGreater(self.ready(b"ready ElyDoc\n"), 0)

    def test_no_ready_line_fails_saying_what_came(self):
        with self.assertRaises(RuntimeError) as said:
            self.ready(b"nothing\n")
        self.assertIn("ely-doc did not report ready: 'nothing\\n'", str(said.exception))

    def test_a_bad_byte_fails_naming_the_command(self):
        with self.assertRaises(RuntimeError) as said:
            self.ready(b"ready ElyDoc " + CP1252)
        self.assertIn(f"{child.named(printing(b'ready ElyDoc ' + CP1252))}: output is not UTF-8", str(said.exception))


if __name__ == "__main__":
    unittest.main()
