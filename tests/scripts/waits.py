#!/usr/bin/env python3
"""A test sleep polls for an event or says why."""
import re
import subprocess
import sys
from pathlib import Path, PurePosixPath

root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[2]
SLEEP = re.compile(r"\bthread::sleep\(")
# A poll: the nearest loop runs until a condition.
LOOP = re.compile(r"\b(while|loop|for)\b|\.any\(")
POLL = re.compile(r"\b(while|loop)\b|\.any\(")
# A counted retry, not a walk over items.
RETRY = re.compile(r"\bfor\s+_\s+in\b")
EXIT = re.compile(r"\b(break|return)\b(\s+'(\w+))?")
SCOPE = re.compile(r"\bfn\b|\|[^|]*\|\s*(->[^{]*)?\{?\s*$")
# A closure's head and the brace opening its body.
CLOSURE = re.compile(r"\|[^|]*\|\s*(->[^{]*)?\{")
MOD = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?mod\s+(\w+)\s*;")
INLINE = re.compile(r"\bmod\s+(\w+)\s*\{")
ROOTS = re.compile(r"(^|/)(src/(lib|main)\.rs|src/bin/[^/]+\.rs|(tests|benches|examples)/[^/]+\.rs)$")
# Where a string, char or comment may start.
TOKEN = re.compile(r"//|/\*|[bc]?r#*\"|[bc]?\"|b?'")
STRING = re.compile(r"[bc]?r(?P<hashes>#*)\"|[bc]?\"")
CHAR = re.compile(r"b?'(\\(x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|.)|[^\\'\n])'")
MARKS = re.compile(r"/\*|\*/")
# Only line ends and indentation survive inside.
INSIDE = re.compile(r"[^\n \t]")


def stripped(src):
    """The lines as code: strings and comments emptied, layout kept."""
    text, out, at = "\n".join(src), [], 0
    while (m := TOKEN.search(text, at)):
        out.append(text[at:m.start()])
        at = m.start()
        word = at > 0 and (text[at - 1].isalnum() or text[at - 1] == "_")
        if text.startswith("//", at):
            end = text.find("\n", at)
            at = end if end >= 0 else len(text)
        elif text.startswith("/*", at):
            nesting, end = 0, at
            for mark in MARKS.finditer(text, at):
                nesting += 1 if mark.group() == "/*" else -1
                if not nesting:
                    end = mark.end()
                    break
            else:
                end = len(text)
            out.append(INSIDE.sub("", text[at:end]))
            at = end
        elif word:
            out.append(text[at])
            at += 1
        elif (s := STRING.match(text, at)):
            hashes = s.group("hashes")
            close = re.compile('"' + hashes) if hashes is not None else re.compile(r'"|\\(.|\n)')
            end = s.end()
            while (c := close.search(text, end)) and c.group() != '"' + (hashes or ""):
                end = c.end()
            end = c.end() if c else len(text)
            out.append('"' + INSIDE.sub("", text[s.end():end]) + '"')
            at = end
        elif (c := CHAR.match(text, at)):
            out.append("' '")
            at = c.end()
        else:
            out.append(text[at])
            at += 1
    out.append(text[at:])
    return "".join(out).split("\n")


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


def attributes(src, at):
    """The attribute lines right above line `at`."""
    found, i = [], at - 1
    while i >= 0 and (src[i].lstrip().startswith("#[") or src[i].lstrip().startswith("//")):
        found.append(src[i].strip())
        i -= 1
    return found


def in_test(lines, src, at):
    """Whether a #[test] fn or #[cfg(test)] mod holds line `at`."""
    head = opener(lines, at)
    while head is not None:
        marks = attributes(src, head)
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


def enclosed(line, at):
    """Whether column `at` sits in this line's closure body."""
    for m in CLOSURE.finditer(line[:at]):
        level = 1
        for char in line[m.end():at]:
            level += {"{": 1, "}": -1}.get(char, 0)
            if not level:
                break
        if level:
            return True
    return False


def leaves(lines, head):
    """Whether the loop has its own break or return."""
    label = re.match(r"\s*'(\w+):", lines[head])
    start = head + 1
    for at in range(start, start + len(body(lines, head))):
        for exit in EXIT.finditer(lines[at]):
            if enclosed(lines[at], exit.start()):
                continue
            if label and exit.group(3) == label.group(1):
                return True
            inner = opener(lines, at)
            while inner is not None and inner != head and not (LOOP.search(lines[inner]) or SCOPE.search(lines[inner])):
                inner = opener(lines, inner)
            if inner == head and not exit.group(3):
                return True
    return False


def polls(lines, head):
    return bool(POLL.search(lines[head]) or (RETRY.search(lines[head]) and leaves(lines, head)))


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


def said(src, at):
    return at > 0 and src[at - 1].lstrip().startswith("//")


def explained(lines, src, at):
    """A poll, or a comment above sleep or loop."""
    head = loop_of(lines, at)
    return said(src, at) or (head is not None and (polls(lines, head) or said(src, head)))


def faults(name, src, whole):
    found, lines = [], stripped(src)
    for at, line in enumerate(lines):
        if not SLEEP.search(line) or explained(lines, src, at):
            continue
        if whole or in_test(lines, src, at):
            found.append(f"{name}:{at + 1}: a sleep outside a polling loop, with no reason above it")
    return found


def declared(path, lines, at, name):
    """The files `mod name;` at line `at` may be."""
    here = PurePosixPath(path)
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
    code = {path: stripped(texts[path]) for path in paths}
    grown = True
    while grown:
        before = len(whole)
        for path in paths:
            src, lines = texts[path], code[path]
            for at, line in enumerate(lines):
                if (m := MOD.match(line)) and (path in whole or "#[cfg(test)]" in attributes(src, at)
                                                or in_test(lines, src, at)):
                    whole |= declared(path, lines, at, m.group(3)) & set(paths)
        grown = len(whole) > before
    return whole


def main():
    test = ["#[test]", "fn t() {"]
    bad = [["#[test]", "fn t() {", "    for n in 0..2 {", "        let slow = || {", "            std::thread::sleep(MS);",
            "        };", "    }", "}"],
           ["#[test]", "fn t() {", "    for key in keys {", "        send(key);", "        std::thread::sleep(MS);", "    }", "}"],
           ["#[test]", "fn t() {", "    for byte in bytes {", "        if write(byte).is_err() {", "            return;",
            "        }", "        std::thread::sleep(MS);", "    }", "}"],
           ["#[cfg(test)]", "mod tests {", "    fn t() {", "        std::thread::sleep(MS);", "    }", "}"],
           ["#[test]", "fn t() {", "    for _ in 0..3 {", "        for key in keys {", "            if send(key) {",
            "                break;", "            }", "        }", "        std::thread::sleep(MS);", "    }", "}"],
           ["#[test]", "fn t() {", "    for _ in 0..3 {", "        keys.iter().for_each(|key| {", "            return;",
            "        });", "        send(\"return\"); // return", "        std::thread::sleep(MS);", "    }", "}"],
           ["#[test]", "fn t() {", "    for _ in 0..3 {", "        let work = || -> Result<(), String> {",
            "            if !valid() {", "                return Err(\"invalid\".into());", "            }", "            send()",
            "        };", "        work();", "        std::thread::sleep(MS);", "    }", "}"],
           ["#[test]", "fn t() {", "    for _ in 0..3 {", "        let work = || { return; };", "        std::thread::sleep(MS);",
            "    }", "}"],
           ["#[test]", "fn t() {", "    for _ in 0..3 {", "        send(r#\"before \"return\" after\"#);",
            "        std::thread::sleep(MS);", "    }", "}"],
           # Exits inside strings that span lines.
           test + ["    for _ in 0..3 {", "        send(r#\"", "            break", "        \"#);", "        std::thread::sleep(MS);",
                   "    }", "}"],
           test + ["    for _ in 0..3 {", "        send(\"first", "            return", "        last\");",
                   "        std::thread::sleep(MS);", "    }", "}"],
           test + ["    for _ in 0..3 {", "        /* return", "            break */", "        std::thread::sleep(MS);", "    }", "}"]]
    good = [["fn t() {", "    // The lock is held this long.", "    std::thread::sleep(MS);"],
            ["fn t() {", "    while !done(|| {", "        true", "    }) {", "        if !tick() {",
             "            std::thread::sleep(MS);", "        }", "    }"],
            ["fn t() {", "    for _ in 0..50 {", "        if done() {", "            return;", "        }",
             "        std::thread::sleep(MS);", "    }", "}"],
            ["fn t() {", "    // Frames past the 1 ms fold.", "    for _ in 0..3 {", "        std::thread::sleep(MS);",
             "    }", "}"],
            ["fn t() {", "    for byte in [1, 2]", "        .into_iter()", "    {", "        // A slow writer's pace.",
             "        std::thread::sleep(MS);", "    }", "}"],
            ["fn work() {", "    std::thread::sleep(MS);", "}"],
            ["fn t() {", "    for _ in 0..50 {", "        if ready(\"for\") {", "            break;", "        }",
             "        std::thread::sleep(MS);", "    }", "}"],
            # Real exits beside OR, closures and quote chars.
            test + ["    for _ in 0..50 {", "        if a() || b() { return; }", "        std::thread::sleep(MS);", "    }", "}"],
            test + ["    for _ in 0..50 {", "        if ready(|| true) { break; }", "        std::thread::sleep(MS);", "    }", "}"],
            test + ["    for _ in 0..50 {", "        let f = || { return; }; if f() { break; }", "        std::thread::sleep(MS);",
                    "    }", "}"],
            test + ["    for _ in 0..50 {", "        if next() == '\"' { break; }", "        std::thread::sleep(MS);", "    }", "}"],
            test + ["    // std::thread::sleep(MS) once stood here", "    let s = \"thread::sleep(MS)\";", "}"]]
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
