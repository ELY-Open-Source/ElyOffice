#!/usr/bin/env python3
"""A test sleep polls for an event or says why."""
import re
import subprocess
import sys
from pathlib import Path

root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[2]
SLEEP = re.compile(r"\bthread::sleep\(")
# A poll: the nearest loop runs until a condition.
LOOP = re.compile(r"\b(while|loop|for)\b|\.any\(")
POLL = re.compile(r"\b(while|loop)\b|\.any\(")
# A counted retry, not a walk over items.
RETRY = re.compile(r"\bfor\s+_\s+in\b")
EXIT = re.compile(r"\b(break|return)\b")
SCOPE = re.compile(r"\bfn\b|\|[^|]*\|\s*\{?\s*$")
TEST_MOD = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?mod\s+(\w+)\s*;")


def test_path(path):
    return re.search(r"(^|/)tests?(/|\.rs$)|_tests?\.rs$", path) is not None


def depth(line):
    return len(line) - len(line.lstrip())


def opener(lines, at):
    """The line that opens the block holding line `at`."""
    head = next((i for i in range(at - 1, -1, -1) if lines[i].strip() and depth(lines[i]) < depth(lines[at])), None)
    # A header's last line: its first line opens the block.
    while head is not None and lines[head].lstrip()[0] in "{})]":
        level = depth(lines[head])
        head = next((i for i in range(head - 1, -1, -1)
                     if lines[i].strip() and depth(lines[i]) <= level and lines[i].lstrip()[0] not in "})]"), None)
    return head


def attributes(lines, at):
    """The attribute lines right above line `at`."""
    found, i = [], at - 1
    while i >= 0 and (lines[i].lstrip().startswith("#[") or lines[i].lstrip().startswith("//")):
        found.append(lines[i].strip())
        i -= 1
    return found


def in_test(lines, at):
    """Whether a #[test] fn or #[cfg(test)] mod holds line `at`."""
    head = opener(lines, at)
    while head is not None:
        marks = attributes(lines, head)
        if ("#[test]" in marks and re.search(r"\bfn\b", lines[head])) or (
                "#[cfg(test)]" in marks and re.search(r"\bmod\b", lines[head])):
            return True
        head = opener(lines, head)
    return False


def body(lines, head):
    """The lines of the block `head` opens."""
    end = next((i for i in range(head + 1, len(lines)) if lines[i].strip() and depth(lines[i]) <= depth(lines[head])
                and lines[i].lstrip()[0] not in "{.)"), len(lines))
    return lines[head + 1:end]


def polls(lines, head):
    retry = RETRY.search(lines[head]) and any(EXIT.search(line) for line in body(lines, head))
    return bool(POLL.search(lines[head]) or retry)


def loop_of(lines, at):
    """The nearest loop of the function holding line `at`."""
    head = opener(lines, at)
    while head is not None:
        if LOOP.search(lines[head]):
            return head
        if SCOPE.search(lines[head]):
            return None
        head = opener(lines, head)
    return None


def said(lines, at):
    return at > 0 and lines[at - 1].lstrip().startswith("//")


def explained(lines, at):
    """A poll, or a comment above sleep or loop."""
    head = loop_of(lines, at)
    return said(lines, at) or (head is not None and (polls(lines, head) or said(lines, head)))


def faults(name, lines, whole):
    found = []
    for at, line in enumerate(lines):
        if not SLEEP.search(line) or explained(lines, at):
            continue
        if whole or in_test(lines, at):
            found.append(f"{name}:{at + 1}: a sleep outside a polling loop, with no reason above it")
    return found


def test_modules(paths, read):
    """Files a #[cfg(test)] mod declaration brings in."""
    found = set()
    for path in paths:
        lines = read(path)
        here = Path(path)
        base = here.parent if here.name in ("mod.rs", "lib.rs", "main.rs") else here.with_suffix("")
        for at, line in enumerate(lines):
            if (m := TEST_MOD.match(line)) and "#[cfg(test)]" in attributes(lines, at):
                found |= {str(base / f"{m.group(3)}.rs"), str(base / m.group(3) / "mod.rs")}
    return found


def main():
    bad = [["#[test]", "fn t() {", "    for n in 0..2 {", "        let slow = || {", "            std::thread::sleep(MS);",
            "        };", "    }", "}"],
           ["#[test]", "fn t() {", "    for key in keys {", "        send(key);", "        std::thread::sleep(MS);", "    }", "}"],
           ["#[test]", "fn t() {", "    for byte in bytes {", "        if write(byte).is_err() {", "            return;",
            "        }", "        std::thread::sleep(MS);", "    }", "}"],
           ["#[cfg(test)]", "mod tests {", "    fn t() {", "        std::thread::sleep(MS);", "    }", "}"]]
    good = [["fn t() {", "    // The lock is held this long.", "    std::thread::sleep(MS);"],
            ["fn t() {", "    while !done(|| {", "        true", "    }) {", "        if !tick() {",
             "            std::thread::sleep(MS);", "        }", "    }"],
            ["fn t() {", "    for _ in 0..50 {", "        if done() {", "            return;", "        }",
             "        std::thread::sleep(MS);", "    }", "}"],
            ["fn t() {", "    // Frames past the 1 ms fold.", "    for _ in 0..3 {", "        std::thread::sleep(MS);",
             "    }", "}"],
            ["fn t() {", "    for byte in [1, 2]", "        .into_iter()", "    {", "        // A slow writer's pace.",
             "        std::thread::sleep(MS);", "    }", "}"],
            ["fn work() {", "    std::thread::sleep(MS);", "}"]]
    if not all(faults("bad", lines, False) for lines in bad) or any(faults("good", lines, False) for lines in good):
        sys.exit("FAIL: waits.py misjudges its own samples")
    tracked = subprocess.run(["git", "ls-files", "apps", "crates"], cwd=root, capture_output=True, text=True, check=True)
    paths = [path for path in tracked.stdout.splitlines() if path.endswith(".rs")]
    texts = {path: (root / path).read_text(encoding="utf-8").split("\n") for path in paths}
    whole = {path for path in paths if test_path(path)} | (test_modules(paths, texts.get) & set(paths))
    found = []
    for path in paths:
        found += faults(path, texts[path], path in whole)
    if found:
        sys.exit("FAIL: test waits name no event:\n" + "\n".join(found))
    print("PASS: every sleep in a test polls for an event or says what it stands for")


main()
