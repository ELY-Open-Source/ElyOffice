#!/usr/bin/env bash
# Usage: review.sh <item-id> <round> [path...] — codex xhigh, fast review.
set -euo pipefail

[ -n "${HC_TERMINAL_ID:-}" ] || { echo "review.sh: not inside the herdr app" >&2; exit 2; }
id=${1:?item id}
round=${2:?round}
shift 2
scope=${*:-"the files this item touched; find them from the item line and the tree"}

cd "$(dirname "$0")/.."
name="rv-$(echo "$id" | tr 'A-Z.' 'a-z-')-r$round"
verdict="tasks/reviews/$id-r$round.md"
mkdir -p tasks/reviews
rm -f "$verdict"

# The list that holds the item.
list=$(grep -l -- "] $id " tasks/TASKS.md tasks/BUGS.md site/TASKS.md site/video/TASKS.md | head -n 1)
[ -n "$list" ] || { echo "review.sh: no item $id in any task list" >&2; exit 2; }
prompt="You review item $id of Ely Office, round $round. Work dir is the project root; agents.md holds the rules; docs/decisions.md is the dated decision log.
Read the line for $id in $list and the PRD.md sections it cites (for tasks/BUGS.md: its entry in docs/BugList.md); the items before it in that list are done.
Scope: $scope
Find defects only: behavior that breaks the PRD, errors swallowed, silent fallbacks, dead code, files over 500 lines, comments over one line or ten words, boxes inside boxes or useless subtitles in UI, abstractions with one user, missing or weak tests, claims no evidence backs. Build and run the tests yourself.
Copies of the tree you build for probes use their own CARGO_TARGET_DIR, never ./target: crates bake their source path in. Write the verdict first, then delete those copies and their target dirs; keep only small logs, in .work/ under the project root. If a deletion is refused, add a last line to the verdict naming what is left. Write nothing in the folder above the project.
On Windows, build and run nothing under %TEMP%: Application Control blocks programs there. Linux runs in WSL (wsl -d Ubuntu); its clone at ~/Documents/Github/ElyOffice holds this tree's changes.
Do not suggest enhancements. Do not reject for taste. Edit no file except the verdict.
Round 2 and later: read the previous verdict tasks/reviews/$id-r$((round - 1)).md, confirm each defect is fixed, then look for new defects in changed code.
Write the verdict to $verdict. Line one is PASS or FAIL. After that, one entry per defect: file:line, what is wrong, a concrete failing scenario, the direction of the fix.
When done, reply with the single word PASS or FAIL."

# The last review's terminal stays open until the next starts.
last=tasks/reviews/.pane
if [ -f "$last" ]; then
  hc kill "$(cat "$last")" >/dev/null 2>&1 || true
fi
# Native Windows jq ends lines with CRLF.
field() { jq -r "$@" | tr -d '\r'; }
self=$(hc get "$HC_TERMINAL_ID" --json)
label() { field --arg k "$1" '.labels[$k]' <<<"$self"; }
term=$(hc run --bg --json --name "$name" --cwd "$PWD" \
  --label "herdr.space=$(label herdr.space)" --label "herdr.tab=$(label herdr.tab)" --label "where=$(label where)" \
  codex --yolo --model gpt-6-astra -c 'model_reasoning_effort="xhigh"' -c 'service_tier="fast"' -c check_for_update_on_startup=false \
  | field '.address')
[ -n "$term" ] && [ "$term" != null ] || { echo "review.sh: codex did not start" >&2; exit 1; }
echo "$term" >"$last"
# Codex takes input once its screen settles.
sent=
for _ in $(seq 60); do
  if err=$(hc send "$term" "$prompt" --if idle 2>&1); then
    sent=1
    break
  fi
  sleep 2
done
[ -n "$sent" ] || { echo "review.sh: $term took no prompt: $err" >&2; exit 1; }
fail() { echo "review.sh: $term $*; terminal left open" >&2; exit 1; }
# A long paste can swallow the Enter.
started=
for _ in $(seq 10); do
  sleep 3
  s=$(hc get "$term" --json | field '.status.state') || fail "status unreadable"
  case $s in
    working | done) started=1; break ;;
    idle) hc keys "$term" enter >/dev/null ;;
    *) fail "is $s before the review began" ;;
  esac
done
[ -n "$started" ] || fail "did not start the review"
# A message to this terminal ends a wait early.
ended=
limit=$((SECONDS + 5400))
while [ "$SECONDS" -lt "$limit" ]; do
  r=$(hc wait "$term" --timeout 600 --json) || fail "wait failed"
  [ "$(field .ended <<<"$r")" = true ] && { ended=1; break; }
done
[ -n "$ended" ] || fail "still working after 90 minutes"
s=$(field '.status // empty' <<<"$r")
[ "$s" = done ] || [ "$s" = idle ] || fail "ended: $(jq -c '{reason, status, exit_code}' <<<"$r")"

head -n 1 "$verdict" 2>/dev/null | tr -d '\r' | grep -qxE 'PASS|FAIL' || fail "left no verdict at $verdict"
cat "$verdict"
