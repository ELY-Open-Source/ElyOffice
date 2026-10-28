#!/usr/bin/env python3
"""Holds expect, unwrap, panic! and unreachable! to scripts/panics.tsv."""
# panics.py [root] [--sites]: --sites prints this host's sites.
import collections
import json
import os
import platform
import re
import subprocess
import sys
from pathlib import Path

ARGS = [arg for arg in sys.argv[1:] if not arg.startswith("--")]
ROOT = Path(ARGS[0]).resolve() if ARGS else Path(__file__).resolve().parent.parent
LIST = ROOT / "scripts" / "panics.tsv"
LINTS = ("expect_used", "unwrap_used", "panic", "unreachable")
# Macro bodies escape clippy's lints; these two reach them.
INSIDE = ("disallowed_macros", "disallowed_methods")
BANNED = """disallowed-macros = ["core::panic", "std::panic", "core::unreachable", "std::unreachable"]
disallowed-methods = ["core::option::Option::expect", "core::result::Result::expect",
                      "core::option::Option::unwrap", "core::result::Result::unwrap"]
"""
# Test support, not shipped.
SKIPPED = ("ely-mail-fake",)
HOSTS = {"Darwin": "macos", "Linux": "linux", "Windows": "windows"}
HOST = HOSTS[platform.system()]


def flat(text):
    return re.sub(r"\s+", " ", text).strip()


def expression(span):
    lines = span["text"]
    if len(lines) == 1:
        line = lines[0]
        return flat(line["text"][line["highlight_start"] - 1 : line["highlight_end"] - 1])
    parts = [lines[0]["text"][lines[0]["highlight_start"] - 1 :]]
    parts += [line["text"] for line in lines[1:-1]]
    parts.append(lines[-1]["text"][: lines[-1]["highlight_end"] - 1])
    return flat("\n".join(parts))


def message(kind, expr):
    found = re.finditer(r'r(#*)"(.*?)"\1|"((?:[^"\\]|\\.)*)"', expr)
    literals = [each.group(2) if each.group(2) is not None else each.group(3) for each in found]
    if not literals or kind == "unwrap_used":
        return ""
    return literals[-1] if kind == "expect_used" else literals[0]


def lint():
    conf = ROOT / "target" / "panics-clippy"
    conf.mkdir(parents=True, exist_ok=True)
    (conf / "clippy.toml").write_text(BANNED, encoding="utf-8")
    flags = [arg for lint in LINTS + INSIDE for arg in ("--force-warn", f"clippy::{lint}")]
    skipped = [arg for name in SKIPPED for arg in ("--exclude", name)]
    run = subprocess.run(
        ["cargo", "clippy", "--workspace", *skipped, "--lib", "--bins", "--message-format=json", "--quiet",
         "--", *flags],
        cwd=ROOT, capture_output=True, text=True, encoding="utf-8",
        env=dict(os.environ, CLIPPY_CONF_DIR=str(conf)),
    )
    if run.returncode != 0:
        sys.exit(f"panics: clippy failed on {HOST}\n{run.stderr}")
    return run.stdout


def kind_of(code, text):
    if code in LINTS:
        return code
    if code == "disallowed_methods":
        return "unwrap_used" if "unwrap" in text else "expect_used"
    return "unreachable" if "unreachable" in text else "panic"


def read(output):
    spans, inside = [], []
    for line in output.splitlines():
        if not line.startswith("{"):
            continue
        note = json.loads(line)
        if note.get("reason") != "compiler-message":
            continue
        code = ((note["message"].get("code") or {}).get("code") or "").removeprefix("clippy::")
        # Build scripts run at build time.
        if code not in LINTS + INSIDE or "custom-build" in note["target"]["kind"]:
            continue
        span = next(s for s in note["message"]["spans"] if s["is_primary"])
        file = local(span["file_name"])
        if file is None:
            continue
        at = (file, span["line_start"], span["column_start"], span["line_end"], span["column_end"])
        kind = kind_of(code, note["message"]["message"])
        (spans if code in LINTS else inside).append((at, kind, span))
    found = {}
    for at, kind, span in spans:
        found[(*at, kind)] = expression(span)
    taken = set()
    for at, kind, _ in spans:
        # A call's own hit: the innermost of its kind within it.
        own = [
            hit for hit, (where, theirs, span) in enumerate(inside)
            if hit not in taken and theirs == kind and span.get("expansion") is None
            and within(where, at)
            and not any(other != at and o_kind == kind and within(other, at) and within(where, other)
                        for other, o_kind, _ in spans)
        ]
        if own:
            taken.add(max(own, key=lambda hit: inside[hit][0][1:3]))
    for hit, (at, kind, span) in enumerate(inside):
        if hit not in taken:
            found[(*at, kind)] = flat(span["text"][0]["text"])
    return found


def within(inner, outer):
    return inner[0] == outer[0] and outer[1:3] <= inner[1:3] and inner[3:5] <= outer[3:5]


def local(name):
    path = Path(name.replace("\\", "/"))
    path = path if path.is_absolute() else ROOT / path
    try:
        relative = path.resolve().relative_to(ROOT)
    except ValueError:
        return None
    return relative.as_posix() if (ROOT / relative).is_file() else None


def rows(text):
    listed = []
    for number, line in enumerate(text.splitlines(), 1):
        if number == 1 or not line.strip():
            continue
        parts = line.split("\t")
        if len(parts) != 5:
            sys.exit(f"panics: {LIST.name}:{number} needs five tab-separated fields")
        listed.append((number, *parts))
    return listed


def main():
    found = read(lint())
    if "--sites" in sys.argv:
        for (file, line, *_, kind), expr in sorted(found.items()):
            print(f"{file}\t{kind}\t{expr}\t{message(kind, expr)}")
        return
    failures = []
    listed, mine = collections.Counter(), collections.Counter()
    for number, file, kind, expr, hosts, reason in rows(LIST.read_text(encoding="utf-8")):
        if kind not in LINTS:
            failures.append(f"{LIST.name}:{number}: unknown kind {kind}")
        if not reason.strip():
            failures.append(f"{LIST.name}:{number}: {file} has no reason")
        if not set(hosts.split(",")) <= set(HOSTS.values()):
            failures.append(f"{LIST.name}:{number}: unknown hosts {hosts}")
        listed[kind] += 1
        if HOST in hosts.split(","):
            mine[(file, kind, expr)] += 1
    seen = collections.Counter((file, kind, expr) for (file, *_, kind), expr in found.items())
    for key in sorted(seen.keys() | mine.keys()):
        if seen[key] > mine[key]:
            failures.append(f"new: {key[0]}: {key[1]} {key[2]} ({seen[key]} here, {mine[key]} listed)")
        if seen[key] < mine[key]:
            failures.append(f"gone on {HOST}: {key[0]}: {key[1]} {key[2]} ({seen[key]} here, {mine[key]} listed)")
    here = collections.Counter(kind for (*_, kind) in found)
    for kind in LINTS:
        print(f"panics: {kind}: {listed[kind]} listed, {here[kind]} on {HOST}")
    base = os.environ.get("PANICS_BASE", "HEAD")
    known = subprocess.run(["git", "cat-file", "-e", f"{base}^{{commit}}"], cwd=ROOT)
    if known.returncode != 0:
        sys.exit(f"panics: no commit {base} to count against")
    old = subprocess.run(
        ["git", "show", f"{base}:scripts/panics.tsv"],
        cwd=ROOT, capture_output=True, text=True, encoding="utf-8",
    )
    if old.returncode != 0:
        print(f"panics: {base} has no {LIST.name}; this commit sets the counts")
    else:
        before = collections.Counter(each[2] for each in rows(old.stdout))
        for kind in LINTS:
            if listed[kind] > before[kind]:
                failures.append(f"rise: {kind} {before[kind]} at {base}, {listed[kind]} now")
    if failures:
        print("\n".join(failures))
        sys.exit(f"panics: {len(failures)} sites differ from {LIST.name}")


main()
