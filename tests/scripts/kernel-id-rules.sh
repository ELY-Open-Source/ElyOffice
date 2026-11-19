#!/usr/bin/env bash
# kernel-id.sh: same content same id, any change a new one.
set -euo pipefail

# Git Bash on Windows: real links, not copies.
export MSYS=winsymlinks:nativestrict
root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
id() {
  "$root/scripts/kernel-id.sh" "$work/kernel" "$work/kernel/program/ely-kernel-id" >/dev/null
  cat "$work/kernel/program/ely-kernel-id"
}

mkdir -p "$work/kernel/program"
echo first >"$work/kernel/program/first.so"
echo second >"$work/kernel/program/second.so"
ln -s first.so "$work/kernel/program/loaded.so"
start=$(id)
[ ${#start} -eq 64 ] || fail "the id is not 64 characters: $start"
[ "$(id)" = "$start" ] || fail "the id counted itself"
ln -sfn second.so "$work/kernel/program/loaded.so"
moved=$(id)
[ "$moved" != "$start" ] || fail "a link's new target kept the id"
echo changed >"$work/kernel/program/second.so"
[ "$(id)" != "$moved" ] || fail "a changed file kept the id"
if "$root/scripts/kernel-id.sh" "$work/kernel" "$work/other-name" 2>/dev/null; then
  fail "an id file of another name was written"
fi

# Each failure leaves no id, not even the old one.
echo stale >"$work/kernel/program/ely-kernel-id"
if "$root/scripts/kernel-id.sh" "$work/kernel/none" "$work/kernel/program/ely-kernel-id" 2>/dev/null; then
  fail "a missing kernel still succeeded"
fi
[ ! -e "$work/kernel/program/ely-kernel-id" ] || fail "a missing kernel left an id"
if "$root/scripts/kernel-id.sh" "$work/kernel" "$work/kernel/program/ely-kernel-id" >&- 2>/dev/null; then
  fail "a closed stdout still succeeded"
fi
[ ! -e "$work/kernel/program/ely-kernel-id" ] || fail "a closed stdout left an id"
ls "$work/kernel/program" | grep -q part && fail "a failed run left its part file"
refused() {
  echo stale >"$work/kernel/program/ely-kernel-id"
  if PATH="$work/bin:$PATH" "$root/scripts/kernel-id.sh" "$work/kernel" "$work/kernel/program/ely-kernel-id" 2>/dev/null; then
    fail "$1 still succeeded"
  fi
  [ ! -e "$work/kernel/program/ely-kernel-id" ] || fail "$1 left an id"
  rm -rf "$work/bin"
}
if command -v sha256sum >/dev/null; then
  real=$(command -v sha256sum)
else
  real="$(command -v shasum) -a 256"
fi
# Hashes the others, fails on second.so, like a real one.
mkdir "$work/bin"
cat >"$work/bin/sha256sum" <<STUB
#!/usr/bin/env bash
args=() bad=
for a; do case \$a in */second.so) bad=\$a ;; *) args+=("\$a") ;; esac; done
$real "\${args[@]}" || exit
[ -z "\$bad" ] || { echo "sha256sum: \$bad: Permission denied" >&2; exit 1; }
STUB
chmod +x "$work/bin/sha256sum"
refused "one unreadable file"
mkdir "$work/bin"
real=$(command -v readlink)
cat >"$work/bin/readlink" <<STUB
#!/usr/bin/env bash
case \$1 in */loaded.so) echo "readlink: \$1: Permission denied" >&2; exit 1 ;; esac
exec $real "\$@"
STUB
chmod +x "$work/bin/readlink"
refused "one unreadable link"
# What the build prunes, back after a bare make.
for extra in share/extensions/dict-fr share/extensions/dict-en/Lightproof.py; do
  mkdir -p "$work/kernel/$(dirname "$extra")"
  echo extra >"$work/kernel/$extra"
  echo stale >"$work/kernel/program/ely-kernel-id"
  if out=$("$root/scripts/kernel-id.sh" "$work/kernel" "$work/kernel/program/ely-kernel-id" 2>&1); then
    fail "a kernel holding $extra got an id"
  fi
  grep -q "$extra" <<<"$out" || fail "the refusal does not name $extra: $out"
  [ ! -e "$work/kernel/program/ely-kernel-id" ] || fail "$extra left an id"
  rm -rf "$work/kernel/share"
done
before=$(id)
mkdir -p "$work/kernel/share/extensions/dict-en"
echo words >"$work/kernel/share/extensions/dict-en/en_US.dic"
[ "$(id)" != "$before" ] || fail "the kept dictionary is not counted"
rm "$work/kernel/program/"*.so
rm -rf "$work/kernel/share"
mkdir "$work/bin"
refused "an empty kernel"
echo "kernel-id-rules: files and links change the id; any failure or pruned extra leaves none"
