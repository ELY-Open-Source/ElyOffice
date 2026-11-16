#!/usr/bin/env python3
"""The idle engines under a data folder: no CPU, memory as recorded (SF-13)."""

import argparse
import ctypes
import json
import os
import re
import subprocess
import sys
import time

RECORD = os.path.join(os.path.dirname(os.path.abspath(__file__)), "idle.json")
APPS = ("ely-doc", "ely-table", "ely-show")
IDLE = re.compile(r"--idle idle-(ely-[a-z]+)-[0-9a-f]{16}")
# Memory may grow this far past the record.
SLACK = 1.1
# Drawinglayer frees its text device at 180 s.
SETTLE_S = 200


def linux():
    """Pid, command line, live threads' run time in ns, resident MB, thread ids."""
    for pid in filter(str.isdigit, os.listdir("/proc")):
        try:
            with open(f"/proc/{pid}/cmdline", "rb") as file:
                line = file.read().replace(b"\0", b" ").decode(errors="replace")
            if not IDLE.search(line):
                continue
            ran, tasks = 0, frozenset(os.listdir(f"/proc/{pid}/task"))
            for task in tasks:
                with open(f"/proc/{pid}/task/{task}/schedstat") as file:
                    ran += int(file.read().split()[0])
            with open(f"/proc/{pid}/status") as file:
                rss = next(int(row.split()[1]) for row in file if row.startswith("VmRSS:"))
        except FileNotFoundError:
            continue
        yield int(pid), line, ran, rss / 1024, tasks


class Usage(ctypes.Structure):
    """rusage_info_v0."""
    _fields_ = [("uuid", ctypes.c_uint8 * 16)] + [(name, ctypes.c_uint64) for name in (
        "user", "system", "idle_wakeups", "interrupt_wakeups", "pageins", "wired", "resident", "footprint",
        "started", "exited")]


def mac():
    """Pid, command line, mach time units run, resident MB."""
    proc = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
    rows = subprocess.run(["ps", "-A", "-o", "pid=,command="], capture_output=True, text=True, check=True).stdout
    for row in rows.splitlines():
        pid, line = row.split(None, 1)
        if not IDLE.search(line):
            continue
        usage = Usage()
        if proc.proc_pid_rusage(int(pid), 0, ctypes.byref(usage)):
            raise OSError(ctypes.get_errno(), f"proc_pid_rusage {pid}")
        yield int(pid), line, usage.user + usage.system, usage.resident / 2**20, None


def windows():
    """Pid, command line, CPU cycles, working set MB."""
    query = ("Get-CimInstance Win32_Process -Filter \"Name = 'ely-lok-worker.exe' and CommandLine like '%--idle idle-%'\" | "
             "Select-Object ProcessId,CommandLine,WorkingSetSize | ConvertTo-Json")
    said = subprocess.run(["powershell.exe", "-NoProfile", "-Command", query], capture_output=True, text=True,
                          check=True).stdout.strip()
    rows = json.loads(said) if said else []
    k32 = ctypes.WinDLL("kernel32", use_last_error=True)
    k32.OpenProcess.restype = ctypes.c_void_p
    for row in rows if isinstance(rows, list) else [rows]:
        # PROCESS_QUERY_LIMITED_INFORMATION
        handle = k32.OpenProcess(0x1000, False, row["ProcessId"])
        if not handle:
            raise ctypes.WinError(ctypes.get_last_error())
        cycles = ctypes.c_uint64()
        try:
            if not k32.QueryProcessCycleTime(ctypes.c_void_p(handle), ctypes.byref(cycles)):
                raise ctypes.WinError(ctypes.get_last_error())
        finally:
            k32.CloseHandle(ctypes.c_void_p(handle))
        yield row["ProcessId"], row["CommandLine"], cycles.value, int(row["WorkingSetSize"]) / 2**20, None


UNIT = {"linux": "ns", "darwin": "mach time units", "win32": "cycles"}


def engines(root):
    """App to (pid, CPU used, resident MB, Linux thread ids) under `root`."""
    read = {"linux": linux, "darwin": mac, "win32": windows}[sys.platform]
    found = {}
    for pid, line, used, mb, tasks in read():
        match = IDLE.search(line)
        app = match.group(1) if match else None
        if app in APPS and os.path.join(root, app, "lok-profile") in line:
            if app in found:
                raise RuntimeError(f"two idle engines for {app}: {found[app][0]}, {pid}")
            found[app] = (pid, used, mb, tasks)
    missing = [app for app in APPS if app not in found]
    if missing:
        raise RuntimeError(f"no idle engine for {', '.join(missing)} under {root}")
    return found


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("root", help="the data folder the engines were started with")
    parser.add_argument("--seconds", type=int, default=600)
    parser.add_argument("--settle", type=int, default=SETTLE_S, help="seconds before the window opens")
    parser.add_argument("--record", action="store_true", help="write this memory as the record")
    args = parser.parse_args()
    with open(RECORD, encoding="utf-8") as file:
        kept = json.load(file)
    record, wrong = kept.get(sys.platform), []
    if record is None and not args.record:
        sys.exit(f"idle: {RECORD} has no {sys.platform} record; run with --record once")
    first = engines(args.root)
    time.sleep(args.settle)
    before = engines(args.root)
    time.sleep(args.seconds)
    after = engines(args.root)
    for app in APPS:
        (pid, used, _, tasks), (now, spent, mb, still) = before[app], after[app]
        print(f"{app} engine {pid}: {spent - used} {UNIT[sys.platform]} CPU in {args.seconds} s, {mb:.0f} MB")
        if now != pid or pid != first[app][0]:
            wrong.append(f"{app}'s engine {first[app][0]} became {now}")
        elif tasks != still:
            # A thread that exits takes its run time along.
            wrong.append(f"{app}'s threads changed while idle: {sorted(tasks ^ still)}")
        elif spent < used:
            wrong.append(f"{app}'s CPU counter went back from {used} to {spent}")
        elif spent > used:
            wrong.append(f"{app} used {spent - used} {UNIT[sys.platform]} CPU idle")
        if not args.record and mb > record[app] * SLACK:
            wrong.append(f"{app} holds {mb:.0f} MB; the record is {record[app]} MB")
    if wrong:
        sys.exit("idle: " + "; ".join(wrong))
    if args.record:
        kept[sys.platform] = {app: round(after[app][2]) for app in APPS}
        with open(RECORD, "w", encoding="utf-8", newline="\n") as file:
            json.dump(kept, file, indent=2)
            file.write("\n")


if __name__ == "__main__":
    main()
