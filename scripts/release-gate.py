#!/usr/bin/env python3
"""What a version's notes must say before it ships."""

import datetime
import os
import re
import subprocess
import sys

ROOT = os.path.abspath(sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(__file__), ".."))
# Releases closer than this say why.
APART_DAYS = 7


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


def faults(version):
    path = os.path.join(ROOT, "site", "changelog", f"{version}.md")
    if not os.path.exists(path):
        return [f"no release notes at site/changelog/{version}.md"]
    with open(path, encoding="utf-8") as notes:
        text = notes.read()
    found, count = [], unreviewed()
    said = re.search(r"^- (\d+) fixes? .*not been reviewed again", text, re.M)
    if not said or int(said.group(1)) != count:
        found.append(f"the notes must say: - {count} fixes were made after their last review failed and have not been reviewed again.")
    head = re.match(rf"# {re.escape(version)} (\d{{4}}-\d{{2}}-\d{{2}})\n", text)
    if not head:
        return found + [f"the notes must begin: # {version} <YYYY-MM-DD>"]
    last = last_tag(version)
    if last:
        days = (datetime.date.fromisoformat(head.group(1)) - datetime.date.fromisoformat(last[1])).days
        why = re.search(rf"^- Released {days} days? after {re.escape(last[0][1:])}: \S", text, re.M)
        if days < APART_DAYS and not why:
            unit = "day" if days == 1 else "days"
            found.append(f"{last[0]} came {days} {unit} before; the notes must say: - Released {days} {unit} after {last[0][1:]}: <why>")
    return found


def main():
    found = faults(sys.argv[1])
    for fault in found:
        print(f"release-gate: {fault}", file=sys.stderr)
    sys.exit(1 if found else 0)


if __name__ == "__main__":
    main()
