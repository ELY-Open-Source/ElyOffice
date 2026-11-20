"""Windows process trees: a parent id outlives its process."""
import ctypes

# Win32 error codes.
NO_SUCH_PROCESS, ACCESS_DENIED, NO_MORE_FILES = 87, 5, 18
STILL_ACTIVE, WAIT_OBJECT_0 = 259, 0


def failed(what, pid):
    return RuntimeError(f"{what} for process {pid} failed: {ctypes.WinError(ctypes.get_last_error())}")


def processes(k32):
    """(pid, parent) for every process, from one snapshot."""
    return [(pid, parent) for pid, parent, _ in listing(k32)]


def image(k32, pid):
    """`pid`'s executable name, or None if it is gone."""
    return next((exe for each, _, exe in listing(k32) if each == pid), None)


def listing(k32):
    """(pid, parent, exe) for every process, from one snapshot."""

    class Entry(ctypes.Structure):
        _fields_ = [("size", ctypes.c_ulong), ("usage", ctypes.c_ulong), ("pid", ctypes.c_ulong),
                    ("heap", ctypes.c_size_t), ("module", ctypes.c_ulong), ("threads", ctypes.c_ulong),
                    ("parent", ctypes.c_ulong), ("base", ctypes.c_long), ("flags", ctypes.c_ulong),
                    ("exe", ctypes.c_wchar * 260)]

    k32.CreateToolhelp32Snapshot.restype = ctypes.c_void_p
    snap = k32.CreateToolhelp32Snapshot(2, 0)
    if snap in (None, ctypes.c_void_p(-1).value):
        raise failed("a process snapshot", 0)
    entry, pairs = Entry(), []
    entry.size = ctypes.sizeof(Entry)
    try:
        more = k32.Process32FirstW(ctypes.c_void_p(snap), ctypes.byref(entry))
        while more:
            pairs.append((entry.pid, entry.parent, entry.exe))
            more = k32.Process32NextW(ctypes.c_void_p(snap), ctypes.byref(entry))
        # Only ERROR_NO_MORE_FILES ends a whole list.
        if ctypes.get_last_error() != NO_MORE_FILES:
            raise failed(f"listing processes after {len(pairs)}", 0)
    finally:
        k32.CloseHandle(ctypes.c_void_p(snap))
    return pairs


def created(k32, pid):
    """Start time, 100 ns ticks; None if gone or foreign."""
    k32.OpenProcess.restype = ctypes.c_void_p
    # PROCESS_QUERY_LIMITED_INFORMATION
    handle = k32.OpenProcess(0x1000, False, pid)
    if not handle:
        # Our own children always open; others cannot be killed either.
        if ctypes.get_last_error() in (NO_SUCH_PROCESS, ACCESS_DENIED):
            return None
        raise failed("opening", pid)
    try:
        born, ended, kernel, user = (ctypes.c_ulonglong() for _ in range(4))
        if not k32.GetProcessTimes(ctypes.c_void_p(handle), *map(ctypes.byref, (born, ended, kernel, user))):
            raise failed("reading the start time", pid)
        return born.value
    finally:
        k32.CloseHandle(ctypes.c_void_p(handle))


def gone(k32, pid):
    """True if `pid` has ended or is ending."""
    k32.OpenProcess.restype = ctypes.c_void_p
    # SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION
    handle = k32.OpenProcess(0x00100000 | 0x1000, False, pid)
    if not handle:
        if ctypes.get_last_error() == NO_SUCH_PROCESS:
            return True
        raise failed("opening", pid)
    try:
        if k32.WaitForSingleObject(ctypes.c_void_p(handle), 0) == WAIT_OBJECT_0:
            return True
        code = ctypes.c_ulong()
        if not k32.GetExitCodeProcess(ctypes.c_void_p(handle), ctypes.byref(code)):
            raise failed("reading the exit code", pid)
        # TerminateProcess refuses a process mid-exit (R20.95).
        return code.value != STILL_ACTIVE
    finally:
        k32.CloseHandle(ctypes.c_void_p(handle))


def descendants(root, pairs, born):
    """`root` and the processes it started, younger than their parents."""
    ours, grew, when = {root}, True, {}

    def started(pid):
        if pid not in when:
            when[pid] = born(pid)
        return when[pid]

    while grew:
        grew = False
        for pid, ppid in pairs:
            if ppid in ours and pid not in ours:
                child, parent = started(pid), started(ppid)
                # wininit's parent pid was reused by an app (R20.30).
                if child is not None and parent is not None and child >= parent:
                    ours.add(pid)
                    grew = True
    return ours
