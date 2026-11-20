#!/usr/bin/env python3
"""What a version's notes and record must say to ship."""

import datetime
import os
import re
import subprocess
import sys

ROOT = os.path.abspath(sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(__file__), ".."))
# Releases closer than this say why.
APART_DAYS = 7
# The user, 2026-11-20: notes hold only New and Fixes.
HEADS = {"New", "Fixes", "新功能", "修复"}


def unreviewed():
    """Fixes made after a failed last review: the `[~]` lines."""
    count = 0
    for record in ("tasks/TASKS.md", "tasks/BUGS.md"):
        with open(os.path.join(ROOT, record), encoding="utf-8") as lines:
            count += sum(1 for line in lines if line.lstrip().startswith("- [~] "))
    return count


def last_tag(version):
    """The release tag before `version`, with its day."""
    out = subprocess.run(["git", "for-each-ref", "--sort=creatordate", "--format=%(refname:short) %(creatordate:short)",
                          "refs/tags/v*"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split("\n")
    tags = [line.split() for line in out if line and line.split()[0] != f"v{version}"]
    return tags[-1] if tags else None


def read(*parts):
    path = os.path.join(ROOT, *parts)
    if not os.path.exists(path):
        return None
    with open(path, encoding="utf-8") as file:
        return file.read()


def faults(version):
    """The notes' sections and date; the record's count and gap."""
    text = read("site", "changelog", f"{version}.md")
    if text is None:
        return [f"no release notes at site/changelog/{version}.md"]
    found = []
    other = [head for head in re.findall(r"^### (.+)$", text, re.M) if head not in HEADS]
    if other:
        found.append(f"the notes hold only New and Fixes (新功能, 修复), not: {', '.join(other)}")
    head = re.match(rf"# {re.escape(version)} (\d{{4}}-\d{{2}}-\d{{2}})\n", text)
    if not head:
        return found + [f"the notes must begin: # {version} <YYYY-MM-DD>"]
    record = read("docs", "releases", f"{version}.md")
    if record is None or not record.startswith(f"# {version}\n"):
        return found + [f"no release record at docs/releases/{version}.md beginning: # {version}"]
    count = unreviewed()
    said = re.search(r"^- (\d+) fix(?:es)? .*not been reviewed again", record, re.M)
    if not said or int(said.group(1)) != count:
        line = ("1 fix was made after its last review failed and has not" if count == 1
                else f"{count} fixes were made after their last review failed and have not")
        found.append(f"the record must say: - {line} been reviewed again.")
    return found + gap(record, head.group(1), last_tag(version))


def gap(record, day, last):
    """The record's gap line against the notes' day."""
    said = re.findall(r"^- Released (\d+) days? after (\S+?):(.*)$", record, re.M)
    if len(said) > 1:
        return ["the record says the gap more than once"]
    if last is None:
        return [f"the record says - Released {said[0][0]} days after {said[0][1]}, but no release came before"] if said else []
    days = (datetime.date.fromisoformat(day) - datetime.date.fromisoformat(last[1])).days
    unit = "day" if days == 1 else "days"
    if not said:
        if days < APART_DAYS:
            return [f"{last[0]} came {days} {unit} before; the record must say: - Released {days} {unit} after {last[0][1:]}: <why>"]
        return []
    n, after, why = int(said[0][0]), said[0][1], said[0][2].strip()
    if days >= APART_DAYS:
        return [f"{last[0]} came {days} {unit} before {day}; a week or more needs no reason: drop - Released {n} days after {after}"]
    if (n, after) != (days, last[0][1:]):
        return [f"the record says - Released {n} days after {after}; the notes' {day} is {days} {unit} after {last[0][1:]} ({last[1]})"]
    if not why:
        return [f"the record's gap line gives no reason: - Released {days} {unit} after {last[0][1:]}: <why>"]
    return []


def main():
    found = faults(sys.argv[1])
    for fault in found:
        print(f"release-gate: {fault}", file=sys.stderr)
    sys.exit(1 if found else 0)


if __name__ == "__main__":
    main()
