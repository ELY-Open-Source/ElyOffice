#!/usr/bin/env bash
# Usage: review.sh <item-id> <round> [path...] — Opus 5.5 high.
set -euo pipefail

[ "${HERDR_ENV:-}" = 1 ] && [ -n "${HERDR_PANE_ID:-}" ] || { echo "review.sh: not inside a herdr pane" >&2; exit 2; }
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

# A pane left by a failed round closes first.
mkdir -p tasks/reviews/.panes
last=tasks/reviews/.panes/$id
# Native Windows jq ends lines with CRLF.
field() { jq -r "$@" | tr -d '\r'; }
if [ -f "$last" ]; then
  old=$(cat "$last")
  if ! said=$(herdr pane get "$old" 2>&1); then
    # Only a pane herdr no longer knows is skipped.
    code=$(field '.error.code' <<<"$said" 2>/dev/null) && [ "$code" = pane_not_found ] \
      || { echo "review.sh: pane $old unreadable: $said" >&2; exit 1; }
  else
    err=$(herdr pane close "$old" 2>&1) || { echo "review.sh: pane $old did not close: $err" >&2; exit 1; }
  fi
  rm -f "$last"
fi
# Reviewers stack in one column right of the caller.
right=$(herdr pane neighbor --direction right --pane "$HERDR_PANE_ID" | field '.result.neighbor.neighbor_pane_id // empty') \
  || { echo "review.sh: no layout for $HERDR_PANE_ID" >&2; exit 1; }
if [ -z "$right" ]; then
  made=$(herdr pane split --pane "$HERDR_PANE_ID" --direction right --cwd "$PWD" --no-focus)
else
  made=$(herdr pane split --pane "$right" --direction down --cwd "$PWD" --no-focus)
fi
pane=$(field '.result.pane.pane_id' <<<"$made")
[ -n "$pane" ] && [ "$pane" != null ] || { echo "review.sh: no pane for $name: $made" >&2; exit 1; }
echo "$pane" >"$last"
fail() { echo "review.sh: $name in $pane $*; pane left open" >&2; exit 1; }
herdr agent start "$name" --kind claude --pane "$pane" --timeout 120000 -- \
  --model claude-opus-5-5 --effort high --dangerously-skip-permissions \
  >/dev/null 2>&1 || fail "did not start claude"
herdr agent prompt "$name" "$prompt" --wait --timeout 5400000 >/dev/null 2>&1 || fail "took no prompt or ran past 90 minutes"
# Claude idles while its own background runs work.
for ((i = 0; ; i++)); do
  s=$(herdr agent get "$name" | field '.result.agent.agent_status') || fail "status unreadable"
  case $s in
    done | idle) head -n 1 "$verdict" 2>/dev/null | tr -d '\r' | grep -qxE 'PASS|FAIL' && break ;;
    working) ;;
    *) fail "ended $s" ;;
  esac
  [ "$i" -lt 180 ] || fail "left no verdict at $verdict in 90 more minutes"
  sleep 30
done
err=$(herdr pane close "$pane" 2>&1) || fail "did not close: $err"
rm -f "$last"
cat "$verdict"
