#!/usr/bin/env bash
# check.sh counts lines in code only, and reports every rule.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
git init -q "$work"
git -C "$work" config core.autocrlf false
cp "$root/.gitattributes" "$work/"
lines() { mkdir -p "$(dirname "$work/$2")" && seq "$1" | sed 's/^/let a = 1; \/\/ /' >"$work/$2"; }
check() {
  git -C "$work" add -A
  said=$(bash "$root/scripts/check.sh" "$work" 2>&1)
}
lines 500 "src/full.rs"
lines 600 "vendor/big.rs"
lines 600 "docs/notes.md"
lines 600 "src/with space/data.txt"
lines 600 "site/video/vendor/big.js"
# The same lines, the last without its LF.
bare() { lines "$1" "$2" && printf '%s' "$(cat "$work/$2")" >"$work/$2"; }
bare 500 "src/bare.rs"
check || fail "files within the rules failed: $said"
bare 501 "src/bare.rs"
! check || fail "501 lines without a final LF passed"
case $said in *"src/bare.rs: 501 lines"*) ;; *) fail "the unterminated file was not named: $said" ;; esac
bare 500 "src/bare.rs"
lines 501 "src/with space/long.rs"
! check || fail "a 501-line .rs passed"
case $said in *"src/with space/long.rs: 501 lines"*) ;; *) fail "the long file was not named: $said" ;; esac
rm "$work/src/with space/long.rs"
head -c 30000 /dev/zero | tr '\0' 'a' >"$work/agents.md"
check || fail "a 30000-byte agents.md failed: $said"
printf a >>"$work/agents.md"
! check || fail "a 30001-byte agents.md passed"
case $said in *"agents.md is 30001 bytes"*) ;; *) fail "the long guide was not named: $said" ;; esac
rm "$work/agents.md"
mkdir -p "$work/apps/ely-doc/src"
printf 'Banner::new("doc-fonts", a);\n' >"$work/apps/ely-doc/src/chrome.rs"
check || fail "the font banner failed: $said"
printf 'Banner::new(\n    "mark-outside", a);\n' >"$work/src/more.rs"
! check || fail "a second banner passed"
case $said in *"src/more.rs: mark-outside"*) ;; *) fail "the second banner was not named: $said" ;; esac
rm "$work/src/more.rs"
printf 'Banner::new("doc-fonts", a);\nBanner::new("doc-fonts", b);\n' >"$work/apps/ely-doc/src/chrome.rs"
! check || fail "a second font banner passed"
printf 'Banner::new("doc-fonts", a);\n' >"$work/apps/ely-doc/src/chrome.rs"
printf 'Banner :: new\n    ("mark-large", a);\n' >"$work/src/more.rs"
! check || fail "a spaced banner passed"
case $said in *"src/more.rs: mark-large"*) ;; *) fail "the spaced banner was not named: $said" ;; esac
rm "$work/src/more.rs"
mkdir -p "$work/crates/a"
printf '// One two three four five six seven eight nine ten eleven.\n' >"$work/crates/a/a.rs"
! check || fail "a long comment passed check.sh"
# Ignored inside another checkout: git lists nothing there.
mkdir -p "$work/scratch/crates/a"
printf 'scratch/\n' >"$work/.gitignore"
cp "$work/crates/a/a.rs" "$work/scratch/crates/a/a.rs"
! said=$(bash "$root/scripts/check.sh" "$work/scratch" 2>&1) || fail "an ignored folder passed with its long comment"
case $said in *"is not the top of its git checkout but scratch/"*) ;; *) fail "the ignored folder was not named: $said" ;; esac
# A tracked folder below the top: still not the top.
! said=$(bash "$root/scripts/check.sh" "$work/crates" 2>&1) || fail "a tracked subfolder passed"
case $said in *"is not the top of its git checkout but crates/"*) ;; *) fail "the subfolder was not named: $said" ;; esac
# No checkout above at all.
loose=$(mktemp -d)
trap 'rm -rf "$work" "$loose"' EXIT
cp -R "$work/scratch/crates" "$loose/"
! said=$(GIT_CEILING_DIRECTORIES=$(dirname "$loose") bash "$root/scripts/check.sh" "$loose" 2>&1) || fail "a tree outside git passed"
case $said in *"is not in a git checkout"*) ;; *) fail "the loose tree was not named: $said" ;; esac
# A checkout that tracks nothing: its long comment goes unread.
blank=$loose/blank
git init -q "$blank"
cp -R "$work/scratch/crates" "$blank/"
! said=$(bash "$root/scripts/check.sh" "$blank" 2>&1) || fail "a checkout tracking nothing passed"
case $said in *"tracks no files"*) ;; *) fail "the empty checkout was not named: $said" ;; esac
# A real clone passes, and its rules ran.
origin=$loose/origin
git init -q "$origin"
git -C "$origin" config core.autocrlf false
printf '// Short.\nlet a = 1;\n' >"$origin/a.rs"
cp "$root/.gitattributes" "$origin/"
git -C "$origin" add a.rs .gitattributes
git -C "$origin" -c user.name=test -c user.email=test@localhost -c commit.gpgsign=false commit -q -m one
git clone -q "$origin" "$loose/clone"
said=$(bash "$root/scripts/check.sh" "$loose/clone" 2>&1) || fail "a real clone failed: $said"
case $said in *"every rule holds"*) ;; *) fail "the clone's rules did not run: $said" ;; esac
echo "ok: check rules"
