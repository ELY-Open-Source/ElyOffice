#!/usr/bin/env python3
"""Idle engines: CPU and memory within the record."""

import argparse
import ctypes
import datetime
import json
import ntpath
import os
import posixpath
import re
import sys
import time

import child

RECORD = os.path.join(os.path.dirname(os.path.abspath(__file__)), "idle.json")
APPS = ("ely-doc", "ely-table", "ely-show")
LOCK = "idle.lock"
WORKER = "ely-lok-worker"
NAME = re.compile(r"idle-(ely-[a-z]+)-[0-9a-f]{16}")
# Memory may grow this far past the record.
SLACK = 1.1
# One-shot wakes end by 360 s (SF.31).
SETTLE_S = 400
# Windows wakes: twice the recorded cycles (R20.13).
CYCLE_SLACK = 2
# macOS: windows another app's launch voids, then a fail (R20.107).
VOIDS = 3
# pmset log: assertions made or released, with owner.
ASSERTION = re.compile(r"^(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d [+-]\d{4}) Assertions\s+PID (\d+)\((.*?)\) "
                       r"(Created|Released) (PreventUserIdle\w+)", re.M)


def profile(root, app):
    return os.path.join(root, app, "lok-profile")


def check(args, root, app, pid):
    """The worker, on this app's profile and idle name."""
    # Its folder varies (ELY_LOK_WORKER, the bundle); its name is fixed.
    win = sys.platform == "win32"
    exe = (ntpath if win else posixpath).basename(args[0]) if args else None
    at = args.index("--idle") + 1 if "--idle" in args else len(args)
    name = NAME.fullmatch(args[at]) if at < len(args) else None
    if exe != WORKER + (".exe" if win else "") or profile(root, app) not in args \
            or name is None or name.group(1) != app:
        raise RuntimeError(f"pid {pid} holds {app}'s idle lock but is not its idle worker: {args}")


def strictly(parts, pid):
    try:
        return [part.decode("utf-8") for part in parts]
    except UnicodeDecodeError as bad:
        raise RuntimeError(f"pid {pid}'s command line is not UTF-8: {bad}") from bad


def posix_holders(path):
    """Who holds `path`'s write lock, by F_GETLK; none if free."""
    import fcntl
    import struct
    try:
        file = open(path, "rb")
    except FileNotFoundError:
        return []
    # struct flock: macOS puts the offsets first.
    mac = sys.platform == "darwin"
    layout = "qqihh" if mac else "hhqqi4x"
    ask = struct.pack(layout, 0, 0, 0, fcntl.F_WRLCK, 0) if mac else struct.pack(layout, fcntl.F_WRLCK, 0, 0, 0, 0)
    with file:
        got = struct.unpack(layout, fcntl.fcntl(file, fcntl.F_GETLK, ask))
    kind, pid = (got[3], got[2]) if mac else (got[0], got[4])
    if kind == fcntl.F_UNLCK:
        return []
    if pid <= 0:
        raise RuntimeError(f"{path} is held from outside this pid namespace")
    return [pid]


def linux(pid):
    """Argv, live threads' ns, resident MB, thread ids."""
    with open(f"/proc/{pid}/cmdline", "rb") as file:
        args = strictly(file.read().split(b"\0")[:-1], pid)
    ran, tasks = 0, frozenset(os.listdir(f"/proc/{pid}/task"))
    for task in tasks:
        with open(f"/proc/{pid}/task/{task}/schedstat") as file:
            ran += int(file.read().split()[0])
    with open(f"/proc/{pid}/status") as file:
        rss = next(int(row.split()[1]) for row in file if row.startswith("VmRSS:"))
    return args, ran, rss / 1024, tasks


class Usage(ctypes.Structure):
    """rusage_info_v0."""
    _fields_ = [("uuid", ctypes.c_uint8 * 16)] + [(name, ctypes.c_uint64) for name in (
        "user", "system", "idle_wakeups", "interrupt_wakeups", "pageins", "wired", "resident", "footprint",
        "started", "exited")]


def mac_argv(pid):
    """The real argv, by KERN_PROCARGS2; ps joins it with spaces."""
    import struct
    libc = ctypes.CDLL(None, use_errno=True)
    most, size = ctypes.c_int(), ctypes.c_size_t(ctypes.sizeof(ctypes.c_int))
    # CTL_KERN, KERN_ARGMAX
    if libc.sysctl((ctypes.c_int * 2)(1, 8), 2, ctypes.byref(most), ctypes.byref(size), None, 0):
        raise OSError(ctypes.get_errno(), "sysctl KERN_ARGMAX")
    buffer, size = ctypes.create_string_buffer(most.value), ctypes.c_size_t(most.value)
    # CTL_KERN, KERN_PROCARGS2
    if libc.sysctl((ctypes.c_int * 3)(1, 49, pid), 3, buffer, ctypes.byref(size), None, 0):
        raise OSError(ctypes.get_errno(), f"sysctl KERN_PROCARGS2 {pid}")
    raw = buffer.raw[:size.value]
    count = struct.unpack_from("i", raw)[0]
    # The exec path, its padding, then argv.
    parts = raw[4:].partition(b"\0")[2].lstrip(b"\0").split(b"\0")
    if len(parts) <= count:
        raise RuntimeError(f"pid {pid}'s argv holds fewer than {count} arguments")
    return strictly(parts[:count], pid)


def mac(pid):
    """Argv, mach time units run, resident MB."""
    args = mac_argv(pid)
    usage = Usage()
    if ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True).proc_pid_rusage(pid, 0, ctypes.byref(usage)):
        raise OSError(ctypes.get_errno(), f"proc_pid_rusage {pid}")
    return args, usage.user + usage.system, usage.resident / 2**20, None


class RmProcess(ctypes.Structure):
    """RM_PROCESS_INFO; FILETIME is two DWORDs."""
    _fields_ = [("pid", ctypes.c_uint32), ("started", ctypes.c_uint32 * 2), ("app", ctypes.c_wchar * 256),
                ("service", ctypes.c_wchar * 64), ("kind", ctypes.c_int), ("status", ctypes.c_uint32),
                ("session", ctypes.c_uint32), ("restartable", ctypes.c_int)]


def windows_holders(path):
    """Who has `path` open; only Restart Manager sees it."""
    rm = ctypes.WinDLL("rstrtmgr")
    session, key = ctypes.c_uint32(), ctypes.create_unicode_buffer(33)

    def ok(said, what):
        if said:
            raise OSError(said, f"{what} for {path}")

    ok(rm.RmStartSession(ctypes.byref(session), 0, key), "RmStartSession")
    try:
        ok(rm.RmRegisterResources(session, 1, (ctypes.c_wchar_p * 1)(path), 0, None, 0, None), "RmRegisterResources")
        needed, count, why = ctypes.c_uint32(), ctypes.c_uint32(0), ctypes.c_uint32()
        said = rm.RmGetList(session, ctypes.byref(needed), ctypes.byref(count), None, ctypes.byref(why))
        if said == 0:
            return []
        # ERROR_MORE_DATA: room for every holder, asked again.
        if said != 234:
            ok(said, "RmGetList")
        found, count = (RmProcess * needed.value)(), ctypes.c_uint32(needed.value)
        ok(rm.RmGetList(session, ctypes.byref(needed), ctypes.byref(count), found, ctypes.byref(why)), "RmGetList")
        return [found[n].pid for n in range(count.value)]
    finally:
        rm.RmEndSession(session)


def windows(pid):
    """Argv, CPU cycles, working set MB."""
    # The pipe gets UTF-8, not the console code page.
    query = ("[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); "
             f"Get-CimInstance Win32_Process -Filter 'ProcessId = {int(pid)}' | "
             "Select-Object CommandLine,WorkingSetSize | ConvertTo-Json")
    said = child.run(["powershell.exe", "-NoProfile", "-Command", query], capture_output=True, check=True).stdout.strip()
    if not said:
        raise RuntimeError(f"pid {pid} holds an idle lock but Win32_Process has no such process")
    row = json.loads(said)
    shell, k32 = ctypes.WinDLL("shell32", use_last_error=True), ctypes.WinDLL("kernel32", use_last_error=True)
    shell.CommandLineToArgvW.restype = ctypes.POINTER(ctypes.c_wchar_p)
    count = ctypes.c_int()
    argv = shell.CommandLineToArgvW(row["CommandLine"], ctypes.byref(count))
    if not argv:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        args = [argv[n] for n in range(count.value)]
    finally:
        k32.LocalFree(ctypes.cast(argv, ctypes.c_void_p))
    k32.OpenProcess.restype = ctypes.c_void_p
    # PROCESS_QUERY_LIMITED_INFORMATION
    handle = k32.OpenProcess(0x1000, False, pid)
    if not handle:
        raise ctypes.WinError(ctypes.get_last_error())
    cycles = ctypes.c_uint64()
    try:
        if not k32.QueryProcessCycleTime(ctypes.c_void_p(handle), ctypes.byref(cycles)):
            raise ctypes.WinError(ctypes.get_last_error())
    finally:
        k32.CloseHandle(ctypes.c_void_p(handle))
    return args, cycles.value, int(row["WorkingSetSize"]) / 2**20, None


UNIT = {"linux": "ns", "darwin": "mach time units", "win32": "cycles"}


def others(log, start, end):
    """Non-Ely PreventUserIdle* assertions made or let go in the window."""
    found = []
    for at, pid, name, action, kind in ASSERTION.findall(log):
        when = datetime.datetime.strptime(at, "%Y-%m-%d %H:%M:%S %z").timestamp()
        # A stamp covers its second; all of it precedes `end`.
        if int(start) <= when and when + 1 <= end and not name.startswith("ely-"):
            found.append(f"{at} pid {pid} ({name}) {action.lower()} {kind}")
    return found


def launches(start, end):
    """macOS: apps launched or quit then; each wakes every app."""
    if sys.platform != "darwin":
        return []
    return others(child.run(["pmset", "-g", "log"], capture_output=True, check=True).stdout, start, end)


def window(root, seconds):
    """Engines before and after one window; void evidence, if any."""
    start = time.time()
    before = engines(root)
    time.sleep(seconds)
    # Evidence after the last sample never voids.
    end = time.time()
    after = engines(root)
    return before, after, launches(start, end)


def allowed(kept, app):
    """Idle CPU allowed: Windows by record, elsewhere none."""
    return kept["win32-cycles"][app] * CYCLE_SLACK if sys.platform == "win32" else 0


def engines(root):
    """Each app's engine under `root`: pid, CPU, MB, threads."""
    # Only a lock holder is read; others' argv may vary.
    holders, read = {"linux": (posix_holders, linux), "darwin": (posix_holders, mac),
                     "win32": (windows_holders, windows)}[sys.platform]
    found, missing = {}, []
    for app in APPS:
        held = holders(os.path.join(profile(root, app), LOCK))
        if len(held) > 1:
            raise RuntimeError(f"{len(held)} processes hold {app}'s idle lock: {held}")
        if not held:
            missing.append(app)
            continue
        args, used, mb, tasks = read(held[0])
        check(args, root, app, held[0])
        found[app] = (held[0], used, mb, tasks)
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
    voided = []
    while True:
        before, after, seen = window(args.root, args.seconds)
        if not seen:
            break
        voided.append(seen)
        # Never judged: another app woke the engines (B: R20.108).
        print(f"idle: window {len(voided)} void, other apps launched or quit: {'; '.join(seen)}", flush=True)
        if len(voided) == VOIDS:
            sys.exit(f"idle: {VOIDS} windows void, never a quiet one: " + " | ".join("; ".join(s) for s in voided))
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
        elif spent - used > allowed(kept, app):
            wrong.append(f"{app} used {spent - used} {UNIT[sys.platform]} CPU idle; {allowed(kept, app)} allowed")
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
