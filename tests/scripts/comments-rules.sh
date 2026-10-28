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
long='One two three four five six seven eight nine ten eleven.'
printf 'x = 1  # %s\n' "$long" >"$work/tests/scripts/a.py"
! check || fail "an eleven-word trailing Python comment passed"
printf 'x = 1  # one\ny = 2  # two\n' >"$work/tests/scripts/a.py"
check || fail "trailing Python comments on two lines failed"
printf 'def f():\n    """%s"""\n' "$long" >"$work/tests/scripts/a.py"
! check || fail "an eleven-word function docstring passed"
printf 'class A:\n    """One.\n\n    Two.\n    """\n' >"$work/tests/scripts/a.py"
! check || fail "a multi-line class docstring passed"
rm "$work/tests/scripts/a.py"
mkdir -p "$work/site/video/lib" "$work/site/video/vendor"
printf '/* A comment\n   spanning lines. */\n' >"$work/site/video/lib/a.js"
! check || fail "a multi-line .js block comment passed"
printf '// %s\n' "$long" >"$work/site/video/lib/a.js"
! check || fail "an eleven-word .js comment passed"
printf 'var a = 1; // one\nvar b = 2; // two\n' >"$work/site/video/lib/a.js"
check || fail "trailing .js comments on two lines failed"
printf '/* %s */\n' "$long" >"$work/site/video/vendor/a.js"
check || fail "a vendored .js file was checked"
rm -r "$work/site/video/vendor"
printf '<p>It is fine.</p>\n<!-- %s -->\n' "$long" >"$work/site/video/a.html"
! check || fail "an eleven-word HTML comment passed"
printf "<p>It's fine.</p>\n<script>\n// %s\n</script>\n" "$long" >"$work/site/video/a.html"
! check || fail "an eleven-word comment in an HTML script passed"
printf "<p>It's fine.</p>\n<style>\n/* Short. */\n</style>\n" >"$work/site/video/a.html"
check || fail "a short HTML style comment failed"
rm "$work/site/video/a.html"
cat >"$work/site/video/a.html" <<'EOF'
<script>const markup = "<!-- One two three four five six seven eight nine ten eleven. -->";</script>
<style>a::after { content: "<!-- One two three four five six seven eight nine ten eleven. -->"; }</style>
<p title="<!-- One two three four five six seven eight nine ten eleven. -->">Hi</p>
EOF
check || fail "a comment inside an HTML script, style or attribute string was read"
rm "$work/site/video/a.html"
printf '/* first */ const a = 1;\n/* second */ const b = 2;\n' >"$work/site/video/lib/a.js"
check || fail "leading .js comments on two code lines were joined"
printf '/* a */ a { color: red; }\n/* b */ b { color: blue; }\n' >"$work/site/video/lib/a.css"
check || fail "leading .css comments on two rules were joined"
rm "$work/site/video/lib/a.css"
cat >"$work/site/video/lib/a.js" <<'EOF'
const quote = /['"]/g;
// One two three four five six seven eight nine ten eleven.
EOF
! check || fail "an eleven-word .js comment after a regex passed"
rm "$work/site/video/lib/a.js"
cat >"$work/site/video/lib/a.tsx" <<'EOF'
const view = <div>It's fine.{/* One two three four five six seven eight nine ten eleven. */}</div>;
EOF
! check || fail "an eleven-word comment in a JSX expression passed"
cat >"$work/site/video/lib/a.tsx" <<'EOF'
const view = <div>// One two three four five six seven eight nine ten eleven.</div>;
EOF
check || fail "JSX text that looks like a comment was read"
rm "$work/site/video/lib/a.tsx"
check || fail "a clean tree failed"
printf '/* One two three four five six seven eight nine ten eleven. */\n' >"$work/site/src/scratch.ts"
python3 "$root/tests/scripts/comments.py" "$work" >/dev/null 2>&1 || fail "an untracked file was checked"
echo "ok: comment rules"
