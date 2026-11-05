#!/usr/bin/env bash
# lo-tarballs.tsv: download.lst's hashes; covers this host's LibreOffice build.
set -euo pipefail
# comm and sort compare bytes.
export LC_ALL=C

root=$(cd "$(dirname "$0")/../.." && pwd)
list=$root/packaging/lo-tarballs.tsv
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
command -v python3 >/dev/null || {
  echo "SKIP: no python3"
  exit 0
}

lst=$root/reference/libreoffice-core/download.lst
if [ -f "$lst" ]; then
  bad=$(python3 - "$lst" "$list" <<'PY'
import re, sys

vals = {}
for line in open(sys.argv[1], encoding="utf-8"):
    m = re.fullmatch(r"([A-Z0-9_]+) := (\S+)", line.strip())
    if m:
        vals[m[1]] = m[2]


def expand(value):
    return re.sub(r"\$\(([A-Z0-9_]+)\)", lambda m: expand(vals[m[1]]), value)


# Makefile.fetch's fetch_Download__subst_var.
pinned = {}
for var, value in vals.items():
    m = re.fullmatch(r"(.+)_(TARBALL|TTF|JAR|DLL|PACK)", var)
    if m:
        pinned[expand(value)] = expand(vals[m[1] + "_SHA256SUM"])
seen = set()
for n, row in enumerate(open(sys.argv[2], encoding="utf-8", newline=""), 1):
    fields = row.rstrip("\n").split("\t")
    if len(fields) != 3:
        print(f"line {n} is not name, sha256, URL: {row!r}")
        continue
    name, sha, url = fields
    if name in seen:
        print(f"{name} is listed twice")
    seen.add(name)
    if pinned.get(name) != sha:
        print(f"{name}: {sha} is not download.lst's {pinned.get(name)}")
    if url not in [f"https://dev-www.libreoffice.org/{d}/{name}" for d in ("src", "extern")]:
        print(f"{name}: {url} is not LibreOffice's mirror")
PY
  ) || fail "download.lst unreadable: $bad"
  [ -z "$bad" ] || fail "$bad"
else
  echo "SKIP: no reference/libreoffice-core; run scripts/fetch-references.sh libreoffice-core"
fi

src=$root/engines/lo-src
if [ ! -f "$src/config_host.mk" ]; then
  echo "SKIP: no configured engines/lo-src"
  exit 0
fi
case $(uname) in
  MINGW* | MSYS*)
    cygwin=$(cygpath -u "${ELY_CYGWIN:-C:/cygwin64}")
    got=$("$cygwin/bin/bash.exe" -lc "bash '$(cygpath -m "$root/scripts/lo-tarballs.sh")' '$(cygpath -m "$src")'")
    ;;
  *) got=$("$root/scripts/lo-tarballs.sh" "$src") ;;
esac
[ -n "$got" ] || fail "the build in $src fetches nothing"
missing=$(comm -23 <(printf '%s\n' "$got" | sort) <(sort "$list"))
[ -z "$missing" ] || fail "packaging/lo-tarballs.tsv lacks what engines/lo-src fetches: $missing"
echo "lo-tarballs: pinned hashes; all $(grep -c . <<<"$got") of this build's fetches listed"
