#!/usr/bin/env python3
"""Writes docs/test-map.md: each PRD id, its tasks, their tests."""

import os
import re
import subprocess

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
LATER = ("AGENT",)
# Where each area's tests live, named or not.
WHERE = {
    "UX": "crates/ely-ui/tests、shell.sh、doc-shell.sh、settings.sh",
    "FILE": "crates/ely-files、crates/ely-office、recovery.sh、autosave.sh",
    "LOK": "crates/ely-lok/src/client/tests、crates/ely-lok/src/view/tests、lokview.sh",
    "DOC": "apps/ely-doc/src/tests、doc-props.sh",
    "TABLE": "apps/ely-table/src/tests、theme.sh",
    "SHOW": "apps/ely-show/src/tests",
    "PDF": "apps/ely-pdf/src/tests、crates/ely-pdf-engine/tests、pdf-recovery.sh",
    "TREE": "apps/ely-tree/src/tests",
    "MARK": "apps/ely-mark 单测、mark-*.sh",
    "MAIL": "apps/ely-mail/src/tests、crates/ely-mail-engine/tests",
    "PERF": "tests/perf/bench.py、lag.sh",
    "QA": "见 docs/qa.md",
}


def read(name):
    with open(os.path.join(ROOT, name)) as text:
        return text.read()


def ids():
    """PRD table rows: id and its requirement."""
    for line in read("PRD.md").splitlines():
        if m := re.match(r"\| ([A-Z]+-\d+) \| (.+?) \|", line):
            yield m.group(1), m.group(2).split("|")[0].strip()


def cites(line):
    """PRD ids a line names: single, lists, ranges."""
    cited = set()
    for prefix, rest in re.findall(r"([A-Z]+)-(\d+(?:(?: ?至 ?|、)(?:[A-Z]+-)?\d+)*)", line):
        numbers = re.split(r"(?: ?至 ?|、)(?:[A-Z]+-)?", rest)
        ranges = re.findall(r"(?: ?至 ?|、)", rest)
        n = [int(x) for x in numbers]
        cited.add(n[0])
        for kind, (a, b) in zip(ranges, zip(n, n[1:])):
            cited.update(range(a, b + 1) if "至" in kind else [b])
        yield from (f"{prefix}-{k:02d}" for k in sorted(cited))
        cited = set()


def tasks():
    """Task lines and decisions: a label, the PRD ids, the line."""
    for line in read("tasks/TASKS.md").splitlines():
        if m := re.match(r"- \[(.)\] (C[\d.]+[a-z]?) ", line):
            yield m.group(2) + ("" if m.group(1) == "x" else f"（{m.group(1)}）"), set(cites(line)), line
    for line in read("docs/decisions.md").splitlines():
        if m := re.match(r"- (\d{4}-\d\d-\d\d)[^：]*：.*?（(C[\d.]+)", line):
            yield f"决策 {m.group(2)}", set(cites(line)), line


def known():
    """Test function and script names present in the tree."""
    found = subprocess.run(["grep", "-rhoE", r"fn [a-z0-9_]+\(", "crates", "apps", "--include=*.rs"],
                           cwd=ROOT, capture_output=True, text=True).stdout
    names = {name[3:-1] for name in found.split()}
    names |= {f for f in os.listdir(os.path.join(ROOT, "tests", "scripts"))}
    return names


def main():
    present = known()
    rows = ["# 功能编号与测试对照", "", "对应 PRD §22。由 `scripts/test-map.py` 从 PRD、`tasks/TASKS.md` 与 `docs/decisions.md` 生成：每个编号列出引用它的任务与决策，其中写明、且在代码里确有其名的测试（单测函数名或 `tests/scripts` 的脚本），以及该领域测试所在之处。Agent 属第二批，不列。", "",
            "| 编号 | 要求 | 任务 | 记下的测试 | 测试所在 |", "| --- | --- | --- | --- | --- |"]
    listed = list(tasks())
    for id_, need in ids():
        if id_.startswith(LATER):
            continue
        mine = [(label, line) for label, cited, line in listed if id_ in cited]
        named = []
        for _, line in mine:
            for name in re.findall(r"`([a-z0-9_.-]+)`|([a-z0-9-]+\.sh)", line):
                name = name[0] or name[1]
                if name in present and name not in named:
                    named.append(name)
        labels = list(dict.fromkeys(label for label, _ in mine))
        where = WHERE[id_.split("-")[0]]
        rows.append(f"| {id_} | {need} | {'、'.join(labels) or '无'} | {'、'.join(f'`{n}`' for n in named) or '—'} | {where} |")
    with open(os.path.join(ROOT, "docs", "test-map.md"), "w") as out:
        out.write("\n".join(rows) + "\n")
    print(f"docs/test-map.md: {len(rows) - 6} ids")


if __name__ == "__main__":
    main()
