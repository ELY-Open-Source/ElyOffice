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
printf -- '- [ ] T1.%s probe\n' 1 2 3 4 5 >"$work/tree/tasks/TASKS.md"
touch "$work/tree/tasks/BUGS.md" "$work/tree/site/TASKS.md" "$work/tree/site/video/TASKS.md"
git init -q "$work/tree"
real_sleep=$(command -v sleep)
cat >"$work/bin/herdr" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
w=$HERDR_FAKE
case "$1 $2" in
  "pane neighbor")
    [ "$6" = w1:p1 ] || { echo "neighbor of $6" >&2; exit 1; }
    [ "$(cat "$w/near")" = 0 ] || { echo '{"error":{"code":"timeout","message":"no answer"}}' >&2; exit 1; }
    r=$(cut -d ' ' -f 1 "$w/rows" 2>/dev/null | head -n 1 || true)
    # As herdr 0.9.5: no neighbor, no field.
    if [ -n "$r" ]; then
      echo "{\"result\":{\"neighbor\":{\"direction\":\"right\",\"pane_id\":\"$6\",\"neighbor_pane_id\":\"$r\"}}}"
    else
      echo "{\"result\":{\"neighbor\":{\"direction\":\"right\",\"pane_id\":\"$6\"}}}"
    fi ;;
  "pane layout")
    [ "$4" = w1:p1 ] || { echo "layout of $4" >&2; exit 1; }
    # A slow answer opens the race between callers.
    [ ! -f "$w/slow" ] || sleep 1
    # The caller holds the left half; reviewers stack right.
    col=$(awk '{ printf ",{\"pane_id\":\"%s\",\"rect\":{\"x\":77,\"width\":77,\"height\":%s}}", $1, $2 }' "$w/rows" 2>/dev/null || true)
    echo "{\"result\":{\"layout\":{\"workspace_id\":\"w1\",\"panes\":[{\"pane_id\":\"w1:p1\",\"rect\":{\"x\":0,\"width\":77,\"height\":45}}$col]}}}" ;;
  "pane split")
    n=$(($(cat "$w/panes" 2>/dev/null || echo 1) + 1))
    echo "$n" >"$w/panes"
    echo "$4 $6" >>"$w/splits"
    echo "w1:p$n" >>"$w/live"
    # As herdr: a split down halves the pane.
    if [ "$6" = right ]; then
      echo "w1:p$n 45" >>"$w/rows"
    else
      h=$(awk -v p="$4" '$1 == p { print $2 }' "$w/rows")
      awk -v p="$4" -v h="$((h - h / 2))" '$1 == p { $2 = h } 1' "$w/rows" >"$w/rows.next"
      echo "w1:p$n $((h / 2))" >>"$w/rows.next"
      mv "$w/rows.next" "$w/rows"
    fi
    echo "{\"result\":{\"pane\":{\"pane_id\":\"w1:p$n\"}}}" ;;
  "tab create")
    [ "$(cat "$w/tab")" = 0 ] || { echo '{"error":{"code":"workspace_not_found","message":"no w1"}}' >&2; exit 1; }
    n=$(($(cat "$w/panes" 2>/dev/null || echo 1) + 1))
    echo "$n" >"$w/panes"
    echo "${*:3}" >>"$w/tabs"
    echo "w1:p$n" >>"$w/live"
    echo "{\"result\":{\"tab\":{\"tab_id\":\"w1:t$n\"},\"root_pane\":{\"pane_id\":\"w1:p$n\"}}}" ;;
  "pane get")
    g=$(cat "$w/get")
    [ "$g" != 2 ] || { echo '{"error":{"code":"pane_not_found","message":"x"}}'; echo 'transport failure'; exit 1; }
    [ "$g" = 0 ] || { echo '{"error":{"code":"timeout","message":"no answer"}}'; exit 1; }
    grep -qx "$3" "$w/live" 2>/dev/null || { echo "{\"error\":{\"code\":\"pane_not_found\",\"message\":\"pane $3 not found\"}}"; exit 1; } ;;
  "pane close")
    [ "$(cat "$w/close")" = 0 ] || { echo "close refused" >&2; exit 1; }
    echo "$3" >>"$w/closed"
    grep -vx "$3" "$w/live" >"$w/live.next" || true
    mv "$w/live.next" "$w/live"
    awk -v p="$3" '$1 != p' "$w/rows" >"$w/rows.next" 2>/dev/null || true
    mv "$w/rows.next" "$w/rows" ;;
  "agent start")
    # As herdr: errors are JSON on stderr.
    s=$(cat "$w/start")
    [ "$s" = 0 ] || { echo "{\"error\":{\"code\":\"$s\",\"message\":\"said $s\"}}" >&2; exit 1; } ;;
  "agent prompt")
    # Without --wait the review is still running.
    case " $* " in *" --wait "*) ;; *) exit 0 ;; esac
    case " $* " in *" --timeout "*) ;; *) exit 0 ;; esac
    printf '%s' "$4" >"$w/asked"
    [ "$(cat "$w/late")" != 0 ] || cp "$w/verdict" "$w/tree/tasks/reviews/$FAKE_VERDICT"
    p=$(cat "$w/prompt")
    [ "$p" = 0 ] || { echo "{\"error\":{\"code\":\"$p\",\"message\":\"said $p\"}}" >&2; exit 1; } ;;
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
  "agent read") printf '\n Do you want to proceed?\n ❯ 1. Yes\n\n' ;;
esac
EOF
# Polls cost no time; shorter waits are real.
printf '#!/usr/bin/env bash\n[ "$1" = 30 ] || exec "%s" "$1"\necho "$1" >>"$HERDR_FAKE/slept"\n' "$real_sleep" >"$work/bin/sleep"
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
  echo "${TAB:-0}" >"$work/tab"
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
case $(cat "$work/asked") in *"inside the project root, under .work/T1.1-r1/"*) ;; *) fail "the prompt allows work outside the tree" ;; esac
# A reviewer blocked five minutes is shown, then closed.
rm -f "$work/closed" "$work/tree/tasks/reviews/.panes/T1.1"
run T1.1 1 0 0 blocked 'PASS\n'
[ "$rc" = 1 ] || fail "a blocked reviewer passed"
case $said in *"stayed blocked 5 minutes at:"*"Do you want to proceed?"*"pane closed"*) ;; *) fail "blocked screen not shown: $said" ;; esac
[ "$(wc -l <"$work/slept")" = 10 ] || fail "blocked polled $(wc -l <"$work/slept") times, not 10"
[ "$(cat "$work/closed" 2>/dev/null)" = "w1:p$(cat "$work/panes")" ] || fail "a blocked reviewer was left running"
[ ! -f "$work/tree/tasks/reviews/.panes/T1.1" ] || fail "a closed blocked pane's id was kept"
# A short block is waited out.
LATE=3 run T1.1 1 0 0 'blocked working idle' 'PASS\n'
[ "$rc" = 0 ] && [ "$said" = PASS ] || fail "a reviewer blocked once: rc $rc, $said"
run T1.1 1 0 0 broken 'PASS\n'
[ "$rc" = 1 ] || fail "an unreadable status passed"
# Herdr's own error reaches the message.
run T1.1 1 agent_pane_busy 0 done 'PASS\n'
[ "$rc" = 1 ] || fail "a claude that never started passed"
case $said in *"did not start claude: agent_pane_busy: said agent_pane_busy"*) ;; *) fail "start error lost: $said" ;; esac
run T1.1 1 0 timeout done 'PASS\n'
[ "$rc" = 1 ] || fail "a prompt that timed out passed"
case $said in *"ran past 90 minutes: timeout: said timeout"*) ;; *) fail "timeout not named: $said" ;; esac
# A stalled prompt is not a timeout.
run T1.1 1 0 agent_prompt_stalled done 'PASS\n'
[ "$rc" = 1 ] || fail "a stalled prompt passed"
case $said in *"never began the prompt: agent_prompt_stalled: said agent_prompt_stalled"*) ;; *) fail "stall not named: $said" ;; esac
case $said in *"90 minutes"*) fail "a stall read as a timeout: $said" ;; esac
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
reset() { rm -f "$work/closed" "$work/panes" "$work/live" "$work/rows" "$work/splits" "$work/tabs" "$work/tree/tasks/reviews/.panes/"*; }
reset
run T1.1 1 0 0 done 'PASS
'
[ "$(cat "$work/splits")" = "w1:p1 right" ] || fail "first pane not right of the caller: $(cat "$work/splits")"
[ "$(cat "$work/closed")" = w1:p2 ] || fail "the finished pane stayed: $(cat "$work/closed" 2>/dev/null)"
[ ! -s "$work/live" ] || fail "panes left: $(cat "$work/live")"
[ ! -f "$work/tree/tasks/reviews/.panes/T1.1" ] || fail "a closed pane's id was kept"
# Failed rounds keep their pane; the next stacks below.
reset
run T1.2 1 0 0 unknown 'PASS
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
run T1.1 1 0 0 unknown 'PASS
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
# Halves keep ten terminal rows; chrome takes two of each.
reset
for t in 1 2 3; do run T1.$t 1 0 0 unknown 'PASS\n'; done
[ "$(cat "$work/splits")" = "$(printf 'w1:p1 right\nw1:p2 down')" ] || fail "column splits: $(cat "$work/splits")"
[ -z "$(awk '$2 - 2 < 10' "$work/rows")" ] || fail "a pane under ten terminal rows: $(cat "$work/rows")"
# A full column sends the next one to a tab.
[ "$(cat "$work/tabs" 2>/dev/null)" = "--workspace w1 --cwd $work/tree --label rv-t1-3-r1 --no-focus" ] || fail "no tab for a full column: $(cat "$work/tabs" 2>/dev/null)"
[ "$(cat "$work/tree/tasks/reviews/.panes/T1.3")" = w1:p4 ] || fail "the tab's pane was not kept"
run T1.3 2 0 0 done 'PASS\n'
[ "$rc" = 0 ] || fail "a review in a tab: rc $rc, $said"
[ "$(cat "$work/closed")" = "$(printf 'w1:p4\nw1:p5')" ] || fail "tab panes left open: $(cat "$work/closed")"
[ "$(wc -l <"$work/splits")" = 2 ] || fail "a full column split again: $(cat "$work/splits")"
# The tallest pane splits, from exactly 24 rows.
reset
printf 'w1:p2 23\nw1:p3 24\n' >"$work/rows"
printf 'w1:p2\nw1:p3\n' >"$work/live"
echo 3 >"$work/panes"
run T1.4 1 0 0 unknown 'PASS\n'
[ "$(cat "$work/splits")" = "w1:p3 down" ] || fail "not the tallest pane split: $(cat "$work/splits")"
run T1.5 1 0 0 unknown 'PASS\n'
[ "$(wc -l <"$work/splits")" = 1 ] || fail "23 rows split: $(cat "$work/splits")"
grep -q -- "--label rv-t1-5-r1 " "$work/tabs" || fail "23 rows did not send a tab: $(cat "$work/tabs" 2>/dev/null)"
# Callers at once place one at a time.
reset
echo "w1:p2 45" >"$work/rows"
echo w1:p2 >"$work/live"
echo 2 >"$work/panes"
touch "$work/slow"
echo unknown >"$work/status"
both() { HERDR_FAKE=$work HERDR_ENV=1 HERDR_PANE_ID=w1:p1 FAKE_VERDICT="$1-r1.md" PATH="$work/bin:$PATH" bash "$work/tree/scripts/review.sh" "$1" 1 >/dev/null 2>&1 || :; }
both T1.4 &
both T1.5 &
wait
rm "$work/slow"
[ "$(cat "$work/splits")" = "w1:p2 down" ] || fail "two callers split at once: $(cat "$work/splits")"
[ "$(wc -l <"$work/tabs")" = 1 ] || fail "the later caller made no tab: $(cat "$work/tabs" 2>/dev/null)"
[ -z "$(awk '$2 - 2 < 10' "$work/rows")" ] || fail "a pane under ten terminal rows: $(cat "$work/rows")"
[ ! -e "$work/tree/.git/review-place.lock" ] || fail "the placement lock was left"
# A lock nobody frees stops it and is named.
reset
mkdir "$work/tree/.git/review-place.lock"
run T1.1 1 0 0 done 'PASS\n'
rmdir "$work/tree/.git/review-place.lock"
[ "$rc" = 1 ] || fail "a held lock passed"
case $said in *"review-place.lock held for 30 s"*) ;; *) fail "held lock not named: $said" ;; esac
[ ! -f "$work/splits" ] || fail "split under a held lock: $(cat "$work/splits")"
# A tab herdr refuses is named.
echo "w1:p2 23" >"$work/rows"
TAB=1 run T1.1 1 0 0 done 'PASS\n'
[ "$rc" = 1 ] || fail "a refused tab passed"
case $said in *"no pane for rv-t1-1-r1"*workspace_not_found*) ;; *) fail "tab refusal not named: $said" ;; esac
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
