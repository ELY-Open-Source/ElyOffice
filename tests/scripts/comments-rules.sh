#!/usr/bin/env bash
# comments.py fails on long and multi-line comments.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
check() {
  git -C "$work" add -A
  python3 "$root/tests/scripts/comments.py" "$work" >/dev/null 2>&1
}
git init -q "$work"
git -C "$work" config core.autocrlf false
mkdir -p "$work/site/src" "$work/tests/scripts"
for ext in ts css; do
  printf '/* A comment\n   spanning lines. */\n' >"$work/site/src/a.$ext"
  ! check || fail "a multi-line .$ext block comment passed"
  printf '/* One two three four five six seven eight nine ten eleven. */\n' >"$work/site/src/a.$ext"
  ! check || fail "an eleven-word .$ext comment passed"
  printf '/* One short line. */\n' >"$work/site/src/a.$ext"
  check || fail "a short .$ext comment failed"
  rm "$work/site/src/a.$ext"
done
printf 'const a = 1 /* A comment\n   spanning lines. */\n' >"$work/site/src/a.ts"
! check || fail "an inline multi-line .ts block comment passed"
printf 'const glob = "/*.css";\n' >"$work/site/src/a.ts"
check || fail "a /* inside a .ts string failed"
printf 'const a = 1; // One two three four five six seven eight nine ten eleven.\n' >"$work/site/src/a.ts"
! check || fail "an eleven-word trailing .ts comment passed"
rm "$work/site/src/a.ts"
printf 'a::after { content: "/*"; }\n' >"$work/site/src/a.css"
check || fail "a /* inside a .css string failed"
printf 'a { color: red; } /* One two three four five six seven eight nine ten eleven. */\n' >"$work/site/src/a.css"
! check || fail "an eleven-word trailing .css comment passed"
rm "$work/site/src/a.css"
printf '// One two three four five six seven eight nine ten eleven.\n' >"$work/tests/scripts/a.swift"
! check || fail "an eleven-word Swift comment passed"
rm "$work/tests/scripts/a.swift"
printf 'CSS = """\n#a { color: red; }\n#b { color: blue; }\n"""\n' >"$work/tests/scripts/a.py"
check || fail "a # inside a Python string was read as a comment"
printf '# One two three four five six seven eight nine ten eleven.\nx = 1\n' >"$work/tests/scripts/a.py"
! check || fail "an eleven-word Python comment passed"
printf '# One line.\n# Another line.\n' >"$work/tests/scripts/a.py"
! check || fail "a two-line Python comment passed"
rm "$work/tests/scripts/a.py"
check || fail "a clean tree failed"
printf '/* One two three four five six seven eight nine ten eleven. */\n' >"$work/site/src/scratch.ts"
python3 "$root/tests/scripts/comments.py" "$work" >/dev/null 2>&1 || fail "an untracked file was checked"
echo "ok: comment rules"
