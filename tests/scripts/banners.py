#!/usr/bin/env python3
"""Only ElyDoc's missing fonts is a banner."""
import re
import subprocess
import sys
from pathlib import Path

root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[2]
ALLOWED = ("apps/ely-doc/src/chrome.rs", "doc-fonts")
BANNER = re.compile(r'\bBanner\s*::\s*new\s*\(\s*(?:"([^"]*)")?')

files = subprocess.run(
    ["git", "-C", str(root), "ls-files", "-z", "--", "*.rs", ":(exclude,glob)**/vendor/**", ":!reference"],
    check=True,
    capture_output=True,
).stdout.decode().split("\0")
found = []
for name in filter(None, files):
    text = (root / name).read_text(encoding="utf-8")
    found += [(name, match.group(1) or "an unnamed banner") for match in BANNER.finditer(text)]
# The font banner may stand once.
extra = sorted(found)
if ALLOWED in extra:
    extra.remove(ALLOWED)
for name, banner in extra:
    print(f"FAIL: {name}: {banner}: only doc-fonts may be a banner")
if extra:
    sys.exit(1)
print("PASS: the only banner is ElyDoc's missing fonts")
