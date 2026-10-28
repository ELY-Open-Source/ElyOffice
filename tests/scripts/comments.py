#!/usr/bin/env python3
"""Comments stay on one line and under eleven words."""
import ast
import io
import json
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
    ("tests/", (".sh", ".py", ".swift", ".c", ".js", ".mjs")),
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
# Parsed by TypeScript itself: regexes and JSX text included.
PARSED = {".ts": "ts", ".tsx": "tsx", ".js": "js", ".mjs": "js"}
TAG = re.compile(r"<(/?)([A-Za-z][\w-]*)")


def entry(text, start, end):
    """A span as (first, last, text, alone on its lines)."""
    raw = text[start:end]
    if raw.startswith("<!--"):
        said = raw[4:-3]
    elif raw.startswith("//"):
        said = raw[2:].lstrip("/!")
    else:
        said = raw[2:-2].strip("*!")
    before = text[text.rfind("\n", 0, start) + 1:start]
    after = text[end:].split("\n", 1)[0]
    alone = not before.strip() and not after.strip()
    return text.count("\n", 0, start) + 1, text.count("\n", 0, end) + 1, said.strip("* \n"), alone


def css(text):
    """Comment spans in CSS; strings skipped."""
    found, i, n = [], 0, len(text)
    while i < n:
        if text[i] in "\"'":
            quote, i = text[i], i + 1
            while i < n and text[i] != quote:
                i += 2 if text[i] == "\\" else 1
        elif text.startswith("/*", i):
            end = text.find("*/", i + 2)
            end = n if end < 0 else end + 2
            found.append((i, end))
            i = end - 1
        i += 1
    return found


def html(text):
    """Comment spans outside tags, styles' spans, and script bodies."""
    spans, scripts, i = [], [], 0
    while (i := text.find("<", i)) >= 0:
        if text.startswith("<!--", i):
            end = text.find("-->", i + 4)
            end = len(text) if end < 0 else end + 3
            spans.append((i, end))
            i = end
            continue
        tag = TAG.match(text, i)
        if not tag:
            i += 1
            continue
        # The tag runs to its ">", quoted values skipped.
        j, quote = tag.end(), None
        while j < len(text) and (quote or text[j] != ">"):
            quote = None if text[j] == quote else quote or (text[j] if text[j] in "\"'" else None)
            j += 1
        j += 1
        name = tag.group(2).lower()
        if tag.group(1) or name not in ("script", "style"):
            i = j
            continue
        close = re.compile(rf"</{name}\b", re.I).search(text, j)
        stop = close.start() if close else len(text)
        if name == "style":
            spans += [(j + s, j + e) for s, e in css(text[j:stop])]
        else:
            scripts.append((j, text[j:stop]))
        i = stop
    return spans, scripts


def parsed(jobs):
    """Spans from the TypeScript parser, one list per (body, kind)."""
    if not jobs:
        return []
    here = Path(__file__).resolve()
    ran = subprocess.run(["node", str(here.with_name("comments.mjs")), str(here.parents[2] / "site")],
                         input=json.dumps([{"text": body, "kind": kind} for body, kind in jobs]).encode(),
                         capture_output=True)
    if ran.returncode:
        sys.exit("FAIL: the TypeScript parser did not run (pnpm install in site/):\n" + ran.stderr.decode())
    out = []
    for (body, _), spans in zip(jobs, json.loads(ran.stdout)):
        # JavaScript counts UTF-16 units; Python counts characters.
        index = [i for i, c in enumerate(body) for _ in range(2 if ord(c) > 0xFFFF else 1)] + [len(body)]
        out.append([(index[s], index[e]) for s, e in spans])
    return out


def python(text):
    """Comments and docstrings as (first, last, text, alone)."""
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
    return found


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
texts, found, jobs = {}, {}, []
for name in sorted(filter(checked, listed.decode().split("\0"))):
    path = root / name
    text = texts[name] = path.read_text(encoding="utf-8")
    if path.suffix == ".py":
        found[name] = python(text)
    elif path.suffix == ".css":
        found[name] = [entry(text, s, e) for s, e in css(text)]
    elif path.suffix == ".html":
        spans, scripts = html(text)
        found[name] = [entry(text, s, e) for s, e in spans]
        jobs += [(name, base, body, "js") for base, body in scripts]
    elif path.suffix in PARSED:
        found[name] = []
        jobs.append((name, 0, text, PARSED[path.suffix]))
    else:
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
for (name, base, _, _), spans in zip(jobs, parsed([(body, kind) for _, _, body, kind in jobs])):
    found[name] += [entry(texts[name], base + s, base + e) for s, e in spans]
for name, entries in found.items():
    last = None
    for first, final, said, alone in sorted(entries):
        if len(said.split()) > 10:
            too_long.append(f"{name}:{first}: {said[:60]}")
        # Only comments alone on their lines join into one.
        if final > first or (alone and last == first - 1):
            repeated.setdefault(name, first)
        if alone:
            last = final

problems = too_long + [f"{name}:{number}: comment runs past one line" for name, number in sorted(repeated.items())]
if problems:
    sys.exit("FAIL: comments break the one-line ten-word rule:\n" + "\n".join(problems))
print("comments: every comment is one line and ten words or fewer")
