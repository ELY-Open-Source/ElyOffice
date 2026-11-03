#!/usr/bin/env bash
# review.sh against a stand-in herdr: tabs, stops, verdict.
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
printf -- '- [ ] T1.1 probe\n- [ ] T1.2 probe\n' >"$work/tree/tasks/TASKS.md"
touch "$work/tree/tasks/BUGS.md" "$work/tree/site/TASKS.md" "$work/tree/site/video/TASKS.md"
cat >"$work/bin/herdr" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
w=$HERDR_FAKE
case "$1 $2" in
  "tab create")
    case " $* " in *" --workspace w1 "*) ;; *) echo "tab outside w1: $*" >&2; exit 1 ;; esac
    n=$(($(cat "$w/tabs" 2>/dev/null || echo 0) + 1))
    echo "$n" >"$w/tabs"
    echo "w1:t$n" >>"$w/live"
    echo "{\"result\":{\"tab\":{\"tab_id\":\"w1:t$n\"},\"root_pane\":{\"pane_id\":\"w1:p$n\"}}}" ;;
  "tab get")
    g=$(cat "$w/get")
    [ "$g" != 2 ] || { echo '{"error":{"code":"tab_not_found","message":"x"}}'; echo 'transport failure'; exit 1; }
    [ "$g" = 0 ] || { echo '{"error":{"code":"timeout","message":"no answer"}}'; exit 1; }
    grep -qx "$3" "$w/live" 2>/dev/null || { echo "{\"error\":{\"code\":\"tab_not_found\",\"message\":\"tab $3 not found\"}}"; exit 1; } ;;
  "tab close")
    [ "$(cat "$w/close")" = 0 ] || { echo "close refused" >&2; exit 1; }
    echo "$3" >>"$w/closed"
    grep -vx "$3" "$w/live" >"$w/live.next" || true
    mv "$w/live.next" "$w/live" ;;
  "agent start") exit "$(cat "$w/start")" ;;
  "agent prompt")
    # Without --wait the review is still running.
    case " $* " in *" --wait "*) ;; *) exit 0 ;; esac
    case " $* " in *" --timeout "*) ;; *) exit 0 ;; esac
    cp "$w/verdict" "$w/tree/tasks/reviews/$FAKE_VERDICT"
    exit "$(cat "$w/prompt")" ;;
  "agent get")
    s=$(cat "$w/status")
    [ "$s" != broken ] || exit 1
    echo "{\"result\":{\"agent\":{\"agent_status\":\"$s\"}}}" ;;
esac
EOF
# Native Windows jq writes CRLF.
cat >"$work/crlf/jq" <<EOF
#!/usr/bin/env bash
set -o pipefail
"$(command -v jq)" "\$@" | awk '{ printf "%s\r\n", \$0 }'
EOF
chmod +x "$work/bin/herdr" "$work/crlf/jq"
# Usage: run item round start prompt status verdict [path].
run() {
  echo "${CLOSE:-0}" >"$work/close"
  echo "${GET:-0}" >"$work/get"
  echo "$3" >"$work/start"
  echo "$4" >"$work/prompt"
  echo "$5" >"$work/status"
  printf '%b' "$6" >"$work/verdict"
  mkdir -p "$work/tree/tasks/reviews"
  rm -f "$work/tree/tasks/reviews/$1-r$2.md"
  rc=0
  said=$(HERDR_FAKE=$work HERDR_ENV=1 HERDR_WORKSPACE_ID=w1 FAKE_VERDICT="$1-r$2.md" PATH="${7:-}$work/bin:$PATH" \
    bash "$work/tree/scripts/review.sh" "$1" "$2" 2>&1) || rc=$?
}
run T1.1 1 0 0 done 'PASS\n'
[ "$rc" = 0 ] && [ "$said" = PASS ] || fail "plain review: rc $rc, $said"
run T1.1 1 0 0 idle 'PASS\n' "$work/crlf:"
[ "$rc" = 0 ] && [ "$said" = PASS ] || fail "CRLF jq: rc $rc, $said"
run T1.1 1 0 0 done 'FAIL\nx.rs:1 bug\n'
case $said in *"x.rs:1 bug"*) ;; *) fail "verdict body lost: $said" ;; esac
run T1.1 1 0 0 blocked 'PASS\n'
[ "$rc" = 1 ] || fail "a blocked reviewer passed"
case $said in *blocked*) ;; *) fail "blocked not named: $said" ;; esac
run T1.1 1 0 0 broken 'PASS\n'
[ "$rc" = 1 ] || fail "an unreadable status passed"
run T1.1 1 1 0 done 'PASS\n'
[ "$rc" = 1 ] || fail "a codex that never started passed"
run T1.1 1 0 1 done 'PASS\n'
[ "$rc" = 1 ] || fail "a prompt that timed out passed"
run T1.1 1 0 0 done ''
[ "$rc" = 1 ] || fail "an empty verdict passed"
# Parallel items keep each other's tabs.
rm -f "$work/closed" "$work/tabs" "$work/live" "$work/tree/tasks/reviews/.tabs/"*
run T1.1 1 0 0 done 'PASS\n'
run T1.2 1 0 0 done 'PASS\n'
[ ! -f "$work/closed" ] || fail "another item's tab closed: $(cat "$work/closed")"
run T1.1 2 0 0 done 'PASS\n'
[ "$(cat "$work/closed")" = w1:t1 ] || fail "round 2 closed $(cat "$work/closed" 2>/dev/null), not its round 1 tab"
# A live tab that will not close stops the review.
CLOSE=1 run T1.1 3 0 0 done 'PASS\n'
[ "$rc" = 1 ] || fail "a refused close passed: $said"
[ "$(cat "$work/tree/tasks/reviews/.tabs/T1.1")" = w1:t3 ] || fail "the open tab's id was lost"
# A lookup that fails otherwise stops it too.
GET=1 run T1.1 3 0 0 done 'PASS\n'
[ "$rc" = 1 ] || fail "a failed lookup passed: $said"
[ "$(cat "$work/tree/tasks/reviews/.tabs/T1.1")" = w1:t3 ] || fail "a failed lookup lost the id"
# So does a lookup answer jq cannot parse.
GET=2 run T1.1 3 0 0 done 'PASS\n'
[ "$rc" = 1 ] || fail "an unparsed lookup passed: $said"
[ "$(cat "$work/tree/tasks/reviews/.tabs/T1.1")" = w1:t3 ] || fail "an unparsed lookup lost the id"
# A tab the user closed is no obstacle.
: >"$work/live"
CLOSE=1 run T1.1 3 0 0 done 'PASS\n'
[ "$rc" = 0 ] || fail "a gone tab stopped the review: $said"
echo "ok: review rules"
