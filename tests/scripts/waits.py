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
EXIT = re.compile(r"\b(break|return)\b(\s+'(\w+))?")
SCOPE = re.compile(r"\bfn\b|\|[^|]*\|\s*(->[^{]*)?\{?\s*$")
# A closure opened and left on one line.
CLOSED = re.compile(r"\|[^|]*\|[^{]*\{[^}]*\b(break|return)\b")
MOD = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?mod\s+(\w+)\s*;")
INLINE = re.compile(r"\bmod\s+(\w+)\s*\{")
ROOTS = re.compile(r"(^|/)(src/(lib|main)\.rs|src/bin/[^/]+\.rs|(tests|benches|examples)/[^/]+\.rs)$")


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


def code(line):
    """The line without strings and comment."""
    line = re.sub(r'r(#*)".*?"\1', '""', line)
    return re.sub(r'"(?:[^"\\]|\\.)*"', '""', line).split("//")[0]


def leaves(lines, head):
    """Whether the loop has its own break or return."""
    label = re.match(r"\s*'(\w+):", lines[head])
    start = head + 1
    for at in range(start, start + len(body(lines, head))):
        exit = EXIT.search(code(lines[at]))
        if not exit or CLOSED.search(code(lines[at])):
            continue
        if label and exit.group(3) == label.group(1):
            return True
        inner = opener(lines, at)
        while inner is not None and inner != head and not (LOOP.search(code(lines[inner])) or SCOPE.search(code(lines[inner]))):
            inner = opener(lines, inner)
        if inner == head and not exit.group(3):
            return True
    return False


def polls(lines, head):
    return bool(POLL.search(code(lines[head])) or (RETRY.search(code(lines[head])) and leaves(lines, head)))


def loop_of(lines, at):
    """The nearest loop of the function holding line `at`."""
    head = opener(lines, at)
    while head is not None:
        if LOOP.search(code(lines[head])):
            return head
        if SCOPE.search(code(lines[head])):
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


def declared(path, lines, at, name):
    """The files `mod name;` at line `at` may be."""
    here = Path(path)
    base = here.parent if here.name == "mod.rs" or ROOTS.search(path) else here.with_suffix("")
    head, inline = opener(lines, at), []
    while head is not None:
        if m := INLINE.search(lines[head]):
            inline.insert(0, m.group(1))
        head = opener(lines, head)
    base = base.joinpath(*inline)
    return {str(base / f"{name}.rs"), str(base / name / "mod.rs")}


def test_files(paths, texts):
    """Test files: by path, or brought in by test code."""
    whole = {path for path in paths if test_path(path)}
    grown = True
    while grown:
        before = len(whole)
        for path in paths:
            lines = texts[path]
            for at, line in enumerate(lines):
                if (m := MOD.match(line)) and (path in whole or "#[cfg(test)]" in attributes(lines, at)
                                                or in_test(lines, at)):
                    whole |= declared(path, lines, at, m.group(3)) & set(paths)
        grown = len(whole) > before
    return whole


def main():
    bad = [["#[test]", "fn t() {", "    for n in 0..2 {", "        let slow = || {", "            std::thread::sleep(MS);",
            "        };", "    }", "}"],
           ["#[test]", "fn t() {", "    for key in keys {", "        send(key);", "        std::thread::sleep(MS);", "    }", "}"],
           ["#[test]", "fn t() {", "    for byte in bytes {", "        if write(byte).is_err() {", "            return;",
            "        }", "        std::thread::sleep(MS);", "    }", "}"],
           ["#[cfg(test)]", "mod tests {", "    fn t() {", "        std::thread::sleep(MS);", "    }", "}"],
           ["#[test]", "fn t() {", "    for _ in 0..3 {", "        for key in keys {", "            if send(key) {",
            "                break;", "            }", "        }", "        std::thread::sleep(MS);", "    }", "}"],
           ["#[test]", "fn t() {", "    for _ in 0..3 {", "        keys.iter().for_each(|key| {", "            return;",
            "        });", "        send(\"return\"); // return", "        std::thread::sleep(MS);", "    }", "}"]]
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
    bad += [["#[test]", "fn t() {", "    for _ in 0..3 {", "        let work = || -> Result<(), String> {",
             "            if !valid() {", "                return Err(\"invalid\".into());", "            }", "            send()",
             "        };", "        work();", "        std::thread::sleep(MS);", "    }", "}"],
            ["#[test]", "fn t() {", "    for _ in 0..3 {", "        let work = || { return; };", "        std::thread::sleep(MS);",
             "    }", "}"],
            ["#[test]", "fn t() {", "    for _ in 0..3 {", "        send(r#\"before \"return\" after\"#);",
             "        std::thread::sleep(MS);", "    }", "}"]]
    good += [["fn t() {", "    for _ in 0..50 {", "        if ready(\"for\") {", "            break;", "        }",
              "        std::thread::sleep(MS);", "    }", "}"]]
    labeled = ["fn t() {", "    'retry: for _ in 0..50 {", "        for part in parts {", "            if ready(part) {",
               "                break 'retry;", "            }", "        }", "        std::thread::sleep(MS);", "    }", "}"]
    nested = {"src/lib.rs": ["mod support {", "    #[cfg(test)]", "    mod checks;", "}"],
              "src/support/checks.rs": ["fn pause() {", "    std::thread::sleep(MS);", "}"]}
    if (not all(faults("bad", lines, False) for lines in bad) or any(faults("good", lines, False) for lines in good + [labeled])
            or test_files(list(nested), nested) != {"src/support/checks.rs"}):
        sys.exit("FAIL: waits.py misjudges its own samples")
    tracked = subprocess.run(["git", "ls-files", "apps", "crates"], cwd=root, capture_output=True, text=True, check=True)
    paths = [path for path in tracked.stdout.splitlines() if path.endswith(".rs")]
    texts = {path: (root / path).read_text(encoding="utf-8").split("\n") for path in paths}
    whole = test_files(paths, texts)
    found = []
    for path in paths:
        found += faults(path, texts[path], path in whole)
    if found:
        sys.exit("FAIL: test waits name no event:\n" + "\n".join(found))
    print("PASS: every sleep in a test polls for an event or says what it stands for")


main()
