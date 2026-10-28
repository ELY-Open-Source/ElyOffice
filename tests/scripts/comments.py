#!/usr/bin/env python3
"""Comments stay on one line and under eleven words."""
import io
import re
import subprocess
import sys
import tokenize
from pathlib import Path

root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[2]
PATTERNS = [
    "crates/**/*.rs",
    "apps/**/*.rs",
    "scripts/*.sh",
    "tests/**/*.sh",
    "tests/**/*.py",
    "packaging/**/*.sh",
    "packaging/**/*.nsi",
    "packaging/**/*.py",
    "packaging/**/*.swift",
    "tests/engines/*.c",
    "crates/**/*.c",
    "crates/**/*.h",
    "patches/**/*.patch",
    "tests/**/*.swift",
    "site/src/**/*.ts",
    "site/src/**/*.css",
    "site/test/**/*.ts",
    "site/e2e/**/*.ts",
    "site/video/**/*.py",
    "site/video/**/*.sh",
]
LINE_COMMENT = {
    ".rs": re.compile(r"^\s*//[!/]?\s?(.*)$"),
    ".c": re.compile(r"^\s*(?://[!/]?\s?(.*)|/\*\s?(.*?)\s*\*/)$"),
    ".h": re.compile(r"^\s*(?://[!/]?\s?(.*)|/\*\s?(.*?)\s*\*/)$"),
    ".nsi": re.compile(r"^\s*;\s?(.*)$"),
    ".swift": re.compile(r"^\s*//\s?(.*)$"),
}

# Scanned by tokens: strings skipped, trailing comments found.
SCANNED = {".ts": True, ".css": False}


def scan(text, slashes):
    """Comments as (first line, last line, text); strings skipped."""
    found, i, line, n = [], 0, 1, len(text)
    while i < n:
        c = text[i]
        if c in "\"'`":
            i += 1
            while i < n and text[i] != c:
                if text[i] == "\\":
                    i += 1
                elif text[i] == "\n":
                    line += 1
                i += 1
        elif text.startswith("/*", i):
            end = text.find("*/", i + 2)
            end = n if end < 0 else end
            body = text[i + 2:end]
            found.append((line, line + body.count("\n"), body.strip("* \n")))
            line += body.count("\n")
            i = end + 1
        elif slashes and text.startswith("//", i):
            end = text.find("\n", i)
            end = n if end < 0 else end
            found.append((line, line, text[i + 2:end].lstrip("/!").strip()))
            i = end - 1
        elif c == "\n":
            line += 1
        i += 1
    return found


def comment_re(path):
    if path.suffix == ".patch":
        return re.compile(r"^\+\s*(?://[!/]?\s?(.*)|/\*\s?(.*?)\s*\*/)$")
    return LINE_COMMENT.get(path.suffix, re.compile(r"^\s*#\s?(.*)$"))


# Only what git tracks: scratch files are not the project's.
listed = subprocess.run(["git", "-C", str(root), "ls-files", "-z"], check=True, capture_output=True).stdout
tracked = set(listed.decode().split("\0"))
too_long = []
repeated = {}
for pattern in PATTERNS:
    for path in sorted(root.glob(pattern)):
        if path.relative_to(root).as_posix() not in tracked:
            continue
        if path.suffix in SCANNED:
            last = None
            for first, final, said in scan(path.read_text(), SCANNED[path.suffix]):
                if len(said.split()) > 10:
                    too_long.append(f"{path.relative_to(root)}:{first}: {said[:60]}")
                if final > first or last == first - 1:
                    repeated.setdefault(path, first)
                last = final
            continue
        comment = comment_re(path)
        previous = None
        source = path.read_text()
        lines = source.split("\n")
        if path.suffix == ".py":
            # Real comments only: a "#" inside a string is code.
            source, lines = lines, [""] * len(lines)
            for token in tokenize.generate_tokens(io.StringIO("\n".join(source)).readline):
                if token.type == tokenize.COMMENT and not token.line[:token.start[1]].strip():
                    lines[token.start[0] - 1] = token.line.rstrip("\n")
            start = 1 if source and source[0].startswith("#!") else 0
            if start < len(source) and source[start].startswith('"""'):
                doc = []
                for line in source[start:]:
                    doc.append(line.strip('" '))
                    if line.count('"""') >= 2 or (len(doc) > 1 and '"""' in line):
                        break
                lines = lines[:start] + [f"# {text}" for text in doc] + lines[start + len(doc):]
        for number, line in enumerate(lines, 1):
            if line.startswith("#!"):
                continue
            # A block comment left open spans lines.
            if path.suffix in (".c", ".h") and re.search(r"/\*(?!.*\*/)", line):
                repeated.setdefault(path, number)
            match = comment.match(line)
            said = match and next(group for group in match.groups() if group is not None)
            if match and len(said.split()) > 10:
                too_long.append(f"{path.relative_to(root)}:{number}: {said[:60]}")
            if match and previous == number - 1:
                repeated.setdefault(path, number)
            previous = number if match else None

problems = too_long + [
    f"{path.relative_to(root)}:{number}: comment runs past one line"
    for path, number in sorted(repeated.items())
]
if problems:
    sys.exit("FAIL: comments break the one-line ten-word rule:\n" + "\n".join(problems))
print("comments: every comment is one line and ten words or fewer")
