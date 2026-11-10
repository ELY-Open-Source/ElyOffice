#!/usr/bin/env python3
"""Writes the version and kernel lines of docs/compatibility.md."""

import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
PAGE = "docs/compatibility.md"
MARK = "<!-- 下两行由 scripts/compatibility.py 写出，勿手改 -->"
# Each pinned commit's release name; a new pin is named here.
NAMED = {
    "8221e31b3ac356a1623c672912a3d2b492f7e3d1": "26.2.6.3",
    "8ad45e92f0935d3d87f1db3f873086472a5e1b24": "1.28.5",
    "a018953688f1b10bddf91bff8747068f5f4746a4": "2.56.0",
}


def read(name):
    with open(os.path.join(ROOT, name), encoding="utf-8") as text:
        return text.read()


def found(pattern, name):
    """The one capture `pattern` makes in file `name`."""
    hits = re.findall(pattern, read(name), re.M)
    if len(hits) != 1:
        sys.exit(f"compatibility.py: {name} has {len(hits)} matches for {pattern}")
    return hits[0]


def pin(upstream):
    """An upstream's release name and short commit."""
    commit = found(rf"^fetch {upstream} \S+ ([0-9a-f]{{40}})", "scripts/fetch-references.sh")
    if commit not in NAMED:
        sys.exit(f"compatibility.py: {upstream} is pinned at {commit}; name its release in NAMED")
    return NAMED[commit], commit[:7]


def patches(folder):
    """A patch folder's first and last number and its count."""
    names = sorted(name for name in os.listdir(os.path.join(ROOT, "patches", folder)) if name.endswith(".patch"))
    if not names:
        sys.exit(f"compatibility.py: patches/{folder} holds no patch")
    return f"加 `patches/{folder}` {names[0][:4]}–{names[-1][:4]}，{len(names)} 个"


def lines():
    version = found(r'^version = "([^"]+)"', "Cargo.toml")
    apps = [row.split("\t") for row in read("packaging/apps.tsv").splitlines()]
    first = "、".join(row[1] for row in apps if row[3] == "1")
    office, mupdf, git = pin("libreoffice-core"), pin("mupdf"), pin("git")
    lfs = found(r"^version=v(\S+)", "scripts/fetch-git-lfs.sh")
    gpui = found(r"^version=(\S+)", "scripts/vendor-gpui.sh")
    return [
        f"版本：Ely Office {version}（首批应用：{first}）。",
        f"内核：LibreOffice {office[0]}（{office[1]}，{patches('libreoffice')}）；MuPDF {mupdf[0]}（{mupdf[1]}，{patches('mupdf')}）；"
        f"Git {git[0]}（{git[1]}）与 git-lfs {lfs}；gpui-pre {gpui}（{patches('gpui')}）。",
    ]


def main():
    page = read(PAGE).split("\n")
    if page.count(MARK) != 1:
        sys.exit(f"compatibility.py: {PAGE} needs the marker line once")
    at = page.index(MARK) + 1
    want = lines()
    if sys.argv[1:] == ["--check"]:
        if page[at : at + 2] != want:
            sys.exit(f"compatibility.py: {PAGE} is not this build's; it should read:\n" + "\n".join(want))
        print(f"compatibility: {PAGE} names this build: {want[0]}")
        return
    page[at : at + 2] = want
    with open(os.path.join(ROOT, PAGE), "w", encoding="utf-8", newline="\n") as out:
        out.write("\n".join(page))
    print(f"compatibility: wrote {want[0]}")


if __name__ == "__main__":
    main()
