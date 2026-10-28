"""Records the takes: takes.py <out> <samples> [take ...]"""
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from drive import *  # noqa: F403
from drive import _log


def menus(files):
    """ElyMark: right-click › 表格 ▸, diagonally into the submenu."""
    proc, h = launch("ElyMark", "ely-mark.exe", files / "发布计划.md")
    with take("menus", OUT):
        move_to(900, 520, 0.8)
        time.sleep(0.4)
        click(1100, 394, 0.8, right=True)
        time.sleep(0.9)
        move_to(1200, 632, 0.7)
        time.sleep(1.1)
        # Across 插入, which has its own submenu, to 下移该行.
        move_to(1490, 824, 0.95)
        time.sleep(0.8)
        press()
        time.sleep(1.6)
        move_to(1000, 900, 0.9)
        time.sleep(0.8)


def cells(files):
    """ElyMark: a cell opens alone, shows source; Tab, arrows move."""
    proc, h = launch("ElyMark", "ely-mark.exe", files / "发布计划.md")
    with take("cells", OUT):
        move_to(1000, 480, 0.8)
        time.sleep(0.3)
        click(1236, 344, 0.8)
        time.sleep(1.4)
        key("tab", "Tab")
        time.sleep(1.1)
        key("down", "↓")
        time.sleep(1.0)
        key("end", "End")
        time.sleep(0.3)
        for _ in range(3):
            key("back", "⌫")
            time.sleep(0.16)
        time.sleep(0.3)
        type_text("85%", cps=5)
        time.sleep(0.8)
        key("enter", "Enter")
        time.sleep(1.4)
        move_to(1100, 760, 0.8)
        time.sleep(0.8)


def doc(files):
    """ElyDoc: Word-style table menu, insert a row, drag a border."""
    proc, h = launch("ElyDoc", "ely-doc.exe", files / "工作汇报.docx", wait=8)
    with take("doc", OUT):
        move_to(1000, 760, 0.8)
        time.sleep(0.3)
        click(1070, 634, 0.7)
        time.sleep(0.4)
        click(1070, 634, 0.2, right=True)
        time.sleep(0.9)
        move_to(1140, 722, 0.6)
        time.sleep(0.9)
        # Across 删除, which has its own submenu.
        move_to(1468, 848, 0.9)
        time.sleep(0.7)
        press()
        time.sleep(1.4)
        # The border between 负责人 and 状态.
        move_to(962, 560, 0.8)
        time.sleep(0.7)
        drag(1040, 560, 1.1)
        time.sleep(1.4)


def table(files):
    """ElyTable: Excel-style column header menu, insert a column."""
    proc, h = launch("ElyTable", "ely-table.exe", files / "季度销售.xlsx", wait=8)
    with take("table", OUT):
        move_to(600, 500, 0.8)
        time.sleep(0.3)
        click(406, 208, 0.8, right=True)
        time.sleep(1.1)
        move_to(510, 294, 0.6)
        time.sleep(0.6)
        press()
        time.sleep(1.6)
        move_to(700, 520, 0.8)
        time.sleep(0.6)


def show(files):
    """ElyShow: the slide strip scrolls; PowerPoint-style table menu."""
    proc, h = launch("ElyShow", "ely-show.exe", files / "季度汇报.pptx", wait=8)
    with take("show", OUT):
        move_to(180, 640, 0.9)
        time.sleep(0.3)
        wheel(-9, 0.11)
        time.sleep(0.6)
        wheel(9, 0.11)
        time.sleep(0.5)
        click(180, 380, 0.6)
        time.sleep(1.4)
        click(1180, 530, 0.8)
        time.sleep(0.4)
        click(1180, 530, 0.2, right=True)
        time.sleep(0.9)
        move_to(1250, 618, 0.6)
        time.sleep(0.8)
        move_to(1578, 662, 0.8)
        time.sleep(0.6)
        press()
        time.sleep(1.6)


def more(files):
    """ElyMark: a second launch hands over; the recent list clears."""
    proc, h = launch("ElyMark", "ely-mark.exe", files / "发布计划.md", files / "读书笔记.md")
    with take("more", OUT):
        move_to(900, 600, 0.8)
        time.sleep(0.6)
        env = dict(os.environ, ELY_LOCALE="zh-CN", ELY_APPEARANCE="dark")
        subprocess.Popen([str(APPS / "ElyMark" / "ely-mark.exe"), str(files / "会议记录.md")], env=env)
        _log("launch", file="会议记录.md")
        time.sleep(2.6)
        click(166, 1082, 1.0)
        time.sleep(1.8)
        move_to(700, 700, 0.8)
        time.sleep(0.6)


OUT = Path(sys.argv[1])
OUT.mkdir(parents=True, exist_ok=True)
SAMPLES = Path(sys.argv[2])
ALL = {"menus": menus, "cells": cells, "doc": doc, "table": table, "show": show, "more": more}
names = sys.argv[3:] or list(ALL)
with fresh_data():
    for name in names:
        # Each take opens untouched copies under the same names.
        with tempfile.TemporaryDirectory(prefix="ely-video-") as tmp:
            files = Path(tmp) / "样例"
            shutil.copytree(SAMPLES, files)
            try:
                ALL[name](files)
            finally:
                quit_all()
                # The next take starts without this one's session.
                if DATA.exists():
                    shutil.rmtree(DATA)
        print("recorded", name, flush=True)
