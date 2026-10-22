#!/usr/bin/env bash
# Usage: review.sh <item-id> <round> [path...] — codex xhigh, fast review.
set -euo pipefail

[ "${HERDR_ENV:-}" = 1 ] || { echo "review.sh: not inside herdr" >&2; exit 2; }
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
Copies of the tree you build for probes use their own CARGO_TARGET_DIR, never ./target: crates bake their source path in. Delete those copies and their target dirs before you write the verdict; keep only small logs, in .work/ under the project root. Write nothing in the folder above the project.
On Windows, build and run nothing under %TEMP%: Application Control blocks programs there. Linux runs in WSL (wsl -d Ubuntu); its clone at ~/Documents/Github/ElyOffice holds this tree's changes.
Do not suggest enhancements. Do not reject for taste. Edit no file except the verdict.
Round 2 and later: read the previous verdict tasks/reviews/$id-r$((round - 1)).md, confirm each defect is fixed, then look for new defects in changed code.
Write the verdict to $verdict. Line one is PASS or FAIL. After that, one entry per defect: file:line, what is wrong, a concrete failing scenario, the direction of the fix.
When done, reply with the single word PASS or FAIL."

# The last review's pane stays open until the next starts.
last=tasks/reviews/.pane
if [ -f "$last" ]; then
  herdr pane close "$(cat "$last")" >/dev/null 2>&1 || true
fi
pane=$(herdr pane split --current --direction right --cwd "$PWD" --no-focus | jq -r '.result.pane.pane_id')
echo "$pane" >"$last"
started=
for _ in 1 2 3 4 5; do
  if err=$(herdr agent start "$name" --kind codex --pane "$pane" --timeout 120000 -- --yolo --model gpt-6.1-sol -c 'model_reasoning_effort="xhigh"' -c 'service_tier="fast"' -c check_for_update_on_startup=false 2>&1); then
    started=1
    break
  fi
  sleep 3
done
[ -n "$started" ] || { echo "review.sh: agent did not start in $pane: $err" >&2; exit 1; }
herdr agent prompt "$name" "$prompt" --wait --timeout 3000000 >/dev/null
for _ in $(seq 360); do
  [ -f "$verdict" ] && break
  sleep 5
done

[ -f "$verdict" ] || { echo "review.sh: no verdict at $verdict; pane $pane left open" >&2; exit 1; }
cat "$verdict"
