#!/usr/bin/env python3
"""Workflows: releases gated and apart; pull requests off our machines."""
import copy
import os
import re
import sys

try:
    import yaml
except ImportError:
    print("SKIP: no PyYAML")
    sys.exit(0)

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
# The one runner pull requests may use: GitHub's, by name.
HOSTED = "ubuntu-24.04"
# GitHub's free arm64 host, for Linux arm64 releases.
ARM = "ubuntu-24.04-arm"
BASE = "${{ github.event.pull_request.base.sha || github.event.before }}"


def read(*parts):
    with open(os.path.join(ROOT, *parts), encoding="utf-8") as f:
        return f.read()


def runs(job):
    return " ".join(step.get("run", "") for step in job["steps"])


# Only a plain line installs: no shell syntax to misread.
INSTALL = re.compile(r"\s*(?:sudo\s+)?apt(?:-get)?\s+install((?:\s+-[a-z]+)*)((?:\s+[a-z0-9][a-z0-9.+-]*)+)\s*")


def installed(job):
    """Packages on plain `[sudo] apt-get install` lines."""
    packages = set()
    for step in job["steps"]:
        for line in step.get("run", "").splitlines():
            match = INSTALL.fullmatch(line)
            if match:
                packages.update(match.group(2).split())
    return packages


def pr_faults(jobs, libraries):
    faults = []
    for name, job in jobs.items():
        if job.get("runs-on") != HOSTED:
            faults.append(f"pr {name} runs on {job.get('runs-on')!r}, not GitHub's {HOSTED}")
        missing = sorted(set(libraries) - installed(job))
        if missing:
            faults.append(f"pr {name} lacks README's {missing} on a plain apt-get install line")
        env = job.get("env", {})
        if env.get("ELY_REQUIRE_ENGINES") != "1":
            faults.append(f"pr {name} lets missing engines skip")
        if env.get("PANICS_BASE") != BASE or "panics.py" not in runs(job):
            faults.append(f"pr {name} counts panics against the change itself")
    return faults


failed = []
jobs = yaml.safe_load(read(".github", "workflows", "release.yml"))["jobs"]
for name, job in jobs.items():
    if "needs" in job:
        failed.append(f"release {name} waits for {job['needs']}: one platform's failure stops another")
    if "scripts/ci.sh" not in runs(job):
        failed.append(f"release {name} releases without scripts/ci.sh and its tests")
# macOS jobs start ci.sh under bash 5, never 3.2.
MAC_CI = {
    "macos": '/opt/homebrew/bin/bash "$dir/scripts/ci.sh"',
    "macos-intel": 'arch -x86_64 /Users/zacharyzhang/ely-share/bash-x86_64/bin/bash "$dir/scripts/ci.sh"',
}


def mac_faults(jobs):
    faults = []
    for name, call in MAC_CI.items():
        lines = [line.strip() for line in runs(jobs[name]).splitlines()]
        if not any(line.startswith(call + " ") for line in lines):
            faults.append(f"release {name} starts ci.sh without its bash 5 ({call})")
    return faults


failed += mac_faults(jobs)
for name, call in MAC_CI.items():
    bash = call.split(' "$dir')[0]
    for how, bad_call in {
        "under /bin/bash": call.replace(bash.split()[-1], "/bin/bash"),
        "by its shebang": '"$dir/scripts/ci.sh"',
        "only in a comment": f"true\n# {call}",
    }.items():
        bad = copy.deepcopy(jobs)
        for step in bad[name]["steps"]:
            if "run" in step:
                step["run"] = step["run"].replace(call, bad_call)
        if bad == jobs:
            failed.append("a counterexample changed nothing")
        if not mac_faults(bad):
            failed.append(f"release {name} starting ci.sh {how} passes")


# Linux x86_64 stays in WSL; arm64 on GitHub's arm64 host.
LINUX_CI = {
    "linux": (["self-hosted", "wsl"], '"$dir/scripts/ci.sh" linux "${{ github.ref_name }}"'),
    "linux-arm64": (ARM, 'bash scripts/ci.sh linux-arm64 "${{ github.ref_name }}"'),
}


def linux_faults(jobs):
    faults = []
    for name, (runner, call) in LINUX_CI.items():
        job = jobs.get(name)
        if job is None:
            faults.append(f"release {name} is missing")
            continue
        if job.get("runs-on") != runner:
            faults.append(f"release {name} runs on {job.get('runs-on')!r}, not {runner!r}")
        if not any(line.strip().endswith(call) for line in runs(job).splitlines()):
            faults.append(f"release {name} does not start ci.sh for its platform ({call})")
    return faults


def linux_run(name, old, new):
    """The jobs with one Linux job's ci.sh call changed."""
    bad = copy.deepcopy(jobs)
    for step in bad[name]["steps"]:
        if "run" in step:
            step["run"] = step["run"].replace(old, new)
    if bad == jobs:
        failed.append("a Linux counterexample changed nothing")
    return bad


failed += linux_faults(jobs)
# Each must be caught: labels, platform, swap, shebang.
bad = copy.deepcopy(jobs)
bad["linux-arm64"]["runs-on"] = ["self-hosted", "wsl"]
if not linux_faults(bad):
    failed.append("release linux-arm64 on a self-hosted runner passes")
if not linux_faults(linux_run("linux-arm64", "linux-arm64", "linux")):
    failed.append("release linux-arm64 as x86_64 passes")
bad = copy.deepcopy(jobs)
bad["linux"], bad["linux-arm64"] = bad["linux-arm64"], bad["linux"]
if not linux_faults(bad):
    failed.append("release Linux jobs swapped pass")
if not linux_faults(linux_run("linux-arm64", "bash scripts/ci.sh", '"$GITHUB_WORKSPACE/scripts/ci.sh"')):
    failed.append("release linux-arm64 started by its shebang passes")

ci = read("scripts", "ci.sh")
gate, package = ci.find("\nscripts/release-tests.sh\n"), ci.find("\ndist=$root/dist\n")
if gate < 0 or package < 0 or gate > package:
    failed.append("ci.sh does not run scripts/release-tests.sh before packaging")

pr = yaml.safe_load(read(".github", "workflows", "pr.yml"))["jobs"]
libraries = re.findall(r"\blib\S+-dev\b", re.search(r"^sudo apt install .*$", read("README.md"), re.M).group(0))
failed += pr_faults(pr, libraries)

# The arm64 host needs GStreamer for the gate.
GSTREAMER = ["libgstreamer-plugins-base1.0-0", "gstreamer1.0-plugins-base", "gstreamer1.0-plugins-good", "gstreamer1.0-libav"]
GATE_DEPS = set(libraries) | set(GSTREAMER)


def host_faults(jobs):
    """The arm64 job installs the gate's host packages."""
    job = jobs.get("linux-arm64")
    if job is None:
        return ["release linux-arm64 is missing"]
    missing = sorted(GATE_DEPS - installed(job))
    if missing:
        return [f"release linux-arm64 lacks {missing} on a plain apt-get install line"]
    return []


failed += host_faults(jobs)
bad = copy.deepcopy(jobs)
for step in bad["linux-arm64"]["steps"]:
    if "run" in step:
        step["run"] = step["run"].replace("gstreamer1.0-libav", "# gstreamer1.0-libav")
if not host_faults(bad):
    failed.append("GStreamer only in a comment passes")

# Each must be caught: our labels, expressions, groups, comments.
name = next(iter(pr))
for runner in ["ely-windows", ["self-hosted"], "${{ matrix.os }}", {"group": "ours"}, ["ubuntu-24.04", "ely"]]:
    bad = copy.deepcopy(pr)
    bad[name]["runs-on"] = runner
    if not pr_faults(bad, libraries):
        failed.append(f"runs-on {runner!r} passes")
lib = libraries[0]
moves = {
    "only in a comment": lambda run: run.replace(f" {lib}", "") + f"\n# {lib}",
    "after &&": lambda run: re.sub(f" {lib}(.*)", rf"\1 && echo {lib}", run),
    "after ;": lambda run: re.sub(f" {lib}(.*)", rf"\1; echo {lib}", run),
    "piped on": lambda run: re.sub(f" {lib}(.*)", rf"\1 | tee {lib}", run),
    "in an echoed install": lambda run: re.sub(f"^(.*install.* {lib}.*)$", r"echo \1", run, flags=re.M),
    "behind a quoted ;": lambda run: re.sub(f"^(.*install.* {lib}.*)$", r'echo ";" \1', run, flags=re.M),
    "under sudo -l": lambda run: re.sub(f"^(\\s*)sudo (.*install.* {lib}.*)$", r"\1sudo -l \2", run, flags=re.M),
}


def moved(move):
    """The pr jobs with `lib`'s install line changed by `move`."""
    bad = copy.deepcopy(pr)
    for step in bad[name]["steps"]:
        if "run" in step and lib in step["run"]:
            step["run"] = move(step["run"])
    if bad == pr:
        failed.append("a counterexample changed nothing")
    return bad


for how, move in moves.items():
    if not pr_faults(moved(move), libraries):
        failed.append(f"{lib} {how} passes")
# apt itself installs as apt-get does.
if pr_faults(moved(lambda run: run.replace("apt-get install", "apt install")), libraries):
    failed.append("apt install is taken for missing libraries")
bad = copy.deepcopy(pr)
del bad[name]["env"]["PANICS_BASE"]
if not pr_faults(bad, libraries):
    failed.append("panics counted against HEAD passes")

if failed:
    sys.exit("FAIL: " + "; ".join(failed))
print(f"PASS: {len(jobs)} release jobs stand alone behind the test gate, macOS under bash 5, Linux arm64 on {ARM}; "
      f"{len(pr)} pr jobs stay on {HOSTED}, install README's libraries and count panics against the base")
