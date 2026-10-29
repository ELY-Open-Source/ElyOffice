"""Processes and window input for the bench, per platform."""

import collections
import ctypes
import os
import platform
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))


def sh(*args, **kw):
    out = subprocess.run(args, capture_output=True, text=True, **kw)
    if out.returncode:
        raise RuntimeError(f"{args[:4]} exited {out.returncode}: {out.stderr.strip()}")
    return out.stdout


class Host:
    """What every platform answers; memory in KB."""

    name = ""

    def exe(self, slug):
        target = os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target"))
        return os.path.join(target, "release", slug + (".exe" if sys.platform == "win32" else ""))

    def git(self):
        return os.environ.get("ELY_GIT") or os.path.join(ROOT, "engines", "git", "bin", "git")

    def lingering(self, marker):
        return subprocess.run(["pgrep", "-f", marker], capture_output=True).returncode == 0

    def family(self, root):
        """Pid to resident KB, for `root` and its descendants."""
        rows = [tuple(map(int, line.split())) for line in sh("ps", "-A", "-o", "pid=,ppid=,rss=").splitlines()]
        ours = descendants(root, [(pid, ppid) for pid, ppid, _ in rows])
        return {pid: rss for pid, _, rss in rows if pid in ours}

    def settle(self, pid, name):
        """Presses here use the control's own action."""


def descendants(root, pairs):
    ours, grew = {root}, True
    while grew:
        grew = False
        for pid, ppid in pairs:
            if ppid in ours and pid not in ours:
                ours.add(pid)
                grew = True
    return ours


class Mac(Host):
    name = "macOS"
    system = ("/System/Library/",)
    keycodes = {"n": "45", "w": "13"}

    def __init__(self, work):
        self.work = work
        for tool in ["ax-values", "ax-press", "click", "chord"]:
            sh("swiftc", "-O", "-o", os.path.join(work, tool), os.path.join(ROOT, "tests", "scripts", tool + ".swift"))

    def handles(self, pids):
        found = collections.Counter()
        for pid in pids:
            kind = None
            for line in sh("lsof", "-p", str(pid), "-F", "tn").splitlines():
                if line[:1] == "t":
                    kind = line[1:]
                elif line[:1] == "n":
                    found[(kind, line[1:])] += 1
        return found

    def keys(self, script):
        sh("osascript", "-e", 'tell application "System Events"', "-e", script, "-e", "end tell")

    def front(self, pid, title):
        events = f'tell application "System Events" to tell (first process whose unix id is {pid})'
        sh("osascript", "-e", f"{events} to set frontmost to true")
        sh("osascript", "-e", f'{events} to perform action "AXRaise" of window "{title}"')

    def type(self, pid, text):
        self.keys(f'keystroke "{text}"')

    def enter(self, pid):
        self.keys("key code 36")

    def chord(self, pid, letter):
        sh(os.path.join(self.work, "chord"), str(pid), self.keycodes[letter])

    def press(self, pid, name):
        sh(os.path.join(self.work, "ax-press"), str(pid), name)

    def controls(self, pid):
        return sh(os.path.join(self.work, "ax-values"), str(pid))

    def shows(self, pid, name):
        return f"title={name}" in self.controls(pid)

    def click_field(self, pid):
        field = next(line for line in self.controls(pid).splitlines() if line.startswith("AXTextField"))
        x, y = field.rsplit("at=", 1)[1].split(",")
        sh(os.path.join(self.work, "click"), x, y)

    def environment(self):
        gpu = re.findall(r"Chipset Model: (.+)", sh("system_profiler", "SPDisplaysDataType"))
        return {
            "machine": sh("sysctl", "-n", "hw.model").strip(),
            "cpu": sh("sysctl", "-n", "machdep.cpu.brand_string").strip(),
            "cores": sh("sysctl", "-n", "hw.ncpu").strip(),
            "memory_gb": int(sh("sysctl", "-n", "hw.memsize")) // 2**30,
            "gpu": ", ".join(gpu),
            "system": f"macOS {platform.mac_ver()[0]}",
            "fonts": fonts(["/System/Library/Fonts", "/Library/Fonts"]),
        }


def fonts(folders):
    return sum(len(files) for folder in folders for _, _, files in os.walk(folder))


class Linux(Host):
    """X under Xvfb, AT-SPI on a private bus: tests/perf/linux.sh."""

    name = "Linux"
    system = ("/usr/", "/etc/fonts/", "/var/cache/fontconfig/")

    def helper(self, *args):
        return sh(sys.executable, "-I", os.path.join(HERE, "atspi.py"), *map(str, args))

    def handles(self, pids):
        found = collections.Counter()
        for pid in pids:
            folder = f"/proc/{pid}/fd"
            for fd in os.listdir(folder):
                try:
                    target = os.readlink(os.path.join(folder, fd))
                except FileNotFoundError:
                    continue
                # Socket and pipe numbers change; their kind does not.
                found[("fd", re.sub(r"^(socket|pipe):\[\d+\]$", r"\1", target))] += 1
        return found

    def front(self, pid, title):
        self.helper("front", pid, title)

    def type(self, pid, text):
        self.helper("type", pid, text)

    def enter(self, pid):
        self.helper("enter", pid)

    def chord(self, pid, letter):
        self.helper("chord", pid, letter)

    def settle(self, pid, name):
        self.helper("settle", pid, name)

    def press(self, pid, name):
        self.helper("press", pid, name)

    def shows(self, pid, name):
        return self.helper("shows", pid, name).strip() == "yes"

    def click_field(self, pid):
        self.helper("field", pid)

    def environment(self):
        cpu = re.search(r"model name\s*:\s*(.+)", open("/proc/cpuinfo").read()).group(1).strip()
        memory = int(re.search(r"MemTotal:\s*(\d+)", open("/proc/meminfo").read()).group(1))
        release = dict(line.split("=", 1) for line in open("/etc/os-release").read().split("\n") if "=" in line)
        return {
            "machine": cpu,
            "cpu": cpu,
            "cores": str(os.cpu_count()),
            "memory_gb": memory // 2**20,
            "gpu": os.environ.get("ELY_BENCH_GPU", "Mesa lavapipe under Xvfb"),
            "system": f"{release.get('PRETTY_NAME', 'Linux').strip(chr(34))}, kernel {platform.release()}",
            "fonts": fonts(["/usr/share/fonts"]),
        }


class Windows(Host):
    name = "Windows"
    # Counts only, unnamed: bench.handles_steady judges them.
    system = None

    def git(self):
        return os.environ.get("ELY_GIT") or os.path.join(ROOT, "engines", "git", "cmd", "git.exe")

    def ui(self, *args):
        script = os.path.join(HERE, "drive.ps1")
        return sh("powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", script, *map(str, args))

    def lingering(self, marker):
        found = self.ui("lingering", 0, marker)
        return found.strip() == "yes"

    def processes(self):
        """(pid, parent) for every process, from one snapshot."""
        k32 = ctypes.windll.kernel32

        class Entry(ctypes.Structure):
            _fields_ = [("size", ctypes.c_ulong), ("usage", ctypes.c_ulong), ("pid", ctypes.c_ulong),
                        ("heap", ctypes.c_size_t), ("module", ctypes.c_ulong), ("threads", ctypes.c_ulong),
                        ("parent", ctypes.c_ulong), ("base", ctypes.c_long), ("flags", ctypes.c_ulong),
                        ("exe", ctypes.c_wchar * 260)]

        k32.CreateToolhelp32Snapshot.restype = ctypes.c_void_p
        snap = ctypes.c_void_p(k32.CreateToolhelp32Snapshot(2, 0))
        entry, pairs = Entry(), []
        entry.size = ctypes.sizeof(Entry)
        more = k32.Process32FirstW(snap, ctypes.byref(entry))
        while more:
            pairs.append((entry.pid, entry.parent))
            more = k32.Process32NextW(snap, ctypes.byref(entry))
        k32.CloseHandle(snap)
        return pairs

    def opened(self, pid):
        k32 = ctypes.windll.kernel32
        k32.OpenProcess.restype = ctypes.c_void_p
        # PROCESS_QUERY_LIMITED_INFORMATION
        return ctypes.c_void_p(k32.OpenProcess(0x1000, False, pid))

    def family(self, root):
        class Counters(ctypes.Structure):
            _fields_ = [("cb", ctypes.c_ulong), ("faults", ctypes.c_ulong)] + [
                (name, ctypes.c_size_t) for name in ("peak", "working", "qpp", "qp", "qpn", "qn", "page", "peak_page")]

        ours, found = descendants(root, self.processes()), {}
        for pid in ours:
            handle, counters = self.opened(pid), Counters()
            counters.cb = ctypes.sizeof(Counters)
            if handle and ctypes.windll.kernel32.K32GetProcessMemoryInfo(handle, ctypes.byref(counters), counters.cb):
                found[pid] = counters.working // 1024
            ctypes.windll.kernel32.CloseHandle(handle)
        return found

    def handles(self, pids):
        total = ctypes.c_ulong()
        found = collections.Counter()
        for pid in pids:
            handle = self.opened(pid)
            if handle and ctypes.windll.kernel32.GetProcessHandleCount(handle, ctypes.byref(total)):
                found[("handles", "")] += total.value
            ctypes.windll.kernel32.CloseHandle(handle)
        return found

    def front(self, pid, title):
        self.ui("front", pid, title)

    def type(self, pid, text):
        self.ui("type", pid, text)

    def enter(self, pid):
        self.ui("enter", pid)

    def chord(self, pid, letter):
        self.ui("chord", pid, letter)

    def press(self, pid, name):
        self.ui("press", pid, name)

    def shows(self, pid, name):
        return self.ui("shows", pid, name).strip() == "yes"

    def click_field(self, pid):
        self.ui("field", pid)

    def environment(self):
        info = self.ui("machine", 0).strip().split("|")
        cpu, cores, memory, gpu, system = info
        return {
            "machine": cpu,
            "cpu": cpu,
            "cores": cores,
            "memory_gb": int(memory) // 2**30,
            "gpu": gpu,
            "system": system,
            "fonts": fonts([os.path.join(os.environ["WINDIR"], "Fonts")]),
        }


def host(work):
    system = platform.system()
    if system == "Darwin":
        return Mac(work)
    if system == "Windows":
        return Windows()
    if system == "Linux":
        return Linux()
    sys.exit(f"SKIP: no bench driver for {system}")
