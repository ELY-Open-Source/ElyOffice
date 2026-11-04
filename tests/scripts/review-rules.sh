#!/usr/bin/env bash
# review.sh against a stand-in herdr: panes, stops, verdict.
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
  "pane neighbor")
    [ "$6" = w1:p1 ] || { echo "neighbor of $6" >&2; exit 1; }
    [ "$(cat "$w/near")" = 0 ] || { echo '{"error":{"code":"timeout","message":"no answer"}}' >&2; exit 1; }
    r=$(head -n 1 "$w/live" 2>/dev/null || true)
    # As herdr 0.9.5: no neighbor, no field.
    if [ -n "$r" ]; then
      echo "{\"result\":{\"neighbor\":{\"direction\":\"right\",\"pane_id\":\"$6\",\"neighbor_pane_id\":\"$r\"}}}"
    else
      echo "{\"result\":{\"neighbor\":{\"direction\":\"right\",\"pane_id\":\"$6\"}}}"
    fi ;;
  "pane split")
    n=$(($(cat "$w/panes" 2>/dev/null || echo 1) + 1))
    echo "$n" >"$w/panes"
    echo "$4 $6" >>"$w/splits"
    echo "w1:p$n" >>"$w/live"
    echo "{\"result\":{\"pane\":{\"pane_id\":\"w1:p$n\"}}}" ;;
  "pane get")
    g=$(cat "$w/get")
    [ "$g" != 2 ] || { echo '{"error":{"code":"pane_not_found","message":"x"}}'; echo 'transport failure'; exit 1; }
    [ "$g" = 0 ] || { echo '{"error":{"code":"timeout","message":"no answer"}}'; exit 1; }
    grep -qx "$3" "$w/live" 2>/dev/null || { echo "{\"error\":{\"code\":\"pane_not_found\",\"message\":\"pane $3 not found\"}}"; exit 1; } ;;
  "pane close")
    [ "$(cat "$w/close")" = 0 ] || { echo "close refused" >&2; exit 1; }
    echo "$3" >>"$w/closed"
    grep -vx "$3" "$w/live" >"$w/live.next" || true
    mv "$w/live.next" "$w/live" ;;
  "agent start") exit "$(cat "$w/start")" ;;
  "agent prompt")
    # Without --wait the review is still running.
    case " $* " in *" --wait "*) ;; *) exit 0 ;; esac
    case " $* " in *" --timeout "*) ;; *) exit 0 ;; esac
    [ "$(cat "$w/late")" != 0 ] || cp "$w/verdict" "$w/tree/tasks/reviews/$FAKE_VERDICT"
    exit "$(cat "$w/prompt")" ;;
  "agent get")
    # A late verdict lands after that many reads.
    n=$(($(cat "$w/gets") + 1))
    echo "$n" >"$w/gets"
    [ "$n" != "$(cat "$w/late")" ] || cp "$w/verdict" "$w/tree/tasks/reviews/$FAKE_VERDICT"
    # Read n takes word n; the last word holds.
    read -ra all <"$w/status"
    s=${all[$((n <= ${#all[@]} ? n - 1 : ${#all[@]} - 1))]}
    [ "$s" != broken ] || exit 1
    echo "{\"result\":{\"agent\":{\"agent_status\":\"$s\"}}}" ;;
esac
EOF
# Polls cost no time.
printf '#!/usr/bin/env bash\necho "$1" >>"$HERDR_FAKE/slept"\n' >"$work/bin/sleep"
# Native Windows jq writes CRLF.
cat >"$work/crlf/jq" <<EOF
#!/usr/bin/env bash
set -o pipefail
"$(command -v jq)" "\$@" | awk '{ printf "%s\r\n", \$0 }'
EOF
chmod +x "$work/bin/herdr" "$work/bin/sleep" "$work/crlf/jq"
# Usage: run item round start prompt status verdict [path].
run() {
  echo "${CLOSE:-0}" >"$work/close"
  echo "${GET:-0}" >"$work/get"
  echo "${NEAR:-0}" >"$work/near"
  echo "${LATE:-0}" >"$work/late"
  echo 0 >"$work/gets"
  rm -f "$work/slept"
  echo "$3" >"$work/start"
  echo "$4" >"$work/prompt"
  echo "$5" >"$work/status"
  printf '%b' "$6" >"$work/verdict"
  mkdir -p "$work/tree/tasks/reviews"
  rm -f "$work/tree/tasks/reviews/$1-r$2.md"
  rc=0
  said=$(HERDR_FAKE=$work HERDR_ENV=1 HERDR_PANE_ID=w1:p1 FAKE_VERDICT="$1-r$2.md" PATH="${7:-}$work/bin:$PATH" \
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
[ "$rc" = 1 ] || fail "a claude that never started passed"
run T1.1 1 0 1 done 'PASS\n'
[ "$rc" = 1 ] || fail "a prompt that timed out passed"
run T1.1 1 0 0 done ''
[ "$rc" = 1 ] || fail "an empty verdict passed"
case $said in *"90 more minutes"*) ;; *) fail "a missing verdict not named: $said" ;; esac
[ "$(wc -l <"$work/slept")" = 180 ] || fail "polled $(wc -l <"$work/slept") times, not 180"
# A reviewer idle on its own background run is awaited.
LATE=3 run T1.1 1 0 0 idle 'PASS
'
[ "$rc" = 0 ] && [ "$said" = PASS ] || fail "a late verdict: rc $rc, $said"
[ "$(cat "$work/slept")" = "$(printf '30\n30')" ] || fail "late verdict slept: $(cat "$work/slept")"
# It may work again before the verdict.
LATE=3 run T1.1 1 0 0 'idle working idle' 'PASS\n'
[ "$rc" = 0 ] && [ "$said" = PASS ] || fail "a resumed reviewer: rc $rc, $said"
# Each round's pane goes once its verdict is read.
reset() { rm -f "$work/closed" "$work/panes" "$work/live" "$work/splits" "$work/tree/tasks/reviews/.panes/"*; }
reset
run T1.1 1 0 0 done 'PASS
'
[ "$(cat "$work/splits")" = "w1:p1 right" ] || fail "first pane not right of the caller: $(cat "$work/splits")"
[ "$(cat "$work/closed")" = w1:p2 ] || fail "the finished pane stayed: $(cat "$work/closed" 2>/dev/null)"
[ ! -s "$work/live" ] || fail "panes left: $(cat "$work/live")"
[ ! -f "$work/tree/tasks/reviews/.panes/T1.1" ] || fail "a closed pane's id was kept"
# Failed rounds keep their pane; the next stacks below.
reset
run T1.2 1 0 0 blocked 'PASS
'
[ "$(cat "$work/tree/tasks/reviews/.panes/T1.2")" = w1:p2 ] || fail "the failed pane's id was lost"
run T1.1 1 0 0 done 'PASS
'
[ "$(sed -n 2p "$work/splits")" = "w1:p2 down" ] || fail "second pane not below the first: $(cat "$work/splits")"
[ "$(cat "$work/closed")" = w1:p3 ] || fail "another item's pane closed: $(cat "$work/closed")"
run T1.2 2 0 0 done 'PASS
'
[ "$(sed -n 2p "$work/closed")" = w1:p2 ] || fail "round 2 closed $(cat "$work/closed"), not its round 1 pane"
# A live pane that will not close stops the review.
reset
run T1.1 1 0 0 blocked 'PASS
'
CLOSE=1 run T1.1 2 0 0 done 'PASS
'
[ "$rc" = 1 ] || fail "a refused close passed: $said"
[ "$(cat "$work/tree/tasks/reviews/.panes/T1.1")" = w1:p2 ] || fail "the open pane's id was lost"
# A lookup that fails otherwise stops it too.
GET=1 run T1.1 2 0 0 done 'PASS
'
[ "$rc" = 1 ] || fail "a failed lookup passed: $said"
[ "$(cat "$work/tree/tasks/reviews/.panes/T1.1")" = w1:p2 ] || fail "a failed lookup lost the id"
# So does a lookup answer jq cannot parse.
GET=2 run T1.1 2 0 0 done 'PASS
'
[ "$rc" = 1 ] || fail "an unparsed lookup passed: $said"
[ "$(cat "$work/tree/tasks/reviews/.panes/T1.1")" = w1:p2 ] || fail "an unparsed lookup lost the id"
# A pane the user closed is no obstacle.
: >"$work/live"
run T1.1 2 0 0 done 'PASS
'
[ "$rc" = 0 ] || fail "a gone pane stopped the review: $said"
# A finished pane that will not close is named.
reset
CLOSE=1 run T1.1 1 0 0 done 'PASS
'
[ "$rc" = 1 ] || fail "a refused final close passed: $said"
[ "$(cat "$work/tree/tasks/reviews/.panes/T1.1")" = w1:p2 ] || fail "the unclosed pane's id was lost"
# A layout herdr cannot read stops it.
reset
NEAR=1 run T1.1 1 0 0 done 'PASS
'
[ "$rc" = 1 ] || fail "an unread layout passed: $said"
case $said in *"no layout"*) ;; *) fail "unread layout not named: $said" ;; esac
[ ! -f "$work/splits" ] || fail "split without a layout: $(cat "$work/splits")"
# Outside a herdr pane nothing runs.
said=$(HERDR_ENV=1 PATH="$work/bin:$PATH" bash "$work/tree/scripts/review.sh" T1.1 1 2>&1) && fail "ran without a pane"
echo "ok: review rules"
