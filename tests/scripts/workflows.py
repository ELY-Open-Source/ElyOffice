#!/usr/bin/env python3
"""release.yml: each platform alone, each through ci.sh's test gate."""
import os
import sys

try:
    import yaml
except ImportError:
    print("SKIP: no PyYAML")
    sys.exit(0)

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
with open(os.path.join(ROOT, ".github", "workflows", "release.yml"), encoding="utf-8") as f:
    jobs = yaml.safe_load(f)["jobs"]
failed = []
for name, job in jobs.items():
    if "needs" in job:
        failed.append(f"{name} waits for {job['needs']}: one platform's failure stops another")
    runs = " ".join(step.get("run", "") for step in job["steps"])
    if "scripts/ci.sh" not in runs:
        failed.append(f"{name} releases without scripts/ci.sh and its tests")
with open(os.path.join(ROOT, "scripts", "ci.sh"), encoding="utf-8") as f:
    ci = f.read()
gate, package = ci.find("\nscripts/release-tests.sh\n"), ci.find("\ndist=$root/dist\n")
if gate < 0 or package < 0 or gate > package:
    failed.append("ci.sh does not run scripts/release-tests.sh before packaging")
if failed:
    sys.exit("FAIL: " + "; ".join(failed))
print(f"PASS: {len(jobs)} release jobs stand alone and pass the test gate")
