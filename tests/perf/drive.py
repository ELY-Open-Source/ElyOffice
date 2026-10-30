"""Processes and window input for the bench, per platform."""

import collections
import ctypes
import os
import platform
import re
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))


def target_dir(frames=False):
    """Shipped builds; PERF-02's profiled ones apart."""
    target = os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target"))
    return os.path.join(target, "frames") if frames else target


# A helper's own console would take the front.
QUIET = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0


def sh(*args, **kw):
    out = subprocess.run(args, capture_output=True, text=True, creationflags=QUIET, **kw)
    if out.returncode:
        raise RuntimeError(f"{args[:4]} exited {out.returncode}: {out.stderr.strip()}")
    return out.stdout


def sent(said):
    """Epoch ms the helper sent its last input."""
    found = re.findall(r"^sent (\d+)$", said, re.M)
    if not found:
        raise RuntimeError(f"the input helper did not say when it sent: {said!r}")
    return float(found[-1])


class Host:
    """What every platform answers; memory in KB."""

    name = ""

    def exe(self, slug, frames=False):
        target = target_dir(frames)
        return os.path.join(target, "release", slug + (".exe" if sys.platform == "win32" else ""))

    def git(self):
        return os.environ.get("ELY_GIT") or os.path.join(ROOT, "engines", "git", "bin", "git")

    def lingering(self, marker):
        return subprocess.run(["pgrep", "-f", marker], capture_output=True).returncode == 0

    def pids(self, root):
        """`root` and its descendants, unsampled."""
        rows = [tuple(map(int, line.split())) for line in sh("ps", "-A", "-o", "pid=,ppid=").splitlines()]
        return descendants(root, rows)

    def family(self, root):
        """Pid to resident KB, for `root` and its descendants."""
        rows = [tuple(map(int, line.split())) for line in sh("ps", "-A", "-o", "pid=,ppid=,rss=").splitlines()]
        ours = descendants(root, [(pid, ppid) for pid, ppid, _ in rows])
        found = {pid: rss for pid, _, rss in rows if pid in ours}
        if root not in found:
            raise RuntimeError(f"process {root} is gone; nothing to sample")
        return found

    def ended(self, root, pid):
        """A child read failed because it exited after the listing."""
        return pid not in self.pids(root)

    def handles(self, root, pids):
        found = collections.Counter()
        for pid in pids:
            try:
                found.update(self.held(pid))
            except (OSError, RuntimeError):
                if not self.ended(root, pid):
                    raise
        return found

    def settle(self, pid, name):
        """A new window's controls reach assistive tech a moment later."""
        until = time.monotonic() + 10
        while not self.shows(pid, name):
            if time.monotonic() > until:
                raise RuntimeError(f"no control named {name} in process {pid} in 10 s")
            time.sleep(0.1)


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
    # US layout virtual key codes.
    keycodes = dict(zip("asdfhgzxcvbqweryt123465=97-80]ou[ip lj'k;", map(str, [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
        30, 31, 32, 33, 34, 35, 49, 37, 38, 39, 40, 41])), n="45", m="46", pagedown="121", tab="48")

    def __init__(self, work):
        self.work = work
        for tool in ["ax-values", "ax-press", "click", "chord", "key"]:
            sh("swiftc", "-O", "-o", os.path.join(work, tool), os.path.join(ROOT, "tests", "scripts", tool + ".swift"))

    def held(self, pid):
        found, kind = collections.Counter(), None
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
        if text[:-1]:
            self.keys(f'keystroke "{text[:-1]}"')
        return self.key(pid, text[-1])

    def key(self, pid, name):
        return sent(sh(os.path.join(self.work, "key"), str(pid), self.keycodes[name]))

    def enter(self, pid):
        return sent(sh(os.path.join(self.work, "key"), str(pid), "36"))

    def chord(self, pid, letter):
        sh(os.path.join(self.work, "chord"), str(pid), self.keycodes[letter])

    def press(self, pid, name):
        out = subprocess.run([os.path.join(self.work, "ax-press"), str(pid), name], capture_output=True, text=True)
        if out.returncode or out.stdout.strip() != "pressed":
            raise RuntimeError(f"ax-press {name}: {out.stdout.strip()} {out.stderr.strip()}")
        return sent(out.stderr)

    def controls(self, pid):
        return sh(os.path.join(self.work, "ax-values"), str(pid))

    def shows(self, pid, name):
        return f"title={name}" in self.controls(pid)

    def click_field(self, pid):
        until = time.monotonic() + 10
        while (field := next((l for l in self.controls(pid).splitlines() if l.startswith("AXTextField")), None)) is None:
            if time.monotonic() > until:
                raise RuntimeError(f"no text field in process {pid} in 10 s")
            time.sleep(0.1)
        x, y = field.rsplit("at=", 1)[1].split(",")
        sh(os.path.join(self.work, "click"), x, y, str(pid))

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

    def held(self, pid):
        found, folder = collections.Counter(), f"/proc/{pid}/fd"
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
        return sent(self.helper("type", pid, text))

    def key(self, pid, name):
        return sent(self.helper("key", pid, name))

    def enter(self, pid):
        return sent(self.helper("enter", pid))

    def chord(self, pid, letter):
        self.helper("chord", pid, letter)

    def settle(self, pid, name):
        self.helper("settle", pid, name)

    def press(self, pid, name):
        return sent(self.helper("press", pid, name))

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

    k32 = ctypes.WinDLL("kernel32", use_last_error=True) if sys.platform == "win32" else None

    def failed(self, what, pid):
        return RuntimeError(f"{what} for process {pid} failed: {ctypes.WinError(ctypes.get_last_error())}")

    def processes(self):
        """(pid, parent) for every process, from one snapshot."""

        class Entry(ctypes.Structure):
            _fields_ = [("size", ctypes.c_ulong), ("usage", ctypes.c_ulong), ("pid", ctypes.c_ulong),
                        ("heap", ctypes.c_size_t), ("module", ctypes.c_ulong), ("threads", ctypes.c_ulong),
                        ("parent", ctypes.c_ulong), ("base", ctypes.c_long), ("flags", ctypes.c_ulong),
                        ("exe", ctypes.c_wchar * 260)]

        self.k32.CreateToolhelp32Snapshot.restype = ctypes.c_void_p
        snap = self.k32.CreateToolhelp32Snapshot(2, 0)
        if snap in (None, ctypes.c_void_p(-1).value):
            raise self.failed("a process snapshot", 0)
        entry, pairs = Entry(), []
        entry.size = ctypes.sizeof(Entry)
        try:
            more = self.k32.Process32FirstW(ctypes.c_void_p(snap), ctypes.byref(entry))
            while more:
                pairs.append((entry.pid, entry.parent))
                more = self.k32.Process32NextW(ctypes.c_void_p(snap), ctypes.byref(entry))
            # Only ERROR_NO_MORE_FILES ends a whole list.
            if ctypes.get_last_error() != 18:
                raise self.failed(f"listing processes after {len(pairs)}", 0)
        finally:
            self.k32.CloseHandle(ctypes.c_void_p(snap))
        return pairs

    def pids(self, root):
        return descendants(root, self.processes())

    def query(self, pid, read):
        """`read(handle)` on an open process; any failure voids the run."""
        self.k32.OpenProcess.restype = ctypes.c_void_p
        # PROCESS_QUERY_LIMITED_INFORMATION
        handle = self.k32.OpenProcess(0x1000, False, pid)
        if not handle:
            raise self.failed("opening", pid)
        try:
            return read(ctypes.c_void_p(handle))
        finally:
            self.k32.CloseHandle(ctypes.c_void_p(handle))

    def family(self, root):
        class Counters(ctypes.Structure):
            _fields_ = [("cb", ctypes.c_ulong), ("faults", ctypes.c_ulong)] + [
                (name, ctypes.c_size_t) for name in ("peak", "working", "qpp", "qp", "qpn", "qn", "page", "peak_page")]

        def working(handle):
            counters = Counters()
            counters.cb = ctypes.sizeof(Counters)
            if not self.k32.K32GetProcessMemoryInfo(handle, ctypes.byref(counters), counters.cb):
                raise self.failed("reading memory", pid)
            return counters.working // 1024

        found = {}
        for pid in descendants(root, self.processes()):
            try:
                found[pid] = self.query(pid, working)
            except RuntimeError:
                if not self.ended(root, pid):
                    raise
        return found

    def held(self, pid):
        total = ctypes.c_ulong()
        if not self.query(pid, lambda handle: self.k32.GetProcessHandleCount(handle, ctypes.byref(total))):
            raise self.failed("counting handles", pid)
        return collections.Counter({("handles", ""): total.value})

    def front(self, pid, title):
        self.ui("front", pid, title)

    def type(self, pid, text):
        return sent(self.ui("type", pid, text))

    def key(self, pid, name):
        return sent(self.ui("key", pid, name))

    def enter(self, pid):
        return sent(self.ui("enter", pid))

    def chord(self, pid, letter):
        self.ui("chord", pid, letter)

    def press(self, pid, name):
        return sent(self.ui("press", pid, name))

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
