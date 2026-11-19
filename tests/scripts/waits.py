#!/usr/bin/env python3
"""A test sleep polls for an event or says why."""
import bisect
import re
import subprocess
import sys
from pathlib import Path, PurePosixPath

root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[2]
SLEEP = re.compile(r"\bthread::sleep\(")
EXIT = re.compile(r"\b(break|return)\b(\s+'(\w+))?")
MOD = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?mod\s+(\w+)\s*;")
ROOTS = re.compile(r"(^|/)(src/(lib|main)\.rs|src/bin/[^/]+\.rs|(tests|benches|examples)/[^/]+\.rs)$")
# Where a string, char or comment may start.
TOKEN = re.compile(r"//|/\*|[bc]?r#*\"|[bc]?\"|b?'")
STRING = re.compile(r"[bc]?r(?P<hashes>#*)\"|[bc]?\"")
CHAR = re.compile(r"b?'(\\(x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|.)|[^\\'\n])'")
MARKS = re.compile(r"/\*|\*/")
# Only line ends and indentation survive inside.
INSIDE = re.compile(r"[^\n \t]")
ATTRS = re.compile(r"#!?\[[^\]]*\]|//")
# What stands before a closure, never after an operand.
BEFORE = re.compile(r"(^|[=(\[,{;&]|=>|\breturn|\bmove|\basync|\bmut)\s*$")
LABEL = re.compile(r"'(\w+):\s*$")
ASYNC = re.compile(r"\basync(\s+move)?\s*$")
KEYWORD = re.compile(r"('(\w+):\s*)?\b(for|while|loop)\b")
OPEN, CLOSE = "([{", ")]}"


def stripped(text):
    """The code: strings emptied, comments shown as `//`, layout kept."""
    out, at = [], 0
    while (m := TOKEN.search(text, at)):
        out.append(text[at:m.start()])
        at = m.start()
        word = at > 0 and (text[at - 1].isalnum() or text[at - 1] == "_")
        if text.startswith("//", at):
            end = text.find("\n", at)
            at = end if end >= 0 else len(text)
            out.append("//")
        elif text.startswith("/*", at):
            nesting, end = 0, len(text)
            for mark in MARKS.finditer(text, at):
                nesting += 1 if mark.group() == "/*" else -1
                if not nesting:
                    end = mark.end()
                    break
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
    return "".join(out)


def closures(code):
    """The ends of closure heads `|…|` where an expression starts."""
    ends, at = [], code.find("|")
    while at >= 0:
        pair = code.startswith("||", at)
        end = None
        if BEFORE.search(code[max(0, at - 80):at]):
            end = at + 2 if pair else pipe_end(code, at)
        if end:
            ends.append(end)
        at = code.find("|", end or at + 1 + pair)
    return ends


def pipe_end(code, at):
    """The end of the parameters the `|` at `at` opens."""
    level = 0
    for i in range(at + 1, min(len(code), at + 400)):
        char = code[i]
        if char in OPEN:
            level += 1
        elif char in CLOSE:
            if not level:
                return None
            level -= 1
        elif not level and char in "|;":
            return i + 1 if char == "|" else None
    return None


def header(code, brace):
    """Where `brace`'s statement starts, and the call it sits in."""
    at, level = brace, 0
    while at > 0:
        char = code[at - 1]
        if char in CLOSE:
            # A block before it ends the statement.
            if char == "}" and not level:
                break
            level += 1
        elif char in OPEN:
            if not level:
                break
            level -= 1
        elif not level and char == ";":
            break
        at -= 1
    call = re.search(r"[\w.]*$", code[:at - 1]).group() if at > 0 and code[at - 1] == "(" else ""
    return at, call


class Block:
    """A braced block: its span, kind, label and keyword's offset."""

    def __init__(self, source, start, end, parent):
        self.start, self.end, self.parent = start, end, parent
        code = source.code
        begin, call = header(code, start)
        self.head = code[begin:start]
        bare = ATTRS.sub(lambda m: " " * len(m.group()), self.head)
        lead = len(bare) - len(bare.lstrip())
        # A loop keyword where an expression may start.
        word = next((m for m in KEYWORD.finditer(bare)
                     if BEFORE.search(bare[:m.start()]) or source.after_head(begin + m.start())), None)
        label = LABEL.search(bare)
        self.label = word.group(2) if word else label.group(1) if label else None
        self.at = begin + (word.start(3) if word else lead)
        if word and word.group(3) == "for":
            self.kind = "retry" if re.match(r"\s*_\s+in\b", bare[word.end():]) else "for"
        elif word:
            self.kind = "poll"
        elif source.after_head(start, typed=True):
            self.kind = "any" if call.endswith(".any") else "closure"
        elif ASYNC.search(bare):
            self.kind = "closure"
        elif re.search(r"\bfn\b", bare):
            self.kind = "fn"
        else:
            self.kind = "block"

    def loop(self):
        return self.kind in ("retry", "for", "poll")

    def scope(self):
        return self.kind in ("fn", "closure", "any")


class Source:
    """A file's code: its lines, braced blocks and braceless closures."""

    def __init__(self, text):
        self.code = stripped(text)
        self.lines = self.code.split("\n")
        self.starts = [0] + [m.end() for m in re.finditer("\n", self.code)]
        self.ends = closures(self.code)
        pairs, stack = {}, []
        for m in re.finditer(r"[{}]", self.code):
            if m.group() == "{":
                stack.append(m.start())
            elif stack:
                pairs[stack.pop()] = m.start()
        self.blocks, open_ = [], []
        for start in sorted(pairs):
            while open_ and open_[-1].end < start:
                open_.pop()
            block = Block(self, start, pairs[start], open_[-1] if open_ else None)
            self.blocks.append(block)
            open_.append(block)
        self.opens = [block.start for block in self.blocks]
        # A braceless closure's body runs to its expression's end.
        self.bare = [(end, self.expression_end(end)) for end in self.ends
                     if not re.match(r"\s*(->[^{;]*)?\{", self.code[end:end + 200])]

    def expression_end(self, at):
        level = 0
        while at < len(self.code):
            char = self.code[at]
            if char in OPEN:
                level += 1
            elif char in CLOSE:
                if not level:
                    return at
                level -= 1
            elif not level and char in ";,":
                return at
            at += 1
        return at

    def line(self, at):
        return bisect.bisect_right(self.starts, at) - 1

    def after_head(self, at, typed=False):
        """Whether offset `at` follows a closure head, return type allowed."""
        index = bisect.bisect_right(self.ends, at) - 1
        gap = self.code[self.ends[index]:at] if index >= 0 else None
        return gap is not None and re.fullmatch(r"\s*(->[^{;]*)?" if typed else r"\s*", gap) is not None

    def holder(self, at):
        """The innermost block holding offset `at`."""
        index = bisect.bisect_left(self.opens, at) - 1
        block = self.blocks[index] if index >= 0 else None
        while block is not None and block.end <= at:
            block = block.parent
        return block

    def said(self, at):
        """A `//` comment on the line above offset `at`'s line."""
        line = self.line(at)
        return line > 0 and self.lines[line - 1].lstrip().startswith("//")

    def owns(self, loop, at, exit):
        """Whether exit `exit` at offset `at` leaves `loop` itself."""
        # A braceless closure's exit leaves no loop outside it.
        if any(start <= at < end and not start <= loop.start < end for start, end in self.bare):
            return False
        block = self.holder(at)
        while block is not None:
            if exit.group(3) and block.label == exit.group(3):
                return block is loop
            if block.scope():
                return False
            if not exit.group(3) and exit.group(1) == "break" and block.loop():
                return block is loop
            if block is loop:
                return True
            block = block.parent
        return False

    def polls(self, loop):
        if loop.kind == "poll":
            return True
        return loop.kind == "retry" and any(
            self.owns(loop, m.start(), m) for m in EXIT.finditer(self.code, loop.start, loop.end))

    def explained(self, at):
        """A poll, or a comment above sleep or loop."""
        if self.said(at):
            return True
        block = self.holder(at)
        while block is not None:
            if block.kind == "any":
                return True
            if block.scope():
                return False
            if block.loop():
                return self.polls(block) or self.said(block.at)
            block = block.parent
        return False

    def in_test(self, at):
        """Whether a #[test] fn or #[cfg(test)] mod holds offset `at`."""
        block = self.holder(at)
        while block is not None:
            if ("#[test]" in block.head and block.kind == "fn") or (
                    "#[cfg(test)]" in block.head and re.search(r"\bmod\b", block.head)):
                return True
            block = block.parent
        return False

    def above(self, line):
        """The attribute and comment lines right above `line`."""
        found, i = [], line - 1
        while i >= 0 and self.lines[i].lstrip().startswith(("#[", "//")):
            found.append(self.lines[i].strip())
            i -= 1
        return found


def faults(name, source, whole):
    found = []
    for m in SLEEP.finditer(source.code):
        if not source.explained(m.start()) and (whole or source.in_test(m.start())):
            found.append(f"{name}:{source.line(m.start()) + 1}: a sleep outside a polling loop, with no reason above it")
    return found


def test_path(path):
    return re.search(r"(^|/)tests?(/|\.rs$)|_tests?\.rs$", path) is not None


def declared(path, source, at, name):
    """The files `mod name;` at offset `at` may be."""
    here = PurePosixPath(path)
    base = here.parent if here.name == "mod.rs" or ROOTS.search(path) else here.with_suffix("")
    inline, block = [], source.holder(at)
    while block is not None:
        if m := re.search(r"\bmod\s+(\w+)\s*$", block.head):
            inline.insert(0, m.group(1))
        block = block.parent
    return {str(base.joinpath(*inline, f"{name}.rs")), str(base.joinpath(*inline, name, "mod.rs"))}


def test_files(paths, sources):
    """Test files: by path, or brought in by test code."""
    whole = {path for path in paths if test_path(path)}
    grown = True
    while grown:
        before = len(whole)
        for path in paths:
            source = sources[path]
            for line, text in enumerate(source.lines):
                at = source.starts[line]
                if (m := MOD.match(text)) and (path in whole or "#[cfg(test)]" in source.above(line)
                                               or source.in_test(at)):
                    whole |= declared(path, source, at, m.group(3)) & set(paths)
        grown = len(whole) > before
    return whole


def judged(*lines):
    """Faults in a sample, as if in a test file."""
    return faults("sample", Source("\n".join(lines)), True)


def main():
    t = ("#[test]", "fn t() {")
    bad = [(*t, "    for n in 0..2 {", "        let slow = || {", "            std::thread::sleep(MS);", "        };", "    }", "}"),
           (*t, "    for key in keys {", "        send(key);", "        std::thread::sleep(MS);", "    }", "}"),
           (*t, "    for byte in bytes {", "        if write(byte).is_err() {", "            return;", "        }",
            "        std::thread::sleep(MS);", "    }", "}"),
           ("#[cfg(test)]", "mod tests {", "    fn t() {", "        std::thread::sleep(MS);", "    }", "}"),
           (*t, "    for _ in 0..3 {", "        for key in keys {", "            if send(key) {", "                break;",
            "            }", "        }", "        std::thread::sleep(MS);", "    }", "}"),
           (*t, "    for _ in 0..3 {", "        keys.iter().for_each(|key| {", "            return;", "        });",
            "        send(\"return\"); // return", "        std::thread::sleep(MS);", "    }", "}"),
           (*t, "    for _ in 0..3 {", "        let work = || -> Result<(), String> {", "            if !valid() {",
            "                return Err(\"invalid\".into());", "            }", "            send()", "        };",
            "        work();", "        std::thread::sleep(MS);", "    }", "}"),
           (*t, "    for _ in 0..3 {", "        let work = || { return; };", "        std::thread::sleep(MS);", "    }", "}"),
           (*t, "    for _ in 0..3 {", "        send(r#\"before \"return\" after\"#);", "        std::thread::sleep(MS);", "    }", "}"),
           # Exits inside strings or comments that span lines.
           (*t, "    for _ in 0..3 {", "        send(r#\"", "            break", "        \"#);", "        std::thread::sleep(MS);",
            "    }", "}"),
           (*t, "    for _ in 0..3 {", "        send(\"first", "            return", "        last\");",
            "        std::thread::sleep(MS);", "    }", "}"),
           (*t, "    for _ in 0..3 {", "        /* return", "            break */", "        std::thread::sleep(MS);", "    }", "}"),
           # A braceless closure's exit; an inner loop on one line.
           (*t, "    for _ in 0..3 {", "        let f = || return;", "        std::thread::sleep(MS);", "    }", "}"),
           (*t, "    for _ in 0..3 {", "        for key in keys { if send(key) { break; } }", "        std::thread::sleep(MS);",
            "    }", "}"),
           # Async and labeled blocks; closures with typed parameters.
           (*t, "    for _ in 0..3 {", "        let work = async { return; };", "        std::thread::sleep(MS);", "    }", "}"),
           (*t, "    for _ in 0..3 {", "        let work = async || { return; };", "        std::thread::sleep(MS);", "    }", "}"),
           (*t, "    for _ in 0..3 {", "        let work = &|| { return; };", "        std::thread::sleep(MS);", "    }", "}"),
           (*t, "    for _ in 0..3 {", "        'inner: { break 'inner; }", "        std::thread::sleep(MS);", "    }", "}"),
           (*t, "    for _ in 0..3 {", "        let work = |x: [u8; 3]| { return; };", "        std::thread::sleep(MS);", "    }", "}"),
           (*t, "    for _ in 0..3 {", "        let work = |Point { x, y }: Point| { return; };", "        std::thread::sleep(MS);",
            "    }", "}"),
           # A comment-like line inside a string is no reason.
           (*t, "    let text = r#\"", "// not a comment", "\"#; std::thread::sleep(MS);", "}")]
    good = [("fn t() {", "    // The lock is held this long.", "    std::thread::sleep(MS);", "}"),
            ("fn t() {", "    while !done(|| {", "        true", "    }) {", "        if !tick() {",
             "            std::thread::sleep(MS);", "        }", "    }", "}"),
            ("fn t() {", "    for _ in 0..50 {", "        if done() {", "            return;", "        }",
             "        std::thread::sleep(MS);", "    }", "}"),
            ("fn t() {", "    // Frames past the 1 ms fold.", "    for _ in 0..3 {", "        std::thread::sleep(MS);", "    }", "}"),
            ("fn t() {", "    for byte in [1, 2]", "        .into_iter()", "    {", "        // A slow writer's pace.",
             "        std::thread::sleep(MS);", "    }", "}"),
            ("fn t() {", "    let found = loop {", "        if let Some(x) = read() {", "            break x;", "        }",
             "        std::thread::sleep(MS);", "    };", "}"),
            ("fn t() {", "    for _ in 0..50 {", "        if ready(\"for\") {", "            break;", "        }",
             "        std::thread::sleep(MS);", "    }", "}"),
            ("fn t() {", "    'retry: for _ in 0..50 {", "        for part in parts {", "            if ready(part) {",
             "                break 'retry;", "            }", "        }", "        std::thread::sleep(MS);", "    }", "}"),
            # Real exits beside OR, closures and quote chars.
            (*t, "    for _ in 0..50 {", "        if a() || b() { return; }", "        std::thread::sleep(MS);", "    }", "}"),
            (*t, "    for _ in 0..50 {", "        if ready(|| true) { break; }", "        std::thread::sleep(MS);", "    }", "}"),
            (*t, "    for _ in 0..50 {", "        let f = || { return; }; if f() { break; }", "        std::thread::sleep(MS);",
             "    }", "}"),
            (*t, "    for _ in 0..50 {", "        if next() == '\"' { break; }", "        std::thread::sleep(MS);", "    }", "}"),
            (*t, "    for _ in 0..3 {", "        if not_ready() || { return; } {}", "        std::thread::sleep(MS);", "    }", "}"),
            (*t, "    for _ in 0..3 {", "        if not_ready() || { break; } {}", "        std::thread::sleep(MS);", "    }", "}"),
            (*t, "    for _ in 0..3 {", "        for key in keys { if done(key) { return; } }", "        std::thread::sleep(MS);",
             "    }", "}"),
            (*t, "    // std::thread::sleep(MS) once stood here", "    let s = \"thread::sleep(MS)\";", "}"),
            (*t, "    let wait = || while !ready() { std::thread::sleep(MS); };", "}"),
            (*t, "    let wait = || for _ in 0..50 { if ready() { break; } std::thread::sleep(MS); };", "}"),
            (*t, "    assert!(keys.iter().any(|key| {", "        std::thread::sleep(MS);", "        ready(key)", "    }));", "}")]
    nested = {"src/lib.rs": Source("mod support {\n    #[cfg(test)]\n    mod checks;\n}"),
              "src/support/checks.rs": Source("fn pause() {\n    std::thread::sleep(MS);\n}")}
    wrong = [n for n, lines in enumerate(bad) if not judged(*lines)] + [-1 - n for n, lines in enumerate(good) if judged(*lines)]
    plain = faults("plain", Source("fn work() {\n    std::thread::sleep(MS);\n}"), False)
    if wrong or plain or test_files(list(nested), nested) != {"src/support/checks.rs"}:
        sys.exit(f"FAIL: waits.py misjudges its own samples {wrong}")
    tracked = subprocess.run(["git", "ls-files", "apps", "crates"], cwd=root, capture_output=True, text=True, check=True)
    paths = [path for path in tracked.stdout.splitlines() if path.endswith(".rs")]
    sources = {path: Source((root / path).read_text(encoding="utf-8")) for path in paths}
    whole = test_files(paths, sources)
    found = [fault for path in paths for fault in faults(path, sources[path], path in whole)]
    if found:
        sys.exit("FAIL: test waits name no event:\n" + "\n".join(found))
    print("PASS: every sleep in a test polls for an event or says what it stands for")


main()
