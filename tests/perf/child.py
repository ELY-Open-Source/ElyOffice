"""Child output as strict UTF-8, naming the command."""
import subprocess


def named(args):
    return " ".join(map(str, args[:4]))


def run(args, **kw):
    """subprocess.run with its output as UTF-8 text."""
    try:
        return subprocess.run(args, encoding="utf-8", errors="strict", **kw)
    except UnicodeDecodeError as bad:
        raise RuntimeError(f"{named(args)}: output is not UTF-8: {bad}") from bad


def line(process):
    """One line from a child opened with encoding="utf-8"."""
    try:
        return process.stdout.readline()
    except UnicodeDecodeError as bad:
        raise RuntimeError(f"{named(process.args)}: output is not UTF-8: {bad}") from bad
