#!/usr/bin/env python3
"""Installed apps' first opens after install and reboot."""

import argparse
import ctypes
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time

import bench
import report
import child
import drive
import loadfiles
import run
import samples

CASES = [("ely-doc", "doc.docx", samples.docx), ("ely-table", "sheet.xlsx", samples.xlsx),
         ("ely-show", "deck.pptx", loadfiles.big_pptx)]
# Microsoft Office's program for each sample, Windows only.
OFFICE = {"doc.docx": "WINWORD.EXE", "sheet.xlsx": "EXCEL.EXE", "deck.pptx": "POWERPNT.EXE"}
OFFICE_DIR = r"C:\Program Files\Microsoft Office\root\Office16"
# "After reboot" means within this of the boot.
BOOTED_S = 900
TIMEOUT_S = 120


def launcher(slug, path):
    """What the system runs when the user opens `path`."""
    name = run.APPS[slug]
    if sys.platform == "win32":
        import winreg
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, rf"Software\Ely Office\{name}") as key:
            folder = winreg.QueryValueEx(key, "InstallDir")[0]
        return [os.path.join(folder, f"{slug}-start.exe"), slug, name, path]
    if sys.platform == "darwin":
        return ["/usr/bin/open", "-a", f"/Applications/{name}.app", path]
    return [shutil.which(slug) or sys.exit(f"firsts: no installed {slug} on PATH"), path]


def log_of(slug):
    """The app's log in the default data folder."""
    if sys.platform == "win32":
        base = os.environ["LOCALAPPDATA"]
    elif sys.platform == "darwin":
        base = os.path.expanduser("~/Library/Application Support")
    else:
        base = os.environ.get("XDG_DATA_HOME") or os.path.expanduser("~/.local/share")
    return os.path.join(base, "ElyOffice", slug, "ely.log")


def uptime_s():
    if sys.platform == "win32":
        ctypes.windll.kernel32.GetTickCount64.restype = ctypes.c_ulonglong
        return ctypes.windll.kernel32.GetTickCount64() / 1000
    if sys.platform == "darwin":
        booted = re.search(r"sec = (\d+)", drive.sh("sysctl", "-n", "kern.boottime")).group(1)
        return time.time() - int(booted)
    return float(open("/proc/uptime").read().split()[0])


def opened(slug, path):
    """Ms to the first tile, and the app's version."""
    log, name = log_of(slug), os.path.basename(path)
    since = os.path.getsize(log) if os.path.exists(log) else 0
    # Log stamps are whole milliseconds.
    began = time.time() * 1000 // 1
    subprocess.Popen(launcher(slug, path), creationflags=drive.QUIET)
    until = time.monotonic() + TIMEOUT_S
    while time.monotonic() < until:
        time.sleep(0.05)
        if not os.path.exists(log):
            continue
        # The app trims a long log as it starts.
        if os.path.getsize(log) < since:
            since = 0
        lines = [line for line in child.logged(log, since) if run.STAMP.match(line) and run.logged_at(line) >= began]
        at = next((i for i, line in enumerate(lines) if f"opened {name}" in line), None)
        tile = next((line for line in lines[at or 0:] if "lok: first tile drawn" in line), None) if at is not None else None
        if tile:
            version = next((m.group(1) for line in lines if (m := re.search(r" (\d+\.\d+\.\d+) started$", line))), None)
            if version is None:
                raise RuntimeError(f"{slug}: no start line naming a version in {log}")
            run.HOST.stopped(lines)
            return run.logged_at(tile) - began, version, app_pid(slug, began)
    raise RuntimeError(f"{slug}: no first tile of {name} in {TIMEOUT_S} s; log {log}")


def app_pid(slug, began):
    """The running app's pid, from the door it opened."""
    door = os.path.join(os.path.dirname(log_of(slug)), "instance.door")
    # A killed app leaves its door; older is stale.
    if not os.path.exists(door) or os.path.getmtime(door) * 1000 < began - 1000:
        raise RuntimeError(f"{slug}: no door written since this open in {door}")
    with open(door, encoding="utf-8") as file:
        return int(file.read().split()[1])


def closed(slug, pid):
    """Ends the app by its pid; its engines stay."""
    if sys.platform == "win32":
        subprocess.run(["taskkill", "/pid", str(pid), "/f"], capture_output=True, creationflags=drive.QUIET)
    else:
        os.kill(pid, signal.SIGTERM)
    until = time.monotonic() + TIMEOUT_S
    while alive(pid):
        if time.monotonic() > until:
            raise RuntimeError(f"{slug}: pid {pid} still runs {TIMEOUT_S} s after it was ended")
        time.sleep(0.05)
    # The next open must not race this exit.
    time.sleep(2)


def alive(pid):
    if sys.platform == "win32":
        kernel32 = ctypes.windll.kernel32
        # SYNCHRONIZE: enough to wait on it.
        handle = kernel32.OpenProcess(0x00100000, False, pid)
        if not handle:
            return False
        try:
            return kernel32.WaitForSingleObject(handle, 0) != 0
        finally:
            kernel32.CloseHandle(handle)
    try:
        # Reaps it when it is our child.
        if os.waitpid(pid, os.WNOHANG)[0] == pid:
            return False
    except ChildProcessError:
        pass
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    return True


def titled(base):
    """Visible windows' titles that hold `base`."""
    user32, titles = ctypes.windll.user32, []
    found = ctypes.WINFUNCTYPE(ctypes.c_bool, ctypes.c_void_p, ctypes.c_void_p)

    def each(window, _):
        text = ctypes.create_unicode_buffer(512)
        user32.GetWindowTextW(ctypes.c_void_p(window), text, 512)
        if base in text.value and user32.IsWindowVisible(ctypes.c_void_p(window)):
            titles.append(text.value)
        return True

    user32.EnumWindows(found(each), 0)
    return titles


def office(program, path):
    """Ms until Office shows a window titled with the file."""
    base = os.path.splitext(os.path.basename(path))[0]
    # One open already would read as 0 ms.
    if before := titled(base):
        raise RuntimeError(f"{program}: a window titled {before[0]!r} was open before the open")
    process = subprocess.Popen([os.path.join(OFFICE_DIR, program), path])
    began = time.monotonic()
    while time.monotonic() - began < TIMEOUT_S:
        if titled(base):
            took = (time.monotonic() - began) * 1000
            time.sleep(2)
            subprocess.run(["taskkill", "/im", program, "/f"], capture_output=True, creationflags=drive.QUIET)
            process.wait()
            return took
        time.sleep(0.01)
    subprocess.run(["taskkill", "/im", program, "/f"], capture_output=True, creationflags=drive.QUIET)
    raise RuntimeError(f"{program}: no window titled {base} in {TIMEOUT_S} s")


def watcher(work):
    """The host, set up for no typing, clicks or layouts."""
    if sys.platform != "darwin":
        return drive.host(work)
    # Mac() compiles input tools and picks a keyboard layout.
    host = object.__new__(drive.Mac)
    host.work = work
    return host


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("when", choices=["install", "reboot"])
    parser.add_argument("--out", default=os.path.join(run.ROOT, "docs", "perf"))
    args = parser.parse_args()
    if args.when == "reboot" and uptime_s() > BOOTED_S:
        sys.exit(f"firsts: booted {uptime_s():.0f} s ago; reboot, then run this within {BOOTED_S} s")
    work = tempfile.mkdtemp()
    run.HOST = watcher(work)
    try:
        rows, versions = [], set()
        for slug, name, make in CASES:
            path = os.path.join(work, name)
            make(path)
            took, version, pid = opened(slug, path)
            closed(slug, pid)
            versions.add(version)
            rows.append(("PERF-01", f"{run.APPS[slug]} {name} first open after {args.when}", [took], None))
        if len(versions) != 1:
            sys.exit(f"firsts: the installed apps are of versions {sorted(versions)}")
        env = {**bench.machine(), "version": versions.pop(), "kind": args.when}
        record = report.write(args.out, env, rows, [], 1)
        if sys.platform == "win32":
            compare(work, record)
    finally:
        shutil.rmtree(work, ignore_errors=True)


def compare(work, record):
    """Appends Office's times; a failure is written down."""
    lines = []
    for _, name, _ in CASES:
        try:
            lines.append(f"- {OFFICE[name]} {name}: {office(OFFICE[name], os.path.join(work, name)):.0f} ms")
        except (OSError, RuntimeError) as error:
            lines.append(f"- {OFFICE[name]} {name}: not measured: {error}")
    with open(record, "a", encoding="utf-8", newline="\n") as file:
        file.write("\n".join(["", "Microsoft Office, to a window titled with the file; recorded, not gated:", "", *lines, ""]))


if __name__ == "__main__":
    main()
