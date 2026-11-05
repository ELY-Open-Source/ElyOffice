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
Every build, probe copy, CARGO_TARGET_DIR and log stays inside the project root, under .work/$id-r$round/: never C:\\, never the folder above the project, never any other place outside it. Copies of the tree you build for probes use their own CARGO_TARGET_DIR there, never ./target: crates bake their source path in. Write the verdict first, then delete those copies and their target dirs; keep only small logs. If a deletion is refused, add a last line to the verdict naming what is left.
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
# Callers in every worktree place one at a time.
lock=$(git rev-parse --git-common-dir)/review-place.lock || { echo "review.sh: not in a git tree" >&2; exit 2; }
for ((t = 0; ; t++)); do
  mkdir "$lock" 2>/dev/null && break
  [ "$t" -lt 30 ] || { echo "review.sh: $lock held for 30 s; remove it if no review.sh is placing" >&2; exit 1; }
  sleep 1
done
trap 'rmdir "$lock"' EXIT
# Each reviewer keeps ten terminal rows; herdr's chrome takes two.
room=10 chrome=2
right=$(herdr pane neighbor --direction right --pane "$HERDR_PANE_ID" | field '.result.neighbor.neighbor_pane_id // empty') \
  && lay=$(herdr pane layout --pane "$HERDR_PANE_ID" | field -c '.result.layout') \
  || { echo "review.sh: no layout for $HERDR_PANE_ID" >&2; exit 1; }
if [ -z "$right" ]; then
  made=$(herdr pane split --pane "$HERDR_PANE_ID" --direction right --cwd "$PWD" --no-focus 2>&1) || :
else
  tall=$(field --arg r "$right" '.panes as $p | ($p[] | select(.pane_id == $r) | .rect.x) as $x
    | [$p[] | select(.rect.x == $x)] | max_by(.rect.height) | "\(.pane_id) \(.rect.height)"' <<<"$lay")
  [ -n "$tall" ] || { echo "review.sh: $right not in the layout: $lay" >&2; exit 1; }
  if [ "${tall#* }" -ge $((2 * (room + chrome))) ]; then
    made=$(herdr pane split --pane "${tall% *}" --direction down --cwd "$PWD" --no-focus 2>&1) || :
  else
    # A full column sends the reviewer to its own tab.
    made=$(herdr tab create --workspace "$(field .workspace_id <<<"$lay")" --cwd "$PWD" --label "$name" --no-focus 2>&1) || :
  fi
fi
pane=$(field '(.result.pane // .result.root_pane).pane_id' <<<"$made" 2>/dev/null) || :
[ -n "$pane" ] && [ "$pane" != null ] || { echo "review.sh: no pane for $name: $made" >&2; exit 1; }
echo "$pane" >"$last"
rmdir "$lock"
trap - EXIT
fail() { echo "review.sh: $name in $pane $*; pane left open" >&2; exit 1; }
# Herdr's error code and message, else its raw reply.
why() { field -e '.error | select(.code) | "\(.code): \(.message)"' <<<"$1" 2>/dev/null || echo "$1"; }
said=$(herdr agent start "$name" --kind claude --pane "$pane" --timeout 120000 -- \
  --model claude-opus-5-5 --effort high --dangerously-skip-permissions 2>&1) || fail "did not start claude: $(why "$said")"
if ! said=$(herdr agent prompt "$name" "$prompt" --wait --timeout 5400000 2>&1); then
  case $(field '.error.code' <<<"$said" 2>/dev/null) in
    agent_prompt_stalled) fail "never began the prompt: $(why "$said")" ;;
    timeout) fail "ran past 90 minutes: $(why "$said")" ;;
    *) fail "took no prompt: $(why "$said")" ;;
  esac
fi
# Blocked ten polls running: show the screen, then close.
blocked() {
  text=$(herdr agent read "$name" --source visible 2>&1) || text="unreadable: $text"
  text=$(grep -v '^[[:space:]]*$' <<<"$text" | tail -n 8 || :)
  err=$(herdr pane close "$pane" 2>&1) || fail "stayed blocked 5 minutes at: $text; did not close: $err"
  rm -f "$last"
  echo "review.sh: $name in $pane stayed blocked 5 minutes at: $text; pane closed" >&2
  exit 1
}
# Claude idles while its own background runs work.
b=0
for ((i = 0; ; i++)); do
  s=$(herdr agent get "$name" | field '.result.agent.agent_status') || fail "status unreadable"
  [ "$s" = blocked ] && b=$((b + 1)) || b=0
  case $s in
    done | idle) head -n 1 "$verdict" 2>/dev/null | tr -d '\r' | grep -qxE 'PASS|FAIL' && break ;;
    working) ;;
    blocked) [ "$b" -le 10 ] || blocked ;;
    *) fail "ended $s" ;;
  esac
  [ "$i" -lt 180 ] || fail "left no verdict at $verdict in 90 more minutes"
  sleep 30
done
err=$(herdr pane close "$pane" 2>&1) || fail "did not close: $err"
rm -f "$last"
cat "$verdict"
