#!/usr/bin/env python3
"""Workflows: releases gated and apart; pull requests off our machines."""
import os
import re
import sys

try:
    import yaml
except ImportError:
    print("SKIP: no PyYAML")
    sys.exit(0)

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))


def read(*parts):
    with open(os.path.join(ROOT, *parts), encoding="utf-8") as f:
        return f.read()


def runs(job):
    return " ".join(step.get("run", "") for step in job["steps"])


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
for name, job in pr.items():
    if "self-hosted" in str(job["runs-on"]):
        failed.append(f"pr {name} runs pull requests on a self-hosted machine")
    missing = [lib for lib in libraries if lib not in runs(job)]
    if missing:
        failed.append(f"pr {name} lacks README's {missing}")
    if job.get("env", {}).get("ELY_REQUIRE_ENGINES") != "1":
        failed.append(f"pr {name} lets missing engines skip")
if failed:
    sys.exit("FAIL: " + "; ".join(failed))
print(f"PASS: {len(jobs)} release jobs stand alone behind the test gate; {len(pr)} pr jobs stay on GitHub's machines")
