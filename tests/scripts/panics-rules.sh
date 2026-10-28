#!/usr/bin/env bash
# panics.py holds sites to the list; counts never rise.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
py=python3
command -v "$py" >/dev/null || py=python
git init -q "$work"
git -C "$work" config core.autocrlf false
git -C "$work" config user.name ely
git -C "$work" config user.email ely@example.com
git -C "$work" config commit.gpgsign false
mkdir -p "$work/src" "$work/scripts"
printf '[package]\nname = "probe"\nversion = "0.1.0"\nedition = "2024"\n\n[workspace]\n' >"$work/Cargo.toml"
printf 'fn main() {\n    let _ = std::env::var("CARGO").expect("built");\n}\n' >"$work/build.rs"
code() {
  printf 'pub fn given(x: Option<u8>) -> u8 {\n    x.expect("given")\n}\n%s\n' "$1" >"$work/src/lib.rs"
  printf '#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {\n        Some(1).unwrap();\n    }\n}\n' >>"$work/src/lib.rs"
}
row=$'src/lib.rs\texpect_used\tx.expect("given")\tlinux,macos,windows\tgiven'
list() { printf 'file\tkind\texpression\thosts\treason\n%s\n' "$@" >"$work/scripts/panics.tsv"; }
run() { said=$("$py" "$root/scripts/panics.py" "$work" 2>&1); }
commit() { git -C "$work" add -A && git -C "$work" commit -qm "$1"; }

code ""
list "$row"
commit "one site"
run || fail "the listed site failed: $said"
# A new site, not listed.
code 'pub fn taken(x: Option<u8>) -> u8 { x.unwrap() }'
! run || fail "an unlisted unwrap passed"
case $said in *"new: src/lib.rs: unwrap_used x.unwrap()"*) ;; *) fail "the new site was not named: $said" ;; esac
# Listed, it still raises the count over HEAD.
list "$row" $'src/lib.rs\tunwrap_used\tx.unwrap()\tlinux,macos,windows\ttaken'
! run || fail "a rise over HEAD passed"
case $said in *"rise: unwrap_used 0 at HEAD, 1 now"*) ;; *) fail "the rise was not named: $said" ;; esac
# A listed site that is gone fails; so does a row with no reason.
code ""
list "$row" $'src/lib.rs\texpect_used\tgone.expect("gone")\tlinux,macos,windows\tgone'
! run || fail "a gone site passed"
case $said in *"gone on "*"gone.expect"*) ;; *) fail "the gone site was not named: $said" ;; esac
list $'src/lib.rs\texpect_used\tx.expect("given")\tlinux,macos,windows\t '
! run || fail "a row without a reason passed"
echo "ok: panics rules"
