#!/usr/bin/env python3
"""A test's sleep polls for an event, or says what it stands for."""
import re
import subprocess
import sys
from pathlib import Path

root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[2]
SLEEP = re.compile(r"\bthread::sleep\(")
# A poll: a loop holds the sleep, inside one function.
LOOP = re.compile(r"\b(while|loop|for)\b|\.any\(")
SCOPE = re.compile(r"\bfn\b|\|[^|]*\|\s*\{?\s*$")


def tests(paths):
    return [path for path in paths if path.endswith(".rs") and re.search(r"(^|/)tests?(/|\.rs$)|_tests?\.rs$", path)]


def depth(line):
    return len(line) - len(line.lstrip())


def opener(lines, at):
    """The line that opens the block holding line `at`."""
    head = next((i for i in range(at - 1, -1, -1) if lines[i].strip() and depth(lines[i]) < depth(lines[at])), None)
    # A condition's last line: its first line opens the block.
    while head is not None and lines[head].lstrip()[0] in "})]":
        level = depth(lines[head])
        head = next((i for i in range(head - 1, -1, -1)
                     if lines[i].strip() and depth(lines[i]) <= level and lines[i].lstrip()[0] not in "})]"), None)
    return head


def polls(lines, at):
    """Whether a loop of the same function holds line `at`."""
    head = opener(lines, at)
    while head is not None:
        if LOOP.search(lines[head]):
            return True
        if SCOPE.search(lines[head]):
            return False
        head = opener(lines, head)
    return False


def faults(name, lines):
    found = []
    for at, line in enumerate(lines):
        if SLEEP.search(line) and not lines[at - 1].lstrip().startswith("//") and not polls(lines, at):
            found.append(f"{name}:{at + 1}: a sleep outside a polling loop, with no reason above it")
    return found


def main():
    bad = ["fn t() {", "    for n in 0..2 {", "        let slow = || {", "            std::thread::sleep(MS);", "        };", "    }", "}"]
    good = [["fn t() {", "    // The lock is held this long.", "    std::thread::sleep(MS);"],
            ["fn t() {", "    while !done(|| {", "        true", "    }) {", "        if !tick() {",
             "            std::thread::sleep(MS);", "        }", "    }"]]
    if not faults("bad", bad) or any(faults("good", lines) for lines in good):
        sys.exit("FAIL: waits.py misjudges its own samples")
    tracked = subprocess.run(["git", "ls-files", "apps", "crates"], cwd=root, capture_output=True, text=True, check=True)
    found = []
    for path in tests(tracked.stdout.splitlines()):
        found += faults(path, (root / path).read_text(encoding="utf-8").split("\n"))
    if found:
        sys.exit("FAIL: test waits name no event:\n" + "\n".join(found))
    print("PASS: every sleep in a test polls for an event or says what it stands for")


main()
