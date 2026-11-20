"""Strict UTF-8 child output, named on failure (R20.84)."""

import ast
import glob
import os
import subprocess
import sys
import tempfile
import unittest

import child

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
    """Subprocess calls that decode other than strict UTF-8."""
    with open(path, encoding="utf-8") as file:
        tree = ast.parse(file.read())
    found = []
    for node in ast.walk(tree):
        named = isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
        if not (named and isinstance(node.func.value, ast.Name) and node.func.value.id == "subprocess" and node.func.attr in CALLS):
            continue
        said = {k.arg: k.value.value if isinstance(k.value, ast.Constant) else k.value for k in node.keywords}
        if {"text", "universal_newlines"} & said.keys():
            found.append(f"{os.path.basename(path)}:{node.lineno}")
        elif {"encoding", "errors"} & said.keys() and (said.get("encoding"), said.get("errors")) != ("utf-8", "strict"):
            found.append(f"{os.path.basename(path)}:{node.lineno}")
    return found


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


if __name__ == "__main__":
    unittest.main()
