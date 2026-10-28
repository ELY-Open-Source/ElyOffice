#!/usr/bin/env python3
"""Comments stay on one line and under eleven words."""
import ast
import io
import re
import subprocess
import sys
import tokenize
from pathlib import Path

root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[2]
# Folders and endings checked; vendored copies, fixtures are not ours.
RULES = [
    ("crates/", (".rs", ".c", ".h")),
    ("apps/", (".rs",)),
    ("scripts/", (".sh",)),
    ("tests/", (".sh", ".py", ".swift", ".c")),
    ("packaging/", (".sh", ".nsi", ".py", ".swift")),
    ("patches/", (".patch",)),
    ("site/", (".ts", ".tsx", ".js", ".mjs", ".css", ".html", ".py", ".sh")),
]
SKIP = ("vendor", "node_modules", "fixtures")
LINE_COMMENT = {
    ".rs": re.compile(r"^\s*//[!/]?\s?(.*)$"),
    ".c": re.compile(r"^\s*(?://[!/]?\s?(.*)|/\*\s?(.*?)\s*\*/)$"),
    ".h": re.compile(r"^\s*(?://[!/]?\s?(.*)|/\*\s?(.*?)\s*\*/)$"),
    ".nsi": re.compile(r"^\s*;\s?(.*)$"),
    ".swift": re.compile(r"^\s*//\s?(.*)$"),
}

# Scanned by tokens: strings skipped, trailing comments found.
SCANNED = {".ts": True, ".tsx": True, ".js": True, ".mjs": True, ".css": False}


def own_line(text, at):
    return not text[text.rfind("\n", 0, at) + 1:at].strip()


def scan(text, slashes):
    """Comments as (first, last, text, own line); strings skipped."""
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
            found.append((line, line + body.count("\n"), body.strip("* \n"), own_line(text, i)))
            line += body.count("\n")
            i = end + 1
        elif slashes and text.startswith("//", i):
            end = text.find("\n", i)
            end = n if end < 0 else end
            found.append((line, line, text[i + 2:end].lstrip("/!").strip(), own_line(text, i)))
            i = end - 1
        elif c == "\n":
            line += 1
        i += 1
    return found


def html(text):
    """HTML comments, then those in its scripts and styles."""
    found = []
    for match in re.finditer(r"<!--(.*?)-->", text, re.S):
        line = text.count("\n", 0, match.start()) + 1
        found.append((line, line + match.group(1).count("\n"), match.group(1).strip(), own_line(text, match.start())))
    for match in re.finditer(r"<(script|style)\b[^>]*>(.*?)</\1>", text, re.S | re.I):
        base = text.count("\n", 0, match.start(2))
        for first, final, said, own in scan(match.group(2), match.group(1).lower() == "script"):
            found.append((base + first, base + final, said, own))
    return sorted(found)


def python(text):
    """Comments and docstrings as (first, last, text, own line)."""
    found = []
    for token in tokenize.generate_tokens(io.StringIO(text).readline):
        if token.type == tokenize.COMMENT and not (token.start[0] == 1 and token.string.startswith("#!")):
            alone = not token.line[:token.start[1]].strip()
            found.append((token.start[0], token.start[0], token.string.lstrip("#").strip(), alone))
    for node in ast.walk(ast.parse(text)):
        if isinstance(node, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)) and node.body:
            first = node.body[0]
            if isinstance(first, ast.Expr) and isinstance(first.value, ast.Constant) \
                    and isinstance(first.value.value, str):
                found.append((first.lineno, first.end_lineno, first.value.value.strip(), False))
    return sorted(found)


def comment_re(path):
    if path.suffix == ".patch":
        return re.compile(r"^\+\s*(?://[!/]?\s?(.*)|/\*\s?(.*?)\s*\*/)$")
    return LINE_COMMENT.get(path.suffix, re.compile(r"^\s*#\s?(.*)$"))


def checked(name):
    parts = name.split("/")
    if any(part in SKIP for part in parts):
        return False
    return any(name.startswith(folder) and name.endswith(ends) for folder, ends in RULES)


# Only what git tracks: scratch files are not the project's.
listed = subprocess.run(["git", "-C", str(root), "ls-files", "-z"], check=True, capture_output=True).stdout
too_long = []
repeated = {}
for name in sorted(filter(checked, listed.decode().split("\0"))):
    path = root / name
    text = path.read_text(encoding="utf-8")
    if path.suffix in SCANNED or path.suffix in (".html", ".py"):
        if path.suffix == ".py":
            found = python(text)
        elif path.suffix == ".html":
            found = html(text)
        else:
            found = scan(text, SCANNED[path.suffix])
        last = None
        for first, final, said, alone in found:
            if len(said.split()) > 10:
                too_long.append(f"{name}:{first}: {said[:60]}")
            if final > first or (alone and last == first - 1):
                repeated.setdefault(name, first)
            if alone:
                last = final
        continue
    comment = comment_re(path)
    previous = None
    for number, line in enumerate(text.split("\n"), 1):
        if line.startswith("#!"):
            continue
        # A block comment left open spans lines.
        if path.suffix in (".c", ".h") and re.search(r"/\*(?!.*\*/)", line):
            repeated.setdefault(name, number)
        match = comment.match(line)
        said = match and next(group for group in match.groups() if group is not None)
        if match and len(said.split()) > 10:
            too_long.append(f"{name}:{number}: {said[:60]}")
        if match and previous == number - 1:
            repeated.setdefault(name, number)
        previous = number if match else None

problems = too_long + [f"{name}:{number}: comment runs past one line" for name, number in sorted(repeated.items())]
if problems:
    sys.exit("FAIL: comments break the one-line ten-word rule:\n" + "\n".join(problems))
print("comments: every comment is one line and ten words or fewer")
