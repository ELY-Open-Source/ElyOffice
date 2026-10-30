#!/usr/bin/env python3
"""Workflows: releases gated and apart; pull requests off our machines."""
import copy
import os
import re
import shlex
import sys

try:
    import yaml
except ImportError:
    print("SKIP: no PyYAML")
    sys.exit(0)

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
# The one runner pull requests may use: GitHub's, by name.
HOSTED = "ubuntu-24.04"
BASE = "${{ github.event.pull_request.base.sha || github.event.before }}"


def read(*parts):
    with open(os.path.join(ROOT, *parts), encoding="utf-8") as f:
        return f.read()


def runs(job):
    return " ".join(step.get("run", "") for step in job["steps"])


def installed(job):
    """Packages named to apt install, comments left out."""
    packages = set()
    for step in job["steps"]:
        for line in step.get("run", "").splitlines():
            words = shlex.split(line, comments=True)
            for at in range(len(words) - 1):
                if words[at] in ("apt", "apt-get") and words[at + 1] == "install":
                    packages.update(word for word in words[at + 2:] if not word.startswith("-"))
    return packages


def pr_faults(jobs, libraries):
    faults = []
    for name, job in jobs.items():
        if job.get("runs-on") != HOSTED:
            faults.append(f"pr {name} runs on {job.get('runs-on')!r}, not GitHub's {HOSTED}")
        missing = sorted(set(libraries) - installed(job))
        if missing:
            faults.append(f"pr {name} lacks README's {missing}")
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
ci = read("scripts", "ci.sh")
gate, package = ci.find("\nscripts/release-tests.sh\n"), ci.find("\ndist=$root/dist\n")
if gate < 0 or package < 0 or gate > package:
    failed.append("ci.sh does not run scripts/release-tests.sh before packaging")

pr = yaml.safe_load(read(".github", "workflows", "pr.yml"))["jobs"]
libraries = re.findall(r"\blib\S+-dev\b", re.search(r"^sudo apt install .*$", read("README.md"), re.M).group(0))
failed += pr_faults(pr, libraries)

# Each must be caught: our labels, expressions, groups, comments.
name = next(iter(pr))
for runner in ["ely-windows", ["self-hosted"], "${{ matrix.os }}", {"group": "ours"}, ["ubuntu-24.04", "ely"]]:
    bad = copy.deepcopy(pr)
    bad[name]["runs-on"] = runner
    if not pr_faults(bad, libraries):
        failed.append(f"runs-on {runner!r} passes")
bad = copy.deepcopy(pr)
for step in bad[name]["steps"]:
    if "run" in step:
        step["run"] = step["run"].replace(f" {libraries[0]}", "") + f"\n# {libraries[0]}"
if not pr_faults(bad, libraries):
    failed.append(f"{libraries[0]} named only in a comment passes")
bad = copy.deepcopy(pr)
del bad[name]["env"]["PANICS_BASE"]
if not pr_faults(bad, libraries):
    failed.append("panics counted against HEAD passes")

if failed:
    sys.exit("FAIL: " + "; ".join(failed))
print(f"PASS: {len(jobs)} release jobs stand alone behind the test gate; {len(pr)} pr jobs stay on {HOSTED}, "
      "install README's libraries and count panics against the base")
