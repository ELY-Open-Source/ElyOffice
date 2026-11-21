"""Child output as strict UTF-8, naming the command."""
import subprocess


def named(args):
    return " ".join(map(str, args[:4]))


def run(args, **kw):
    """subprocess.run with its output as UTF-8 text."""
    # Windows decodes in reader threads, which swallow the error.
    out = subprocess.run(args, **kw)
    out.stdout = None if out.stdout is None else decoded(args, out.stdout)
    out.stderr = None if out.stderr is None else decoded(args, out.stderr)
    return out


def decoded(args, raw):
    """Bytes a child wrote, as strict UTF-8, newlines as text=True."""
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as bad:
        raise RuntimeError(f"{named(args)}: output is not UTF-8: {bad}") from bad
    return text.replace("\r\n", "\n").replace("\r", "\n")


def logged(path, since=0):
    """Complete lines from byte `since`; a cut tail waits."""
    with open(path, "rb") as log:
        log.seek(since)
        raw = log.read()
    try:
        return raw[:raw.rfind(b"\n") + 1].decode("utf-8").splitlines()
    except UnicodeDecodeError as bad:
        raise RuntimeError(f"{path}: a logged line is not UTF-8: {bad}") from bad


def line(process):
    """One line from a child opened with encoding="utf-8"."""
    try:
        return process.stdout.readline()
    except UnicodeDecodeError as bad:
        raise RuntimeError(f"{named(process.args)}: output is not UTF-8: {bad}") from bad
