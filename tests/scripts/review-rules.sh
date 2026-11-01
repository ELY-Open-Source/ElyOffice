#!/usr/bin/env bash
# review.sh against a stand-in hc: start, Enter, stops, verdict.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
mkdir -p "$work/bin" "$work/crlf" "$work/tree/scripts" "$work/tree/tasks" "$work/tree/site/video"
cp "$root/scripts/review.sh" "$work/tree/scripts/"
echo "- [ ] T1.1 probe" >"$work/tree/tasks/TASKS.md"
touch "$work/tree/tasks/BUGS.md" "$work/tree/site/TASKS.md" "$work/tree/site/video/TASKS.md"
# Pops the first line; the last line repeats.
cat >"$work/bin/pop" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
f=$1
head -n 1 "$f"
if [ "$(wc -l <"$f")" -gt 1 ]; then
  tail -n +2 "$f" >"$f.next"
  mv "$f.next" "$f"
fi
EOF
cat >"$work/bin/hc" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
w=$HC_FAKE
case $1 in
  kill | send) ;;
  run) echo '{"address":"t9"}' ;;
  keys) echo "$3" >>"$w/keys" ;;
  get)
    if [ "$2" = "$HC_TERMINAL_ID" ]; then
      echo '{"labels":{"herdr.space":"s","herdr.tab":"t","where":"w"}}'
      exit
    fi
    s=$("$w/bin/pop" "$w/states")
    [ "$s" != broken ] || exit 1
    echo "{\"status\":{\"state\":\"$s\"}}" ;;
  wait)
    r=$("$w/bin/pop" "$w/waits")
    case $r in *'"ended":true'*) cp "$w/verdict" "$w/tree/tasks/reviews/T1.1-r1.md" ;; esac
    echo "$r" ;;
esac
EOF
# Native Windows jq writes CRLF.
cat >"$work/crlf/jq" <<EOF
#!/usr/bin/env bash
set -o pipefail
"$(command -v jq)" "\$@" | awk '{ printf "%s\r\n", \$0 }'
EOF
chmod +x "$work/bin/hc" "$work/bin/pop" "$work/crlf/jq"
done='{"ended":true,"reason":"status","status":"done","exit_code":null}'
late='{"ended":false,"reason":"timeout","status":null,"exit_code":null}'
note='{"ended":false,"reason":"message","status":null,"exit_code":null}'
# Usage: run <states> <waits> <verdict> [path]; sets rc, said, keys.
run() {
  printf '%s\n' $1 >"$work/states"
  printf '%s\n' $2 >"$work/waits"
  printf '%b' "$3" >"$work/verdict"
  rm -f "$work/keys" "$work/tree/tasks/reviews/T1.1-r1.md"
  rc=0
  said=$(HC_FAKE=$work HC_TERMINAL_ID=t1 PATH="${4:-}$work/bin:$PATH" bash "$work/tree/scripts/review.sh" T1.1 1 2>&1) || rc=$?
  keys=0
  if [ -f "$work/keys" ]; then keys=$(($(wc -l <"$work/keys"))); fi
}
run "working" "$late $done" 'PASS\n'
[ "$rc" = 0 ] && [ "$said" = PASS ] && [ "$keys" = 0 ] || fail "plain review: rc $rc, keys $keys, $said"
run "working" "$late $done" 'PASS\n' "$work/crlf:"
[ "$rc" = 0 ] && [ "$said" = PASS ] || fail "CRLF jq: rc $rc, $said"
run "working" "$note $note $note $note $note $note $note $note $note $note $done" 'PASS\n'
[ "$rc" = 0 ] && [ "$said" = PASS ] || fail "ten messages: rc $rc, $said"
run "idle working" "$done" 'FAIL\nx.rs:1 bug\n'
[ "$rc" = 0 ] && [ "$keys" = 1 ] || fail "swallowed Enter: rc $rc, keys $keys, $said"
case $said in *"x.rs:1 bug"*) ;; *) fail "verdict body lost: $said" ;; esac
run "blocked" "$done" 'PASS\n'
[ "$rc" = 1 ] && [ "$keys" = 0 ] || fail "blocked: rc $rc, keys $keys"
case $said in *blocked*) ;; *) fail "blocked not named: $said" ;; esac
run "broken" "$done" 'PASS\n'
[ "$rc" = 1 ] && [ "$keys" = 0 ] || fail "unreadable status: rc $rc, keys $keys"
run "done" "$done" 'PASS\r\n'
[ "$rc" = 0 ] || fail "a review done before the first look failed: $said"
run "working" '{"ended":true,"reason":"exit","status":null,"exit_code":17}' 'PASS\n'
[ "$rc" = 1 ] || fail "an exited reviewer passed"
case $said in *17*) ;; *) fail "exit code not named: $said" ;; esac
run "working" "$done" ''
[ "$rc" = 1 ] || fail "an empty verdict passed"
echo "ok: review rules"
