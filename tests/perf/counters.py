"""Paging counters each second on macOS and Windows (R20.58)."""

import datetime
import re
import sys
import threading
import time

import child
import judge


def mac():
    """vm_stat's totals: swapped in and out, paged out, in KiB."""
    out = child.run(["vm_stat"], capture_output=True, check=True).stdout
    page = int(re.search(r"page size of (\d+) bytes", out).group(1))
    total = {key: int(value) for key, value in re.findall(r"^(Swapins|Swapouts|Pageouts):\s+(\d+)\.", out, re.M)}
    if len(total) != 3:
        raise RuntimeError(f"vm_stat lacks a counter: {sorted(total)}")
    # xnu counts file write-back as Pageouts: logged, not judged.
    return {"in": total["Swapins"] * page // 1024, "out": total["Swapouts"] * page // 1024,
            "logged": {"pageouts KiB": total["Pageouts"] * page // 1024}}


class Pdh:
    """Windows' raw memory counters, by their English names."""

    # PDH_CSTATUS_VALID_DATA and PDH_CSTATUS_NEW_DATA; others are invalid.
    VALID = (0x0, 0x1)
    NAMES = {"out": r"\Memory\Pages Output/sec", "reads": r"\Memory\Page Reads/sec", "input": r"\Memory\Pages Input/sec"}

    def __init__(self):
        import ctypes
        import mmap

        class Raw(ctypes.Structure):
            # FILETIME is two DWORDs: 4-byte aligned.
            _fields_ = [("status", ctypes.c_ulong), ("low", ctypes.c_ulong), ("high", ctypes.c_ulong),
                        ("first", ctypes.c_longlong), ("second", ctypes.c_longlong), ("multi", ctypes.c_ulong)]

        self.ctypes, self.raw, self.page = ctypes, Raw, mmap.PAGESIZE
        self.pdh = ctypes.WinDLL("pdh")
        self.query = ctypes.c_void_p()
        self.check(self.pdh.PdhOpenQueryW(None, 0, ctypes.byref(self.query)), "opening a query")
        self.counters = {}
        for key, name in self.NAMES.items():
            handle = ctypes.c_void_p()
            self.check(self.pdh.PdhAddEnglishCounterW(self.query, name, 0, ctypes.byref(handle)), f"adding {name}")
            self.counters[key] = handle

    def check(self, status, what):
        if status:
            raise RuntimeError(f"PDH failed {what}: 0x{status & 0xFFFFFFFF:08X}")

    def read(self):
        """Pages written to the paging file, in KiB; reads logged."""
        self.check(self.pdh.PdhCollectQueryData(self.query), "collecting")
        pages = {}
        for key, handle in self.counters.items():
            raw = self.raw()
            self.check(self.pdh.PdhGetRawCounterValue(handle, None, self.ctypes.byref(raw)), f"reading {self.NAMES[key]}")
            if raw.status not in self.VALID:
                raise RuntimeError(f"PDH gave invalid data for {self.NAMES[key]}: status 0x{raw.status & 0xFFFFFFFF:08X}")
            pages[key] = raw.first
        # Inputs mix in file reads: swap-in is unmeasured.
        return {"in": None, "out": pages["out"] * self.page // 1024,
                "logged": {"page reads": pages["reads"], "pages input": pages["input"]}}


def rates(before, after, seconds):
    """(si KiB/s or None, so KiB, logged) between two readings."""
    logged = {key: value - before["logged"][key] for key, value in after["logged"].items()}
    si = None if after["in"] is None else (after["in"] - before["in"]) / seconds
    return si, after["out"] - before["out"], logged


class Sampler:
    """`read` each second on a thread; judge samples, trace lines."""

    def __init__(self, path, read):
        self.path, self.read, self.error = path, read, None
        self.samples, self.done = [], threading.Event()
        self.thread = threading.Thread(target=self.loop, daemon=True)
        self.thread.start()

    def loop(self):
        try:
            with open(self.path, "w", encoding="utf-8") as trace:
                last, began = self.read(), time.monotonic()
                tick = began
                while not self.done.is_set():
                    tick += judge.INTERVAL.total_seconds()
                    # Waits out the second, or a stop.
                    self.done.wait(max(0.0, tick - time.monotonic()))
                    now, reading = time.monotonic(), self.read()
                    si, so, logged = rates(last, reading, now - began)
                    stamp = datetime.datetime.now(datetime.timezone.utc)
                    self.samples.append((stamp, si, so))
                    extra = " ".join(f"{key}={value}" for key, value in logged.items())
                    said = "unmeasured" if si is None else f"{si:.1f}"
                    trace.write(f"{stamp.isoformat(timespec='milliseconds')} si={said} so={so} {extra}\n")
                    trace.flush()
                    last, began = reading, now
        except BaseException as error:
            self.error = error

    def alive(self):
        """Running; a reader's failure surfaces here."""
        if self.error is not None:
            raise RuntimeError(f"the paging counters failed: {self.error}") from self.error
        return self.thread.is_alive()

    def stop(self):
        self.done.set()
        self.thread.join()


def reader():
    """This system's reader; Linux has vmstat instead."""
    if sys.platform == "darwin":
        return mac
    if sys.platform == "win32":
        return Pdh().read
    raise RuntimeError(f"no paging counters for {sys.platform}")


class Counted:
    """bench's Paging on macOS and Windows: same calls, these counters."""

    suffix = "paging"

    def __init__(self, path):
        self.path = path
        self.sampler = Sampler(path, reader())
        self.until(lambda taken: taken)

    def taken(self):
        return list(self.sampler.samples)

    def until(self, enough):
        deadline = time.monotonic() + 10
        while not enough(self.taken()):
            if not self.sampler.alive() or time.monotonic() > deadline:
                raise RuntimeError("the paging counters gave no samples in 10 s")
            # Polls for the sampler's next second.
            time.sleep(0.2)

    def judged(self, windows):
        """Each window's verdict, once a sample covers the last."""
        if windows:
            self.until(lambda taken: taken and taken[-1][0] >= windows[-1][2])
        self.stop()
        # Windows' page reads mix in file reads: swap-in unjudged.
        return judge.verdicts(windows, self.taken(), swap_in=sys.platform != "win32") if windows else []

    def stop(self):
        self.sampler.stop()
        self.sampler.alive()
